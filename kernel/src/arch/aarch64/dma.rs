//! AArch64 ordering primitives for coherent DMA and device MMIO.
//!
//! These barriers do not perform cache maintenance. Callers must reject
//! non-coherent devices until the kernel has a cache clean/invalidate API.

use core::arch::asm;

/// Publishes Normal-memory descriptor and payload writes before a device is
/// notified through an Outer Shareable MMIO mapping.
#[inline]
pub fn publish_to_device() {
    unsafe {
        asm!("dmb oshst", options(nostack, preserves_flags));
    }
}

/// Orders device writes observed through a coherent used ring before payload
/// or status bytes are consumed by the CPU.
#[inline]
pub fn acquire_from_device() {
    unsafe {
        asm!("dmb oshld", options(nostack, preserves_flags));
    }
}

/// Ensures a reset or queue-notify MMIO write has reached the peripheral
/// boundary before ownership or control state changes locally.
#[inline]
pub fn complete_mmio_write() {
    unsafe {
        asm!("dsb oshst", options(nostack, preserves_flags));
    }
}
