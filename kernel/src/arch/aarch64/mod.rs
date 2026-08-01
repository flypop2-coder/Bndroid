use core::arch::{asm, global_asm};

pub mod dma;
pub mod exceptions;
pub mod mmu;
pub mod timer;
pub mod usercopy;

#[cfg(feature = "app-data-runtime")]
const MONITOR_STACK_BYTES: usize = 256 * 1024;
#[cfg(not(feature = "app-data-runtime"))]
const MONITOR_STACK_BYTES: usize = 128 * 1024;

global_asm!(
    include_str!("boot.S"),
    monitor_stack_bytes = const MONITOR_STACK_BYTES,
);
global_asm!(include_str!("exceptions.S"));
global_asm!(include_str!("threads.S"));
global_asm!(include_str!("usercopy.S"));
#[cfg(feature = "el0-fault-containment-self-test")]
global_asm!(include_str!("user_init.S"));

pub fn current_exception_level() -> u8 {
    let current_el: u64;
    unsafe {
        asm!(
            "mrs {current_el}, CurrentEL",
            current_el = out(reg) current_el,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((current_el >> 2) & 0b11) as u8
}

pub fn halt() -> ! {
    unsafe {
        asm!(
            "msr daifset, #0xf",
            "isb",
            options(nomem, nostack, preserves_flags)
        );
    }
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Exits the QEMU process through the AArch64 semihosting debug contract.
///
/// The opaque token can only be constructed after the complete M66 resident,
/// storage, IRQ, and durable-close proof succeeds. This is intentionally not a
/// PSCI or physical-device poweroff implementation. Returning from `HLT` is a
/// fail-closed kernel panic rather than a return to userspace.
#[cfg(feature = "resident-platform-shutdown-runtime")]
pub fn qemu_semihosting_power_off(
    proof: bndroid_kernel::platform_shutdown::ValidatedShutdown,
) -> ! {
    use bndroid_kernel::platform_shutdown::Backend;

    const SYS_EXIT_EXTENDED: u64 = 0x20;
    const ADP_STOPPED_APPLICATION_EXIT: u64 = 0x0002_0026;

    #[repr(C, align(16))]
    struct ExitBlock {
        reason: u64,
        subcode: u64,
    }

    if proof.backend() != Backend::QemuSemihosting
        || proof.psci().is_some()
        || proof.generation() == 0
    {
        panic!("M66 platform backend received an invalid validated token");
    }
    let block = ExitBlock {
        reason: ADP_STOPPED_APPLICATION_EXIT,
        subcode: 0,
    };
    unsafe {
        asm!(
            "hlt #0xf000",
            in("x0") SYS_EXIT_EXTENDED,
            in("x1") core::ptr::addr_of!(block) as u64,
            options(nostack)
        );
    }
    panic!("M66 QEMU semihosting poweroff returned unexpectedly");
}

/// Probes the PSCI firmware version over the exact FDT-selected conduit.
///
/// M70 calls this before installing the descriptor into the platform shutdown
/// contract. A negative firmware return or a version older than PSCI 0.2 is
/// rejected before userspace starts.
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
pub fn psci_version(method: bndroid_kernel::fdt::PsciMethod) -> Result<u32, &'static str> {
    const PSCI_VERSION: u64 = 0x8400_0000;

    let raw = unsafe { psci_call(method, PSCI_VERSION) } as u32;
    if (raw as i32) < 0 {
        return Err("PSCI_VERSION returned a firmware error");
    }
    if !bndroid_kernel::platform_shutdown::psci_version_supported(raw) {
        return Err("PSCI version is older than 0.2");
    }
    Ok(raw)
}

/// Requests QEMU `virt` shutdown through PSCI `SYSTEM_OFF`.
///
/// The descriptor must match the one discovered from `/psci`, successfully
/// probed with `PSCI_VERSION`, installed once during boot, and then bound into
/// the opaque final shutdown token. This remains QEMU evidence, not a physical
/// PMIC or real-phone poweroff claim.
#[cfg(feature = "unified-product-psci-shutdown-runtime")]
pub fn qemu_psci_power_off(proof: bndroid_kernel::platform_shutdown::ValidatedShutdown) -> ! {
    use bndroid_kernel::platform_shutdown::{Backend, installed_psci};

    const PSCI_SYSTEM_OFF: u64 = 0x8400_0008;

    let descriptor = proof
        .psci()
        .unwrap_or_else(|| panic!("M70 PSCI backend received a token without a descriptor"));
    if proof.backend() != Backend::QemuPsci
        || proof.generation() == 0
        || installed_psci() != Some(descriptor)
    {
        panic!("M70 PSCI backend received an invalid validated token");
    }

    unsafe {
        asm!("dsb sy", "isb", options(nostack, preserves_flags));
        let _ = psci_call(descriptor.method, PSCI_SYSTEM_OFF);
    }
    panic!("M70 QEMU PSCI SYSTEM_OFF returned unexpectedly");
}

#[cfg(feature = "unified-product-psci-shutdown-runtime")]
unsafe fn psci_call(method: bndroid_kernel::fdt::PsciMethod, function_id: u64) -> u64 {
    use bndroid_kernel::fdt::PsciMethod;

    let result: u64;
    match method {
        PsciMethod::Hvc => unsafe {
            asm!(
                "hvc #0",
                inlateout("x0") function_id => result,
                inlateout("x1") 0_u64 => _,
                inlateout("x2") 0_u64 => _,
                inlateout("x3") 0_u64 => _,
                clobber_abi("C"),
                options(nostack)
            );
        },
        PsciMethod::Smc => unsafe {
            asm!(
                "smc #0",
                inlateout("x0") function_id => result,
                inlateout("x1") 0_u64 => _,
                inlateout("x2") 0_u64 => _,
                inlateout("x3") 0_u64 => _,
                clobber_abi("C"),
                options(nostack)
            );
        },
    }
    result
}

#[cfg(feature = "el0-fault-containment-self-test")]
pub fn idle() -> ! {
    loop {
        wait_for_interrupt();
    }
}

pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

pub fn enable_irq() {
    unsafe {
        asm!(
            "msr daifclr, #2",
            "isb",
            options(nomem, nostack, preserves_flags)
        );
    }
}

pub fn irq_is_masked() -> bool {
    let daif: u64;
    unsafe {
        asm!(
            "mrs {daif}, daif",
            daif = out(reg) daif,
            options(nomem, nostack, preserves_flags)
        );
    }
    daif & (1 << 7) != 0
}

#[cfg(feature = "app-data-runtime")]
pub fn stack_pointer() -> usize {
    let stack_pointer: usize;
    unsafe {
        asm!(
            "mov {stack_pointer}, sp",
            stack_pointer = out(reg) stack_pointer,
            options(nomem, nostack, preserves_flags)
        );
    }
    stack_pointer
}

/// Returns DAIF after masking IRQ locally on the current CPU.
pub fn save_and_mask_irq() -> u64 {
    let saved: u64;
    unsafe {
        asm!(
            "mrs {saved}, daif",
            "msr daifset, #2",
            "isb",
            saved = out(reg) saved,
            options(nostack, preserves_flags)
        );
    }
    saved
}

/// Restores the exact interrupt-mask state returned by `save_and_mask_irq`.
pub fn restore_daif(saved: u64) {
    unsafe {
        asm!(
            "msr daif, {saved}",
            "isb",
            saved = in(reg) saved,
            options(nostack, preserves_flags)
        );
    }
}
