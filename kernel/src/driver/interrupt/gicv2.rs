use core::ptr::{read_volatile, write_volatile};

const GICD_CTLR: usize = 0x000;
const GICD_TYPER: usize = 0x004;
const GICD_ISENABLER: usize = 0x100;
const GICD_ICENABLER: usize = 0x180;
const GICD_ICPENDR: usize = 0x280;
const GICD_ICACTIVER: usize = 0x380;
const GICD_IPRIORITYR: usize = 0x400;
const GICD_ITARGETSR: usize = 0x800;
const GICD_ICFGR: usize = 0xc00;

const FIRST_SPI_ID: usize = 32;
const ARCHITECTURAL_INTERRUPT_LIMIT: usize = 1020;

const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_BPR: usize = 0x008;
const GICC_IAR: usize = 0x00c;
const GICC_EOIR: usize = 0x010;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrqId(u16);

impl IrqId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct IrqAck(u32);

impl IrqAck {
    pub const fn id(self) -> IrqId {
        IrqId::new((self.0 & 0x3ff) as u16)
    }
}

/// The two trigger modes that GICv2 can route for a shared peripheral
/// interrupt. SPI polarity is fixed to active-high by the architecture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trigger {
    LevelHigh,
    EdgeRising,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiError {
    NotSpi,
    InterruptUnavailable,
    UnsupportedTargetCpu,
}

impl SpiError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotSpi => "interrupt ID is not an SPI",
            Self::InterruptUnavailable => "SPI is not implemented by the distributor",
            Self::UnsupportedTargetCpu => "only CPU 0 SPI routing is supported",
        }
    }
}

pub struct GicV2 {
    distributor: usize,
    cpu_interface: usize,
}

impl GicV2 {
    /// # Safety
    ///
    /// Both addresses must be valid, mapped GICv2 MMIO regions for the entire
    /// lifetime of the returned controller. This M1 driver assumes the QEMU
    /// `secure=off` programming model; it does not configure Group1NS.
    pub const unsafe fn new(distributor: usize, cpu_interface: usize) -> Self {
        Self {
            distributor,
            cpu_interface,
        }
    }

    pub fn init_boot_cpu(&self) -> usize {
        self.write_distributor(GICD_CTLR, 0);
        self.write_cpu(GICC_CTLR, 0);

        let interrupt_count = self.implemented_interrupt_count();
        let register_count = interrupt_count.div_ceil(32);

        // SGIs (0..15) remain enabled by architecture; reset all PPIs and SPIs.
        self.write_distributor(GICD_ICENABLER, 0xffff_0000);
        self.write_distributor(GICD_ICPENDR, 0xffff_0000);
        self.write_distributor(GICD_ICACTIVER, 0xffff_0000);
        for register in 1..register_count {
            self.write_distributor(GICD_ICENABLER + register * 4, u32::MAX);
            self.write_distributor(GICD_ICPENDR + register * 4, u32::MAX);
            self.write_distributor(GICD_ICACTIVER + register * 4, u32::MAX);
        }

        self.write_cpu(GICC_PMR, 0xff);
        self.write_cpu(GICC_BPR, 0);
        self.write_cpu(GICC_CTLR, 1);
        self.write_distributor(GICD_CTLR, 1);
        barrier();
        interrupt_count
    }

    pub fn enable(&self, irq: IrqId, priority: u8) {
        let id = irq.raw() as usize;
        if id >= ARCHITECTURAL_INTERRUPT_LIMIT {
            return;
        }

        unsafe {
            write_volatile(
                (self.distributor + GICD_IPRIORITYR + id) as *mut u8,
                priority,
            );
        }
        self.write_distributor(GICD_ICPENDR + (id / 32) * 4, 1 << (id % 32));
        self.write_distributor(GICD_ISENABLER + (id / 32) * 4, 1 << (id % 32));
        barrier();
    }

    /// Configures and enables one GICv2 shared peripheral interrupt.
    ///
    /// Bndroid is currently single-core, so the checked target CPU must be
    /// zero. The update deliberately disables the source before changing its
    /// trigger and routing fields, then drops any stale pending/active state
    /// before enabling it again.
    pub fn configure_enable_spi(
        &self,
        irq: IrqId,
        trigger: Trigger,
        cpu: u8,
        priority: u8,
    ) -> Result<(), SpiError> {
        let id = self.validate_spi(irq)?;
        if cpu != 0 {
            return Err(SpiError::UnsupportedTargetCpu);
        }

        let register_bit = 1 << (id % 32);
        self.write_distributor(GICD_ICENABLER + (id / 32) * 4, register_bit);

        let configuration_offset = GICD_ICFGR + (id / 16) * 4;
        let configuration = self.read_distributor(configuration_offset);
        self.write_distributor(
            configuration_offset,
            icfgr_with_trigger(configuration, id, trigger),
        );

        // GICD_ITARGETSR and GICD_IPRIORITYR are byte-addressable. A target
        // mask of bit 0 routes this SPI to the only supported CPU.
        self.write_distributor_byte(GICD_ITARGETSR + id, 1);
        self.write_distributor_byte(GICD_IPRIORITYR + id, priority);

        self.write_distributor(GICD_ICPENDR + (id / 32) * 4, register_bit);
        self.write_distributor(GICD_ICACTIVER + (id / 32) * 4, register_bit);
        self.write_distributor(GICD_ISENABLER + (id / 32) * 4, register_bit);
        barrier();
        Ok(())
    }

    /// Reconfigures and reenables an SPI without discarding an edge that may
    /// have arrived while the source was disabled.
    ///
    /// Recovery callers must first use [`Self::disable_and_clear_pending`]
    /// exactly once, while the old device epoch is still quiesced. From this
    /// point onward ICPENDR/ICACTIVER must not be written: an edge raised by
    /// the rebuilt device before or during this method is valid new-epoch
    /// evidence and must remain pending until the logical IRQ gate commits.
    pub fn reconfigure_enable_spi_preserving_pending(
        &self,
        irq: IrqId,
        trigger: Trigger,
        cpu: u8,
        priority: u8,
    ) -> Result<(), SpiError> {
        let id = self.validate_spi(irq)?;
        if cpu != 0 {
            return Err(SpiError::UnsupportedTargetCpu);
        }

        let register_bit = 1 << (id % 32);
        self.write_distributor(GICD_ICENABLER + (id / 32) * 4, register_bit);

        let configuration_offset = GICD_ICFGR + (id / 16) * 4;
        let configuration = self.read_distributor(configuration_offset);
        self.write_distributor(
            configuration_offset,
            icfgr_with_trigger(configuration, id, trigger),
        );
        self.write_distributor_byte(GICD_ITARGETSR + id, 1);
        self.write_distributor_byte(GICD_IPRIORITYR + id, priority);

        // Deliberately do not touch ICPENDR or ICACTIVER here. The matching
        // disable path already retired the old epoch, and clearing either bit
        // now would lose a legitimate edge from the rebuilt device.
        self.write_distributor(GICD_ISENABLER + (id / 32) * 4, register_bit);
        barrier();
        Ok(())
    }

    /// Prevents a peripheral from raising a new SPI and retires all pending or
    /// active delivery state from the old device epoch.
    #[allow(dead_code)]
    pub fn disable_and_clear_pending(&self, irq: IrqId) -> Result<(), SpiError> {
        let id = self.validate_spi(irq)?;
        let register_bit = 1 << (id % 32);
        self.write_distributor(GICD_ICENABLER + (id / 32) * 4, register_bit);
        self.write_distributor(GICD_ICPENDR + (id / 32) * 4, register_bit);
        self.write_distributor(GICD_ICACTIVER + (id / 32) * 4, register_bit);
        barrier();
        Ok(())
    }

    pub fn acknowledge(&self) -> Option<IrqAck> {
        let acknowledge = IrqAck(self.read_cpu(GICC_IAR));
        (usize::from(acknowledge.id().raw()) < ARCHITECTURAL_INTERRUPT_LIMIT).then_some(acknowledge)
    }

    pub fn end(&self, acknowledge: IrqAck) {
        self.write_cpu(GICC_EOIR, acknowledge.0);
        barrier();
    }

    fn validate_spi(&self, irq: IrqId) -> Result<usize, SpiError> {
        let id = irq.raw() as usize;
        if !(FIRST_SPI_ID..ARCHITECTURAL_INTERRUPT_LIMIT).contains(&id) {
            return Err(SpiError::NotSpi);
        }
        if id >= self.implemented_interrupt_count() {
            return Err(SpiError::InterruptUnavailable);
        }
        Ok(id)
    }

    fn implemented_interrupt_count(&self) -> usize {
        ((((self.read_distributor(GICD_TYPER) & 0x1f) + 1) * 32) as usize)
            .min(ARCHITECTURAL_INTERRUPT_LIMIT)
    }

    fn read_distributor(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.distributor + offset) as *const u32) }
    }

    fn write_distributor(&self, offset: usize, value: u32) {
        unsafe {
            write_volatile((self.distributor + offset) as *mut u32, value);
        }
    }

    fn write_distributor_byte(&self, offset: usize, value: u8) {
        unsafe {
            write_volatile((self.distributor + offset) as *mut u8, value);
        }
    }

    fn read_cpu(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.cpu_interface + offset) as *const u32) }
    }

    fn write_cpu(&self, offset: usize, value: u32) {
        unsafe {
            write_volatile((self.cpu_interface + offset) as *mut u32, value);
        }
    }
}

const fn icfgr_with_trigger(configuration: u32, id: usize, trigger: Trigger) -> u32 {
    let edge_bit = 1 << ((id % 16) * 2 + 1);
    match trigger {
        Trigger::LevelHigh => configuration & !edge_bit,
        Trigger::EdgeRising => configuration | edge_bit,
    }
}

fn barrier() {
    unsafe {
        core::arch::asm!("dsb sy", "isb", options(nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use super::{Trigger, icfgr_with_trigger};

    #[test]
    fn edge_trigger_sets_only_the_selected_edge_bit() {
        let original = 0x5a5a_a525;
        let updated = icfgr_with_trigger(original, 35, Trigger::EdgeRising);
        let selected_edge_bit = 1 << 7;
        assert_eq!(updated, original | selected_edge_bit);
        assert_eq!(updated & !selected_edge_bit, original & !selected_edge_bit);
    }

    #[test]
    fn level_trigger_clears_only_the_selected_edge_bit() {
        let original = u32::MAX;
        let updated = icfgr_with_trigger(original, 47, Trigger::LevelHigh);
        let selected_edge_bit = 1 << 31;
        assert_eq!(updated, original & !selected_edge_bit);
        assert_eq!(updated & !selected_edge_bit, original & !selected_edge_bit);
    }

    #[test]
    fn trigger_encoding_uses_the_interrupt_index_within_its_register() {
        assert_eq!(
            icfgr_with_trigger(0, 32, Trigger::EdgeRising),
            icfgr_with_trigger(0, 48, Trigger::EdgeRising)
        );
        assert_eq!(icfgr_with_trigger(0, 32, Trigger::EdgeRising), 1 << 1);
    }
}
