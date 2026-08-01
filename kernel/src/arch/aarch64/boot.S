    .section .text.boot
    .global _start
    .type _start, %function

_start:
    // The QEMU/AArch64 boot protocol passes the flattened device tree in x0.
    mov x19, x0
    msr daifset, #0xf

    // The current M1 kernel is single-core. Defensively park any secondary
    // QEMU-style affinity before it can share the boot stack or clear BSS.
    // The platform contract currently requires the primary affinity to be 0.
    mrs x1, MPIDR_EL1
    and x2, x1, #0x00ffffff
    cbnz x2, .Lsecondary_park
    ubfx x2, x1, #32, #8
    cbnz x2, .Lsecondary_park

    // Normalize a common firmware/QEMU EL2 entry to EL1h. The rest of the
    // kernel has a strict EL1 contract; continuing at EL2 would make EL1 page
    // tables and VBAR ineffective while producing misleading boot logs.
    // Rust-generated AArch64 code may use FP/SIMD registers even when the
    // source contains no floating-point operations. Enable them before the
    // first Rust instruction and mask interrupts until the GIC is ready.
    mrs x1, CurrentEL
    lsr x20, x1, #2
    cmp x1, #(2 << 2)
    b.eq .Ldrop_from_el2
    cmp x1, #(1 << 2)
    b.eq .Lenter_el1
    b .Lunsupported_el

.Ldrop_from_el2:
    // EL1 executes AArch64, may access the physical counter/timer, and does
    // not trap FP/SIMD through EL2.
    mov x1, #(1 << 31)
    msr hcr_el2, x1
    mrs x1, cnthctl_el2
    orr x1, x1, #3
    msr cnthctl_el2, x1
    msr cntvoff_el2, xzr
    msr cptr_el2, xzr

    // Return to EL1h with D/A/I/F masked. MMU and caches remain disabled.
    mov x1, #0x3c5
    msr spsr_el2, x1
    adr x1, .Lenter_el1
    msr elr_el2, x1
    isb
    eret

.Lenter_el1:
    mrs x1, cpacr_el1
    orr x1, x1, #(3 << 20)
    msr cpacr_el1, x1
    isb

    msr SPSel, #1
    ldr x30, =__stack_top
    mov sp, x30
    ldr x1, =__sp0_stack_top
    msr sp_el0, x1

    // Rust requires zero-initialized BSS. Do not rely on a particular loader.
    ldr x1, =__bss_start
    ldr x2, =__bss_end
1:
    cmp x1, x2
    b.hs 2f
    str xzr, [x1], #8
    b 1b

2:
    ldr x1, =__exception_vectors
    msr vbar_el1, x1
    isb

    mov x0, x19
    mov x1, x20
    mov x29, xzr
    ldr x2, =kernel_main
    blr x2

3:
    wfe
    b 3b

.Lunsupported_el:
    wfe
    b .Lunsupported_el

.Lsecondary_park:
    wfe
    b .Lsecondary_park

    .size _start, . - _start

    .section .bss.stack, "aw", %nobits
    .align 12
    .global __stack_bottom
__stack_bottom:
    // The single boot/monitor context carries the debug-build validation
    // ledger. It is intentionally larger than the fixed 16 KiB worker and
    // EL0 exception stacks; the scheduler still protects its bottom with the
    // paired canary words after initialization.
    .skip {monitor_stack_bytes}
    .global __stack_top
__stack_top:

    // SP0 fatal vectors need valid scratch space. Normal EL1h IRQs and future
    // lower-EL exceptions use SP_EL1, not this stack.
    .section .bss.sp0_stack, "aw", %nobits
    .align 12
    .global __sp0_stack_bottom
__sp0_stack_bottom:
    .skip 16384
    .global __sp0_stack_top
__sp0_stack_top:
