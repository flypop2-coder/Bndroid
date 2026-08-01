//! Permanent dual-device ownership and IRQ serialization for virtio-input.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU64, Ordering},
};

use bndroid_kernel::virtio_input::{InputDeviceKind, InputEvent};

use crate::{
    arch::aarch64,
    driver::virtio::input::{
        InputDeviceInfo, InputError, InputInterruptService, InputStats, VirtioInput,
    },
};

const EMPTY: u8 = 0;
const INSTALLING: u8 = 1;
const READY: u8 = 2;

struct PermanentInputDevice {
    state: AtomicU8,
    irq_id: AtomicU16,
    value: UnsafeCell<MaybeUninit<VirtioInput>>,
    irq_armed: AtomicBool,
    irq_failed: AtomicBool,
    irq_entries: AtomicU64,
    irq_queue_events: AtomicU64,
    irq_config_events: AtomicU64,
    irq_completions: AtomicU64,
    irq_spurious: AtomicU64,
}

impl PermanentInputDevice {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            irq_id: AtomicU16::new(0),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            irq_armed: AtomicBool::new(false),
            irq_failed: AtomicBool::new(false),
            irq_entries: AtomicU64::new(0),
            irq_queue_events: AtomicU64::new(0),
            irq_config_events: AtomicU64::new(0),
            irq_completions: AtomicU64::new(0),
            irq_spurious: AtomicU64::new(0),
        }
    }

    fn install(&self, device: VirtioInput, irq_id: u16) -> Result<(), InstallError> {
        if !(32..1020).contains(&irq_id) {
            return Err(InstallError::InvalidIrq);
        }
        self.state
            .compare_exchange(EMPTY, INSTALLING, Ordering::Acquire, Ordering::Acquire)
            .map_err(|_| InstallError::AlreadyInstalled)?;
        unsafe {
            (*self.value.get()).write(device);
        }
        self.irq_id.store(irq_id, Ordering::Relaxed);
        self.state.store(READY, Ordering::Release);
        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.state.load(Ordering::Acquire) == READY
    }

    fn irq_id(&self) -> Option<u16> {
        self.is_ready().then(|| self.irq_id.load(Ordering::Relaxed))
    }

    fn arm_irq(&self, irq_id: u16) -> Result<(), RuntimeError> {
        if self.irq_id() != Some(irq_id) {
            return Err(RuntimeError::IrqMismatch);
        }
        self.irq_failed.store(false, Ordering::Relaxed);
        self.irq_armed.store(true, Ordering::Release);
        Ok(())
    }

    fn handles_irq(&self, irq_id: u16) -> bool {
        self.irq_armed.load(Ordering::Acquire) && self.irq_id() == Some(irq_id)
    }

    fn handle_irq(&self, irq_id: u16) -> bool {
        if !self.handles_irq(irq_id) {
            return false;
        }
        self.irq_entries.fetch_add(1, Ordering::Relaxed);
        let service = self.with_device_masked(VirtioInput::service_interrupt);
        match service {
            Ok(Ok(InputInterruptService { bits, completions })) => {
                if bits == 0 {
                    self.irq_spurious.fetch_add(1, Ordering::Relaxed);
                }
                if bits & 1 != 0 {
                    self.irq_queue_events.fetch_add(1, Ordering::Relaxed);
                }
                if bits & 2 != 0 {
                    self.irq_config_events.fetch_add(1, Ordering::Relaxed);
                    self.irq_failed.store(true, Ordering::Release);
                }
                self.irq_completions
                    .fetch_add(u64::from(completions), Ordering::Relaxed);
            }
            Ok(Err(_)) | Err(_) => self.irq_failed.store(true, Ordering::Release),
        }
        true
    }

    fn with_device_masked<T>(
        &self,
        operation: impl FnOnce(&mut VirtioInput) -> T,
    ) -> Result<T, RuntimeError> {
        if !aarch64::irq_is_masked() {
            return Err(RuntimeError::IrqMustBeMasked);
        }
        if !self.is_ready() {
            return Err(RuntimeError::NotReady);
        }
        let device = unsafe { &mut *(*self.value.get()).as_mut_ptr() };
        Ok(operation(device))
    }

    fn irq_snapshot(&self) -> IrqSnapshot {
        IrqSnapshot {
            entries: self.irq_entries.load(Ordering::Relaxed),
            queue_events: self.irq_queue_events.load(Ordering::Relaxed),
            config_events: self.irq_config_events.load(Ordering::Relaxed),
            completions: self.irq_completions.load(Ordering::Relaxed),
            spurious: self.irq_spurious.load(Ordering::Relaxed),
            failed: self.irq_failed.load(Ordering::Acquire),
        }
    }
}

unsafe impl Sync for PermanentInputDevice {}

static KEYBOARD_DEVICE: PermanentInputDevice = PermanentInputDevice::new();
static POINTER_DEVICE: PermanentInputDevice = PermanentInputDevice::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallError {
    AlreadyInstalled,
    InvalidIrq,
    KindMismatch,
}

impl InstallError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyInstalled => "permanent input device role is already installed",
            Self::InvalidIrq => "input device IRQ is not a valid SPI",
            Self::KindMismatch => "input device capabilities do not match the permanent role",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    NotReady,
    IrqMustBeMasked,
    IrqMismatch,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    InterruptFailure,
    #[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
    Driver(InputError),
}

impl RuntimeError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotReady => "permanent input device is not ready",
            Self::IrqMustBeMasked => "input driver access requires local IRQ masking",
            Self::IrqMismatch => "input IRQ registration does not match the FDT",
            Self::InterruptFailure => "input hard-IRQ processing failed",
            Self::Driver(error) => error.as_str(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IrqSnapshot {
    pub entries: u64,
    pub queue_events: u64,
    pub config_events: u64,
    pub completions: u64,
    pub spurious: u64,
    pub failed: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSnapshot {
    pub stats: InputStats,
    pub irq: IrqSnapshot,
}

pub fn install(
    kind: InputDeviceKind,
    device: VirtioInput,
    irq_id: u16,
) -> Result<(), InstallError> {
    if device.info().kind() != kind {
        return Err(InstallError::KindMismatch);
    }
    endpoint(kind).install(device, irq_id)
}

pub fn is_ready(kind: InputDeviceKind) -> bool {
    endpoint(kind).is_ready()
}

pub fn irq_id(kind: InputDeviceKind) -> Option<u16> {
    endpoint(kind).irq_id()
}

pub fn arm_irq(kind: InputDeviceKind, irq_id: u16) -> Result<(), RuntimeError> {
    endpoint(kind).arm_irq(irq_id)
}

pub fn handle_irq(irq_id: u16) -> bool {
    KEYBOARD_DEVICE.handle_irq(irq_id) || POINTER_DEVICE.handle_irq(irq_id)
}

#[cfg_attr(feature = "el0-fault-containment-self-test", allow(dead_code))]
pub fn take_event(kind: InputDeviceKind) -> Result<Option<InputEvent>, RuntimeError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let endpoint = endpoint(kind);
    let result = if endpoint.irq_failed.load(Ordering::Acquire) {
        Err(RuntimeError::InterruptFailure)
    } else {
        match endpoint.with_device_masked(VirtioInput::take_event) {
            Ok(result) => result.map_err(RuntimeError::Driver),
            Err(error) => Err(error),
        }
    };
    aarch64::restore_daif(saved_daif);
    result
}

pub fn device_info(kind: InputDeviceKind) -> Result<InputDeviceInfo, RuntimeError> {
    with_foreground_device(kind, |device| *device.info())
}

pub fn runtime_snapshot(kind: InputDeviceKind) -> Result<RuntimeSnapshot, RuntimeError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let endpoint = endpoint(kind);
    let result = endpoint.with_device_masked(|device| RuntimeSnapshot {
        stats: device.stats(),
        irq: endpoint.irq_snapshot(),
    });
    aarch64::restore_daif(saved_daif);
    result
}

fn endpoint(kind: InputDeviceKind) -> &'static PermanentInputDevice {
    match kind {
        InputDeviceKind::Keyboard => &KEYBOARD_DEVICE,
        InputDeviceKind::AbsolutePointer => &POINTER_DEVICE,
    }
}

fn with_foreground_device<T>(
    kind: InputDeviceKind,
    operation: impl FnOnce(&mut VirtioInput) -> T,
) -> Result<T, RuntimeError> {
    let saved_daif = aarch64::save_and_mask_irq();
    let result = endpoint(kind).with_device_masked(operation);
    aarch64::restore_daif(saved_daif);
    result
}
