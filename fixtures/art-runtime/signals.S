.text
.global signal_register_probe
.type signal_register_probe, %function
signal_register_probe:
    sub sp, sp, #96
    stp x19, x20, [sp]
    stp x29, x30, [sp, #16]
    str q8, [sp, #32]
    mrs x3, fpcr
    str x3, [sp, #64]
    mrs x3, fpsr
    str x3, [sp, #72]
    mov x19, #0x1234
    mov x20, #0x5678
    mov x3, #-1
    dup v8.2d, x3
    mov x3, #0x400000
    msr fpcr, x3
    mov x3, #0x8000000
    msr fpsr, x3
    mov x8, #131
    svc #0
    cbnz x0, 1f
    mov x4, #0x1234
    cmp x19, x4
    b.ne 1f
    mov x4, #0x5678
    cmp x20, x4
    b.ne 1f
    umov x3, v8.d[0]
    cmn x3, #1
    b.ne 1f
    umov x3, v8.d[1]
    cmn x3, #1
    b.ne 1f
    mrs x3, fpcr
    mov x4, #0x400000
    cmp x3, x4
    b.ne 1f
    mrs x3, fpsr
    mov x4, #0x8000000
    cmp x3, x4
    b.ne 1f
    mov x0, #0
    b 2f
1:  mov x0, #1
2:  ldr x3, [sp, #64]
    msr fpcr, x3
    ldr x3, [sp, #72]
    msr fpsr, x3
    ldr q8, [sp, #32]
    ldp x29, x30, [sp, #16]
    ldp x19, x20, [sp]
    add sp, sp, #96
    ret
.size signal_register_probe, .-signal_register_probe
.section .note.GNU-stack,"",%progbits
