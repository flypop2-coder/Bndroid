    .macro LOAD64 reg, h0, h1, h2, h3
        movz \reg, #\h0
        movk \reg, #\h1, lsl #16
        movk \reg, #\h2, lsl #32
        movk \reg, #\h3, lsl #48
    .endm

    .equ SVC_KERNEL_SLEEP, 0xb0

    // Each validation worker is deliberately CPU-bound until the monitor says
    // the test is complete. It cannot yield into the scheduler: only a timer
    // IRQ can move execution to another context. The persistent x19, x22, q8,
    // FPCR and FPSR values exercise every register class in TrapFrame.
    .macro SCHED_WORKER name, selected, observed, work, sleep_resume, sleep_returns, sleep_epoch, sleep_ticks, g0, g1, g2, g3, q00, q01, q02, q03, q10, q11, q12, q13, fpcr_hi, fpsr_hi, fpsr_lo
        .global \name
        .type \name, %function
\name:
        LOAD64 x19, \g0, \g1, \g2, \g3
        LOAD64 x9, \q00, \q01, \q02, \q03
        LOAD64 x10, \q10, \q11, \q12, \q13
        ins v8.d[0], x9
        ins v8.d[1], x10
        movz x9, #\fpcr_hi, lsl #16
        msr fpcr, x9
        movz x9, #\fpsr_lo
        movk x9, #\fpsr_hi, lsl #16
        msr fpsr, x9
        isb
        mov x22, xzr
        mov x23, xzr

1:
        LOAD64 x9, \g0, \g1, \g2, \g3
        cmp x19, x9
        b.ne __scheduler_register_failure

        LOAD64 x9, \q00, \q01, \q02, \q03
        umov x11, v8.d[0]
        cmp x11, x9
        b.ne __scheduler_register_failure
        LOAD64 x9, \q10, \q11, \q12, \q13
        umov x11, v8.d[1]
        cmp x11, x9
        b.ne __scheduler_register_failure

        mrs x11, fpcr
        movz x9, #\fpcr_hi, lsl #16
        cmp x11, x9
        b.ne __scheduler_register_failure
        mrs x11, fpsr
        movz x9, #\fpsr_lo
        movk x9, #\fpsr_hi, lsl #16
        cmp x11, x9
        b.ne __scheduler_register_failure

        // On the first pass after SVC, independently prove that the preserved
        // x22 still equals its exact sleep baseline.
        cmp x23, #2
        b.ne 7f
        ldr x9, =\sleep_epoch
        ldar x9, [x9]
        cmp x22, x9
        b.ne __scheduler_epoch_failure
7:

        // The worker-owned observed mirror must agree with the x22 value from
        // the saved TrapFrame. Reusing an old initial frame would restart x22
        // at zero while the mirror remains nonzero, so continuity still fails
        // without assuming that a selected context executes before the next
        // physical timer interrupt.
        ldr x9, =\observed
        ldar x10, [x9]
        cmp x22, x10
        b.ne __scheduler_epoch_failure

        // A host stall may let multiple strictly newer selections coalesce
        // before this worker executes an instruction. Accept that legitimate
        // gap, but reject any non-monotonic selection epoch.
        ldr x9, =\selected
        ldar x10, [x9]
        cmp x10, x22
        b.eq 2f
        b.ls __scheduler_epoch_failure
        mov x22, x10
        ldr x9, =\observed
        stlr x22, [x9]

2:
        ldr x9, =\work
        ldar x10, [x9]
        add x10, x10, #1
        stlr x10, [x9]

        ldr x9, =SCHED_VALIDATION_COMPLETE
        ldar x10, [x9]
        cbnz x10, 3f
        b 1b

3:
        cmp x23, #2
        b.eq 6f
        cbnz x23, 4f
        mov x23, #1
        ldr x9, =\sleep_epoch
        stlr x22, [x9]
        mov x0, #\sleep_ticks
        svc #SVC_KERNEL_SLEEP
        cbnz x0, __scheduler_sleep_status_failure

        // Returning past SVC proves that the original synchronous TrapFrame
        // was retained while sleeping and restored only after timer wakeup.
        ldr x9, =SCHED_CURRENT_TICK
        ldar x10, [x9]
        ldr x9, =\sleep_resume
        stlr x10, [x9]
        mov x23, #2
        b 1b

        // Publish completion only after label 1 has revalidated the complete
        // preserved register set and label 2 has made forward progress.
6:
        mov x23, #3
        ldr x9, =\sleep_returns
        ldar x10, [x9]
        add x10, x10, #1
        stlr x10, [x9]
        b 1b

4:
        yield
        b 1b
        .size \name, . - \name
    .endm

    .section .text.scheduler_workers, "ax"
    .balign 16

    SCHED_WORKER __scheduler_worker_0, SCHED_WORKER_0_SELECTED, SCHED_WORKER_0_OBSERVED, SCHED_WORKER_0_WORK, SCHED_WORKER_0_SLEEP_RESUME_TICK, SCHED_WORKER_0_SLEEP_RETURNS, SCHED_WORKER_0_SLEEP_EPOCH_BASELINE, 12, 0xace0, 0x2468, 0x9bdf, 0x1357, 0xcdef, 0x89ab, 0x4567, 0x0123, 0x3210, 0x7654, 0xba98, 0xfedc, 0x0200, 0x0000, 0x0001
    SCHED_WORKER __scheduler_worker_1, SCHED_WORKER_1_SELECTED, SCHED_WORKER_1_OBSERVED, SCHED_WORKER_1_WORK, SCHED_WORKER_1_SLEEP_RESUME_TICK, SCHED_WORKER_1_SLEEP_RETURNS, SCHED_WORKER_1_SLEEP_EPOCH_BASELINE, 6, 0x9687, 0xb4a5, 0xd2c3, 0xf0e1, 0x7788, 0x5566, 0x3344, 0x1122, 0x8877, 0xaa99, 0xccbb, 0xeedd, 0x0100, 0x0800, 0x0000

__scheduler_register_failure:
    ldr x9, =SCHED_REGISTER_FAILURES
    mov x10, #1
    stlr x10, [x9]
    b __scheduler_failed_thread

__scheduler_epoch_failure:
    ldr x9, =SCHED_EPOCH_FAILURES
    mov x10, #1
    stlr x10, [x9]
    b __scheduler_failed_thread

__scheduler_sleep_status_failure:
    ldr x9, =SCHED_SLEEP_STATUS_FAILURES
    mov x10, #1
    stlr x10, [x9]

__scheduler_failed_thread:
    wfi
    b __scheduler_failed_thread
