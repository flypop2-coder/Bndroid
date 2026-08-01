use core::arch::asm;

unsafe extern "C" {
    static __exception_vectors: u8;
}

#[repr(C, align(16))]
pub struct TrapFrame {
    pub x: [u64; 31],
    pub sp_el0: u64,
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub q: [u128; 32],
    pub fpcr: u64,
    pub fpsr: u64,
}

impl TrapFrame {
    /// Builds the exception-return image used for a new EL1h kernel thread.
    ///
    /// D, A and F remain masked, while IRQ is deliberately unmasked so the
    /// generic timer can preempt the thread immediately after `eret`.
    pub fn for_kernel_thread(
        entry: u64,
        argument: u64,
        thread_exit: u64,
        pan_enabled: bool,
    ) -> Self {
        const SPSR_EL1H_DAF_MASKED_IRQ_ENABLED: u64 = 0x345;

        let mut frame = Self {
            x: [0; 31],
            sp_el0: 0,
            elr_el1: entry,
            spsr_el1: SPSR_EL1H_DAF_MASKED_IRQ_ENABLED,
            q: [0; 32],
            fpcr: 0,
            fpsr: 0,
        };
        frame.x[0] = argument;
        frame.x[30] = thread_exit;
        if pan_enabled {
            frame.spsr_el1 |= 1 << 22;
        }
        frame
    }

    /// Builds an exception-return image for an AArch64 EL0t task.
    ///
    /// D, A and F remain masked while IRQ is enabled. `sp_el0` is the user
    /// stack; the TrapFrame itself must live on the task's kernel exception
    /// stack so a lower-EL IRQ never trusts user-controlled stack memory.
    pub fn for_user_thread(entry: u64, user_stack: u64, argument: u64) -> Self {
        const SPSR_EL0T_DAF_MASKED_IRQ_ENABLED: u64 = 0x340;

        let mut frame = Self {
            x: [0; 31],
            sp_el0: user_stack,
            elr_el1: entry,
            spsr_el1: SPSR_EL0T_DAF_MASKED_IRQ_ENABLED,
            q: [0; 32],
            fpcr: 0,
            fpsr: 0,
        };
        frame.x[0] = argument;
        frame
    }
}

const _: [(); 800] = [(); core::mem::size_of::<TrapFrame>()];
const _: [(); 248] = [(); core::mem::offset_of!(TrapFrame, sp_el0)];
const _: [(); 256] = [(); core::mem::offset_of!(TrapFrame, elr_el1)];
const _: [(); 264] = [(); core::mem::offset_of!(TrapFrame, spsr_el1)];
const _: [(); 272] = [(); core::mem::offset_of!(TrapFrame, q)];
const _: [(); 784] = [(); core::mem::offset_of!(TrapFrame, fpcr)];
const _: [(); 792] = [(); core::mem::offset_of!(TrapFrame, fpsr)];

const ESR_EC_SHIFT: u64 = 26;
const ESR_EC_MASK: u64 = 0x3f;
const ESR_EC_SVC64: u64 = 0x15;
const ESR_EC_DATA_ABORT_CURRENT_EL: u64 = 0x25;
const ESR_SVC_IMMEDIATE_MASK: u64 = 0xffff;
pub const SVC_KERNEL_SLEEP: u64 = 0xb0;

pub fn install() {
    let vector_address = core::ptr::addr_of!(__exception_vectors) as u64;
    unsafe {
        asm!(
            "msr VBAR_EL1, {vectors}",
            "isb",
            vectors = in(reg) vector_address,
            options(nostack, preserves_flags)
        );
    }
}

#[unsafe(no_mangle)]
extern "C" fn exception_handler(kind: u64) -> ! {
    let esr: u64;
    let elr: u64;
    let far: u64;
    let spsr: u64;

    unsafe {
        asm!(
            "mrs {esr}, ESR_EL1",
            "mrs {elr}, ELR_EL1",
            "mrs {far}, FAR_EL1",
            "mrs {spsr}, SPSR_EL1",
            esr = out(reg) esr,
            elr = out(reg) elr,
            far = out(reg) far,
            spsr = out(reg) spsr,
            options(nomem, nostack, preserves_flags)
        );
    }

    crate::kprintln!("fatal exception: {}", description(kind));
    crate::kprintln!(
        "  ESR={:#018x} ELR={:#018x} FAR={:#018x} SPSR={:#018x}",
        esr,
        elr,
        far,
        spsr
    );
    super::halt()
}

#[unsafe(no_mangle)]
extern "C" fn sync_dispatch(frame: *mut TrapFrame) -> *mut TrapFrame {
    let esr: u64;
    let far: u64;
    unsafe {
        asm!(
            "mrs {esr}, ESR_EL1",
            "mrs {far}, FAR_EL1",
            esr = out(reg) esr,
            far = out(reg) far,
            options(nomem, nostack, preserves_flags)
        );
    }

    let exception_class = (esr >> ESR_EC_SHIFT) & ESR_EC_MASK;
    let immediate = esr & ESR_SVC_IMMEDIATE_MASK;
    let source_mode = unsafe { (*frame).spsr_el1 & 0x1f };
    if source_mode == 0x5
        && exception_class == ESR_EC_DATA_ABORT_CURRENT_EL
        && super::usercopy::try_fixup_data_abort(unsafe { &mut *frame }, esr, far)
    {
        return frame;
    }
    if source_mode == 0x5 && exception_class == ESR_EC_SVC64 && immediate == SVC_KERNEL_SLEEP {
        return crate::scheduler::sleep_current(frame);
    }

    exception_handler(4)
}

#[unsafe(no_mangle)]
extern "C" fn lower_sync_dispatch(frame: *mut TrapFrame) -> *mut TrapFrame {
    let esr: u64;
    let far: u64;
    unsafe {
        asm!(
            "mrs {esr}, ESR_EL1",
            "mrs {far}, FAR_EL1",
            esr = out(reg) esr,
            far = out(reg) far,
            options(nomem, nostack, preserves_flags)
        );
    }
    let exception_class = (esr >> ESR_EC_SHIFT) & ESR_EC_MASK;
    let immediate = esr & ESR_SVC_IMMEDIATE_MASK;
    let source_mode = unsafe { (*frame).spsr_el1 & 0x1f };
    if source_mode == 0 && exception_class == ESR_EC_SVC64 {
        if !crate::scheduler::validate_current_user_frame(frame) {
            return crate::scheduler::fault_current_user(
                frame,
                crate::scheduler::UserFaultReason::InvalidReturnState,
                esr,
                far,
            );
        }
        if immediate == u64::from(bndr_abi::SVC_USER_SYSCALL) {
            return crate::syscall::dispatch(frame);
        }
        return crate::syscall::reject_svc_immediate(frame, immediate);
    }
    if source_mode == 0 {
        return crate::scheduler::fault_current_user(
            frame,
            crate::scheduler::UserFaultReason::SynchronousException,
            esr,
            far,
        );
    }
    exception_handler(8)
}

fn description(kind: u64) -> &'static str {
    match kind {
        0 => "synchronous from current EL using SP0",
        1 => "IRQ from current EL using SP0",
        2 => "FIQ from current EL using SP0",
        3 => "SError from current EL using SP0",
        4 => "synchronous from current EL using SPx",
        5 => "IRQ from current EL using SPx",
        6 => "FIQ from current EL using SPx",
        7 => "SError from current EL using SPx",
        8 => "synchronous from lower EL using AArch64",
        9 => "IRQ from lower EL using AArch64",
        10 => "FIQ from lower EL using AArch64",
        11 => "SError from lower EL using AArch64",
        12 => "synchronous from lower EL using AArch32",
        13 => "IRQ from lower EL using AArch32",
        14 => "FIQ from lower EL using AArch32",
        15 => "SError from lower EL using AArch32",
        _ => "unknown exception",
    }
}
