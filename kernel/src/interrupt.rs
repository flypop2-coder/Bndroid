#[cfg(feature = "storage-server-repeated-recovery-runtime")]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::arch::aarch64::{self, exceptions::TrapFrame, timer};
use crate::driver::interrupt::gicv2::{GicV2, IrqId, SpiError, Trigger};
use crate::platform::qemu_virt::{GICC_BASE, GICD_BASE, PHYSICAL_TIMER_IRQ};
use bndroid_kernel::virtio_input::InputDeviceKind;

static GIC: GicV2 = unsafe { GicV2::new(GICD_BASE, GICC_BASE) };
static BLOCK_IRQ_CONFIG: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "storage-server-repeated-recovery-runtime")]
static ABORT_NEXT_BLOCK_IRQ_REARM_COMMIT: AtomicBool = AtomicBool::new(false);

const BLOCK_IRQ_CONFIG_VALID: u32 = 1 << 31;
const BLOCK_IRQ_CONFIG_EDGE: u32 = 1 << 16;

pub struct InterruptInfo {
    pub interrupt_count: usize,
    pub timer_frequency_hz: u64,
    pub block_irq: u16,
    pub keyboard_irq: Option<u16>,
    pub pointer_irq: Option<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockIrqConfig {
    pub id: u16,
    pub trigger: Trigger,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputIrqConfig {
    pub kind: InputDeviceKind,
    pub id: u16,
    pub trigger: Trigger,
}

#[derive(Clone, Copy)]
pub enum InterruptError {
    Timer(timer::TimerError),
    Spi(SpiError),
    Storage(crate::storage::StorageError),
    Input(crate::input::RuntimeError),
    DuplicatePeripheralIrq,
}

impl InterruptError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timer(error) => error.as_str(),
            Self::Spi(error) => error.as_str(),
            Self::Storage(error) => error.as_str(),
            Self::Input(error) => error.as_str(),
            Self::DuplicatePeripheralIrq => "peripheral devices share one IRQ or input role",
        }
    }
}

pub fn init(
    ticks_per_second: u32,
    block: BlockIrqConfig,
    inputs: [Option<InputIrqConfig>; 2],
) -> Result<InterruptInfo, InterruptError> {
    if BLOCK_IRQ_CONFIG.load(Ordering::Acquire) != 0 {
        return Err(InterruptError::DuplicatePeripheralIrq);
    }
    let interrupt_count = GIC.init_boot_cpu();
    GIC.enable(PHYSICAL_TIMER_IRQ, 0x80);
    let block_irq = IrqId::new(block.id);
    GIC.configure_enable_spi(block_irq, block.trigger, 0, 0x90)
        .map_err(InterruptError::Spi)?;
    crate::storage::arm_irq(block.id).map_err(InterruptError::Storage)?;
    BLOCK_IRQ_CONFIG.store(encode_block_irq_config(block), Ordering::Release);
    let mut keyboard_irq = None;
    let mut pointer_irq = None;
    for input in inputs.into_iter().flatten() {
        if input.id == block.id || keyboard_irq == Some(input.id) || pointer_irq == Some(input.id) {
            return Err(InterruptError::DuplicatePeripheralIrq);
        }
        let (slot, priority) = match input.kind {
            InputDeviceKind::Keyboard => (&mut keyboard_irq, 0xa0),
            InputDeviceKind::AbsolutePointer => (&mut pointer_irq, 0xb0),
        };
        if slot.is_some() {
            return Err(InterruptError::DuplicatePeripheralIrq);
        }
        GIC.configure_enable_spi(IrqId::new(input.id), input.trigger, 0, priority)
            .map_err(InterruptError::Spi)?;
        crate::input::arm_irq(input.kind, input.id).map_err(InterruptError::Input)?;
        *slot = Some(input.id);
    }
    let timer_frequency_hz =
        timer::start_periodic(ticks_per_second).map_err(InterruptError::Timer)?;
    aarch64::enable_irq();
    Ok(InterruptInfo {
        interrupt_count,
        timer_frequency_hz,
        block_irq: block.id,
        keyboard_irq,
        pointer_irq,
    })
}

#[cfg_attr(not(feature = "app-data-runtime"), allow(dead_code))]
pub fn registered_block_irq() -> Option<BlockIrqConfig> {
    decode_block_irq_config(BLOCK_IRQ_CONFIG.load(Ordering::Acquire))
}

#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
pub fn disable_block_irq(block: BlockIrqConfig) -> Result<(), InterruptError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let result = disable_block_irq_masked(block);
    aarch64::restore_daif(saved_daif);
    result
}

#[cfg_attr(not(feature = "storage-server-runtime"), allow(dead_code))]
pub(crate) fn disable_block_irq_masked(block: BlockIrqConfig) -> Result<(), InterruptError> {
    if !aarch64::irq_is_masked() {
        return Err(InterruptError::Storage(
            crate::storage::StorageError::IrqMustBeMasked,
        ));
    }
    let gic_result = GIC
        .disable_and_clear_pending(IrqId::new(block.id))
        .map_err(InterruptError::Spi);
    let storage_result = crate::storage::disarm_irq(block.id).map_err(InterruptError::Storage);
    gic_result.and(storage_result)
}

#[cfg_attr(not(bndroid_storage_irq_timeout_profile), allow(dead_code))]
pub fn reenable_block_irq(block: BlockIrqConfig) -> Result<(), InterruptError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let result = reenable_block_irq_masked(block);
    aarch64::restore_daif(saved_daif);
    result
}

#[cfg_attr(not(feature = "storage-server-runtime"), allow(dead_code))]
pub(crate) fn reenable_block_irq_masked(block: BlockIrqConfig) -> Result<(), InterruptError> {
    if !aarch64::irq_is_masked() {
        return Err(InterruptError::Storage(
            crate::storage::StorageError::IrqMustBeMasked,
        ));
    }
    let result = match crate::storage::prepare_irq_rearm(block.id).map_err(InterruptError::Storage)
    {
        Ok(()) => match GIC
            .reconfigure_enable_spi_preserving_pending(IrqId::new(block.id), block.trigger, 0, 0x90)
            .map_err(InterruptError::Spi)
        {
            Ok(()) => crate::storage::validate_irq_rearm_tail(block.id)
                .and_then(|()| {
                    #[cfg(feature = "storage-server-repeated-recovery-runtime")]
                    if ABORT_NEXT_BLOCK_IRQ_REARM_COMMIT.swap(false, Ordering::AcqRel) {
                        return Err(crate::storage::StorageError::RecoveryRequired);
                    }
                    crate::storage::commit_irq_rearm(block.id)
                })
                .map_err(InterruptError::Storage),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    if result.is_err() {
        // GIC disable retires any failed new-epoch delivery before logical
        // rollback. The successful path never clears pending after enable, so
        // an edge after the tail audit remains deliverable after DAIF restore.
        if let Err(error) = rollback_block_irq_rearm_masked(block) {
            panic!(
                "block IRQ rearm failed and its fail-closed rollback failed: {}",
                error.as_str()
            );
        }
    }
    result
}

/// Disables the physical SPI and relatches the logical recovery barrier as
/// one checked cleanup step. Both operations run even if the GIC reports an
/// error; continuing with an unverified physical route would be unsafe.
#[cfg_attr(not(feature = "storage-server-runtime"), allow(dead_code))]
pub(crate) fn rollback_block_irq_rearm_masked(block: BlockIrqConfig) -> Result<(), InterruptError> {
    if !aarch64::irq_is_masked() {
        return Err(InterruptError::Storage(
            crate::storage::StorageError::IrqMustBeMasked,
        ));
    }
    let gic_result = GIC
        .disable_and_clear_pending(IrqId::new(block.id))
        .map_err(InterruptError::Spi);
    let logical_result =
        crate::storage::rollback_irq_rearm(block.id).map_err(InterruptError::Storage);
    gic_result.and(logical_result)
}

#[cfg(feature = "storage-server-repeated-recovery-runtime")]
pub(crate) fn abort_next_block_irq_rearm_commit_for_test() -> bool {
    ABORT_NEXT_BLOCK_IRQ_REARM_COMMIT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

#[cfg(feature = "storage-server-repeated-recovery-runtime")]
pub(crate) fn block_irq_rearm_commit_abort_armed() -> bool {
    ABORT_NEXT_BLOCK_IRQ_REARM_COMMIT.load(Ordering::Acquire)
}

const fn encode_block_irq_config(block: BlockIrqConfig) -> u32 {
    BLOCK_IRQ_CONFIG_VALID
        | block.id as u32
        | match block.trigger {
            Trigger::LevelHigh => 0,
            Trigger::EdgeRising => BLOCK_IRQ_CONFIG_EDGE,
        }
}

const fn decode_block_irq_config(encoded: u32) -> Option<BlockIrqConfig> {
    if encoded & BLOCK_IRQ_CONFIG_VALID == 0 {
        return None;
    }
    Some(BlockIrqConfig {
        id: (encoded & u16::MAX as u32) as u16,
        trigger: if encoded & BLOCK_IRQ_CONFIG_EDGE == 0 {
            Trigger::LevelHigh
        } else {
            Trigger::EdgeRising
        },
    })
}

const _: () = {
    let level = BlockIrqConfig {
        id: 79,
        trigger: Trigger::LevelHigh,
    };
    let edge = BlockIrqConfig {
        id: 80,
        trigger: Trigger::EdgeRising,
    };
    assert!(matches!(
        decode_block_irq_config(encode_block_irq_config(level)),
        Some(BlockIrqConfig {
            id: 79,
            trigger: Trigger::LevelHigh
        })
    ));
    assert!(matches!(
        decode_block_irq_config(encode_block_irq_config(edge)),
        Some(BlockIrqConfig {
            id: 80,
            trigger: Trigger::EdgeRising
        })
    ));
};

#[unsafe(no_mangle)]
extern "C" fn irq_dispatch(frame: *mut TrapFrame) -> *mut TrapFrame {
    let Some(acknowledge) = GIC.acknowledge() else {
        return frame;
    };

    let acknowledged_id = acknowledge.id();
    let is_timer_tick = acknowledged_id == PHYSICAL_TIMER_IRQ;
    crate::storage::handle_irq(acknowledged_id.raw());
    let input_handled = crate::input::handle_irq(acknowledged_id.raw());
    if is_timer_tick {
        timer::handle_interrupt();
    }
    // The device-specific source is ACKed and its used ring drained above;
    // only then may the GIC deactivate this interrupt.
    GIC.end(acknowledge);

    // A keyboard SYN_REPORT may have raised the bound Surface's KEY_READY
    // level while the virtio-input queue was reaped above. Wake authenticated
    // object waiters only after the device and GIC acknowledgements are
    // complete. Pointer reports are rasterized by the foreground monitor, so
    // an early scan for those IRQs is intentionally harmless.
    if input_handled {
        crate::scheduler::wake_object_waiters();
    }

    if is_timer_tick {
        #[cfg(all(
            feature = "graphics-frame-clock-runtime",
            not(feature = "graphics-owner-death-runtime"),
            not(feature = "app-crash-recovery-runtime")
        ))]
        crate::display::on_timer_tick(timer::logical_tick());
        crate::scheduler::on_timer_tick(frame)
    } else {
        frame
    }
}
