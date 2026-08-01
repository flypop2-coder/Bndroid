use crate::driver::interrupt::gicv2::IrqId;

pub const GICD_BASE: usize = 0x0800_0000;
pub const GICC_BASE: usize = 0x0801_0000;
pub const PHYSICAL_TIMER_IRQ: IrqId = IrqId::new(30);
#[cfg(feature = "mobile-ui-runtime")]
pub const PL031_RTC_BASE: usize = 0x0901_0000;
#[cfg(feature = "mobile-ui-runtime")]
pub const PL031_DATA_REGISTER_OFFSET: usize = 0;

/// Reads QEMU `virt`'s PL031 data register as an unsigned Unix timestamp.
///
/// The fixed address is part of the QEMU `virt` platform contract and lies in
/// the kernel's identity-mapped Device region. This is deliberately a
/// read-only preview-platform primitive; no RTC control or match register is
/// exposed to userspace.
#[cfg(feature = "mobile-ui-runtime")]
pub fn read_pl031_unix_seconds() -> u64 {
    let register = (PL031_RTC_BASE + PL031_DATA_REGISTER_OFFSET) as *const u32;
    // SAFETY: the mobile preview runs only on QEMU `virt`, whose PL031 data
    // register is a readable 32-bit MMIO register at this identity-mapped
    // Device address. Volatile access preserves the required MMIO read.
    u64::from(unsafe { core::ptr::read_volatile(register) })
}
