    // Relocatable one-page EL0 init image. It deliberately contains no
    // external references or literal pool: the kernel copies these bytes to a
    // separately allocated user RX page before the task becomes runnable.
    .section .text.user_init, "ax"
    .balign 4
    .global __user_init_template_start
    .global __user_init_template_end
    .global __user_init_fault_template_entry
    .type __user_init_template_start, %function

__user_init_template_start:
    // x23 is the register-context sentinel checked after SVC and timer IRQ.
    movz x23, #0xc0de
    movk x23, #0x0bad, lsl #16
    movk x23, #0xa11e, lsl #32
    movk x23, #0xd15c, lsl #48

    // Prove that the dedicated EL0 stack is writable and readable.
    movz x22, #0xbaad
    movk x22, #0xf00d, lsl #16
    movk x22, #0xc3c3, lsl #32
    movk x22, #0x5a5a, lsl #48
    sub sp, sp, #16
    str x22, [sp]
    ldr x24, [sp]
    add sp, sp, #16
    cmp x24, x22
    b.ne .Lfail_stack_initial

    // A lower-EL svc #0xb0 must never reach the private EL1 sleep path.
    svc #0xb0
    cmp x0, #9
    b.ne .Lfail_private_svc

    // Query the shared scalar ABI version.
    mov x8, #0
    svc #0
    cbnz x0, .Lfail_abi_status
    cmp x1, #1
    b.ne .Lfail_abi_version

    // Unknown syscall numbers are rejected without altering the task.
    mov x8, #0xffff
    svc #0
    cmp x0, #9
    b.ne .Lfail_unknown

    // Create a channel pair. Handles live in x19/x20 across later traps.
    mov x8, #1
    svc #0
    cbnz x0, .Lfail_create
    mov x19, x1
    mov x20, x2
    cbz x19, .Lfail_create_handle
    cbz x20, .Lfail_create_handle
    cmp x19, x20
    b.eq .Lfail_create_handle

    // Send one inline scalar message from the left endpoint.
    mov x0, x19
    mov x1, #0x1234
    movz x2, #0xcdef
    movk x2, #0x89ab, lsl #16
    movk x2, #0x4567, lsl #32
    movk x2, #0x0123, lsl #48
    mov x8, #2
    svc #0
    cbnz x0, .Lfail_write

    // Receive it from the peer and validate FIFO payload integrity.
    mov x0, x20
    mov x8, #3
    svc #0
    cbnz x0, .Lfail_read
    mov x24, #0x1234
    cmp x1, x24
    b.ne .Lfail_read_tag
    movz x24, #0xcdef
    movk x24, #0x89ab, lsl #16
    movk x24, #0x4567, lsl #32
    movk x24, #0x0123, lsl #48
    cmp x2, x24
    b.ne .Lfail_read_payload

    // Duplicate left with WRITE-only rights, then prove READ is denied.
    mov x0, x19
    mov x1, #2
    mov x8, #4
    svc #0
    cbnz x0, .Lfail_duplicate
    mov x21, x1
    cbz x21, .Lfail_duplicate_handle
    mov x0, x21
    mov x8, #3
    svc #0
    cmp x0, #2
    b.ne .Lfail_rights

    // Close all live handles, including one deliberate stale-handle retry.
    mov x0, x21
    mov x8, #5
    svc #0
    cbnz x0, .Lfail_close_duplicate
    mov x0, x19
    mov x8, #5
    svc #0
    cbnz x0, .Lfail_close_left
    mov x0, x19
    mov x8, #5
    svc #0
    cmp x0, #3
    b.ne .Lfail_stale
    mov x0, x20
    mov x8, #5
    svc #0
    cbnz x0, .Lfail_close_right

.Lready_retry:
    // Re-read the user stack immediately before every readiness SVC. The
    // kernel accepts readiness only after a lower-EL timer IRQ and a later
    // reschedule, so the successful values prove both contexts survived.
    sub sp, sp, #16
    str x22, [sp]
    ldr x2, [sp]
    add sp, sp, #16
    movz x0, #0xf00d
    movk x0, #0xbaad, lsl #16
    movk x0, #0x1a17, lsl #32
    movk x0, #0xc0de, lsl #48
    mov x1, x23
    mov x8, #6
    svc #0
    cmp x0, #10
    b.eq .Lready_retry
    cbnz x0, .Lfail_ready

.Lsuccess:
    yield
    b .Lsuccess

.Lfail_stack_initial:
    mov x0, #1
    b .Lreport_failure
.Lfail_private_svc:
    mov x0, #2
    b .Lreport_failure
.Lfail_abi_status:
    mov x0, #3
    b .Lreport_failure
.Lfail_abi_version:
    mov x0, #4
    b .Lreport_failure
.Lfail_unknown:
    mov x0, #5
    b .Lreport_failure
.Lfail_create:
    mov x0, #6
    b .Lreport_failure
.Lfail_create_handle:
    mov x0, #7
    b .Lreport_failure
.Lfail_write:
    mov x0, #8
    b .Lreport_failure
.Lfail_read:
    mov x0, #9
    b .Lreport_failure
.Lfail_read_tag:
    mov x0, #10
    b .Lreport_failure
.Lfail_read_payload:
    mov x0, #11
    b .Lreport_failure
.Lfail_duplicate:
    mov x0, #12
    b .Lreport_failure
.Lfail_duplicate_handle:
    mov x0, #13
    b .Lreport_failure
.Lfail_rights:
    mov x0, #14
    b .Lreport_failure
.Lfail_close_duplicate:
    mov x0, #15
    b .Lreport_failure
.Lfail_close_left:
    mov x0, #16
    b .Lreport_failure
.Lfail_stale:
    mov x0, #17
    b .Lreport_failure
.Lfail_close_right:
    mov x0, #18
    b .Lreport_failure
.Lfail_ready:
    mov x0, #19

.Lreport_failure:
    mov x8, #7
    svc #0
.Lfailure_halt:
    yield
    b .Lfailure_halt

// Negative-test entry: an EL0 read of the EL1-only kernel text page must
// become a contained lower-EL permission fault. No external relocation is
// used, so this entry remains valid after the whole template is copied.
__user_init_fault_template_entry:
    movz x9, #0x0000
    movk x9, #0x4008, lsl #16
    ldr x10, [x9]

    // Reaching this SVC means the permission boundary failed.
    mov x0, #32
    mov x8, #7
    svc #0
.Lfault_test_failed:
    yield
    b .Lfault_test_failed

__user_init_template_end:
    .size __user_init_template_start, .-__user_init_template_start
