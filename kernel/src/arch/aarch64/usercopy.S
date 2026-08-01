    .section .text.usercopy, "ax"
    .balign 4

    // Returns the number of bytes that were not copied. Only the LDTRB site
    // is recoverable; a bad kernel destination remains a fatal kernel fault.
    .global __bndroid_copy_from_user
    .type __bndroid_copy_from_user, %function
__bndroid_copy_from_user:
    cbz x2, .Lcopy_from_success
.Lcopy_from_loop:
.Lcopy_from_fault:
    ldtrb w3, [x1]
    strb w3, [x0]
    add x0, x0, #1
    add x1, x1, #1
    subs x2, x2, #1
    b.ne .Lcopy_from_loop
.Lcopy_from_success:
    mov x0, xzr
    ret
.Lcopy_from_fixup:
    mov x0, x2
    ret
    .size __bndroid_copy_from_user, . - __bndroid_copy_from_user

    // Returns the number of bytes that were not copied. Only the STTRB site
    // is recoverable; a bad kernel source remains a fatal kernel fault.
    .global __bndroid_copy_to_user
    .type __bndroid_copy_to_user, %function
__bndroid_copy_to_user:
    cbz x2, .Lcopy_to_success
.Lcopy_to_loop:
    ldrb w3, [x1]
.Lcopy_to_fault:
    sttrb w3, [x0]
    add x0, x0, #1
    add x1, x1, #1
    subs x2, x2, #1
    b.ne .Lcopy_to_loop
.Lcopy_to_success:
    mov x0, xzr
    ret
.Lcopy_to_fixup:
    mov x0, x2
    ret
    .size __bndroid_copy_to_user, . - __bndroid_copy_to_user

    // repr(C) UserCopyExceptionEntry:
    // fault PC, fixup PC, expected WnR, FAR register, active direction.
    .pushsection .usercopy_ex_table, "a", %progbits
    .balign 8
    .quad .Lcopy_from_fault
    .quad .Lcopy_from_fixup
    .quad 0
    .quad 1
    .quad 1

    .quad .Lcopy_to_fault
    .quad .Lcopy_to_fixup
    .quad 1
    .quad 0
    .quad 2
    .popsection
