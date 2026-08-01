    .macro VECTOR_FATAL kind
        mov x0, #\kind
        b __exception_common
        .balign 128
    .endm

    .macro VECTOR_IRQ
        b __irq_common
        .balign 128
    .endm

    .macro VECTOR_SYNC
        b __sync_common
        .balign 128
    .endm

    .macro VECTOR_LOWER_SYNC
        b __lower_sync_common
        .balign 128
    .endm

    // TrapFrame layout is mirrored by arch::aarch64::exceptions::TrapFrame.
    // IRQ and returnable synchronous exceptions share these macros so their
    // context-switch ABI cannot drift apart.
    .macro SAVE_TRAP_FRAME
        sub sp, sp, #800

        stp x0, x1, [sp, #0]
        stp x2, x3, [sp, #16]
        stp x4, x5, [sp, #32]
        stp x6, x7, [sp, #48]
        stp x8, x9, [sp, #64]
        stp x10, x11, [sp, #80]
        stp x12, x13, [sp, #96]
        stp x14, x15, [sp, #112]
        stp x16, x17, [sp, #128]
        stp x18, x19, [sp, #144]
        stp x20, x21, [sp, #160]
        stp x22, x23, [sp, #176]
        stp x24, x25, [sp, #192]
        stp x26, x27, [sp, #208]
        stp x28, x29, [sp, #224]
        str x30, [sp, #240]

        mrs x9, sp_el0
        str x9, [sp, #248]
        mrs x9, elr_el1
        str x9, [sp, #256]
        mrs x9, spsr_el1
        str x9, [sp, #264]

        stp q0, q1, [sp, #272]
        stp q2, q3, [sp, #304]
        stp q4, q5, [sp, #336]
        stp q6, q7, [sp, #368]
        stp q8, q9, [sp, #400]
        stp q10, q11, [sp, #432]
        stp q12, q13, [sp, #464]
        stp q14, q15, [sp, #496]
        stp q16, q17, [sp, #528]
        stp q18, q19, [sp, #560]
        stp q20, q21, [sp, #592]
        stp q22, q23, [sp, #624]
        stp q24, q25, [sp, #656]
        stp q26, q27, [sp, #688]
        stp q28, q29, [sp, #720]
        stp q30, q31, [sp, #752]

        mrs x9, fpcr
        str x9, [sp, #784]
        mrs x9, fpsr
        str x9, [sp, #792]
    .endm

    .macro RESTORE_TRAP_FRAME
        ldr x9, [sp, #256]
        msr elr_el1, x9
        ldr x9, [sp, #264]
        msr spsr_el1, x9
        ldr x9, [sp, #248]
        msr sp_el0, x9

        ldr x9, [sp, #784]
        msr fpcr, x9
        ldr x9, [sp, #792]
        msr fpsr, x9

        ldp q0, q1, [sp, #272]
        ldp q2, q3, [sp, #304]
        ldp q4, q5, [sp, #336]
        ldp q6, q7, [sp, #368]
        ldp q8, q9, [sp, #400]
        ldp q10, q11, [sp, #432]
        ldp q12, q13, [sp, #464]
        ldp q14, q15, [sp, #496]
        ldp q16, q17, [sp, #528]
        ldp q18, q19, [sp, #560]
        ldp q20, q21, [sp, #592]
        ldp q22, q23, [sp, #624]
        ldp q24, q25, [sp, #656]
        ldp q26, q27, [sp, #688]
        ldp q28, q29, [sp, #720]
        ldp q30, q31, [sp, #752]

        ldp x2, x3, [sp, #16]
        ldp x4, x5, [sp, #32]
        ldp x6, x7, [sp, #48]
        ldp x8, x9, [sp, #64]
        ldp x10, x11, [sp, #80]
        ldp x12, x13, [sp, #96]
        ldp x14, x15, [sp, #112]
        ldp x16, x17, [sp, #128]
        ldp x18, x19, [sp, #144]
        ldp x20, x21, [sp, #160]
        ldp x22, x23, [sp, #176]
        ldp x24, x25, [sp, #192]
        ldp x26, x27, [sp, #208]
        ldp x28, x29, [sp, #224]
        ldr x30, [sp, #240]
        ldp x0, x1, [sp, #0]
        add sp, sp, #800
    .endm

    .section .text.exceptions, "ax"
    .align 11
    .global __exception_vectors
__exception_vectors:
    // Current EL with SP0.
    VECTOR_FATAL 0
    VECTOR_IRQ
    VECTOR_FATAL 2
    VECTOR_FATAL 3

    // Current EL with SPx. SVC-based kernel operations use this sync slot.
    VECTOR_SYNC
    VECTOR_IRQ
    VECTOR_FATAL 6
    VECTOR_FATAL 7

    // Lower EL using AArch64.
    VECTOR_LOWER_SYNC
    VECTOR_IRQ
    VECTOR_FATAL 10
    VECTOR_FATAL 11

    // Lower EL using AArch32.
    VECTOR_FATAL 12
    VECTOR_IRQ
    VECTOR_FATAL 14
    VECTOR_FATAL 15

// Save every GPR and SIMD/FP register: rustc may use SIMD registers in ordinary
// integer code, so every returnable exception must preserve them all.
__irq_common:
    msr daifset, #2
    SAVE_TRAP_FRAME
    mov x0, sp
    bl irq_dispatch
    mov sp, x0
    RESTORE_TRAP_FRAME
    eret

__sync_common:
    msr daifset, #2
    SAVE_TRAP_FRAME
    mov x0, sp
    bl sync_dispatch
    mov sp, x0
    RESTORE_TRAP_FRAME
    eret

// Lower-EL SVC is deliberately dispatched through a separate symbol so the
// private current-EL sleep operation can never become a user syscall by
// immediate-number confusion.
__lower_sync_common:
    msr daifset, #2
    SAVE_TRAP_FRAME
    mov x0, sp
    bl lower_sync_dispatch
    mov sp, x0
    RESTORE_TRAP_FRAME
    eret

__exception_common:
    bl exception_handler
1:
    wfe
    b 1b
