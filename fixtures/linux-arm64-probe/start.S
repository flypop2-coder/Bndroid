.section .text.start,"ax",%progbits
.global _start
.type _start,%function
_start:
    mov x0, sp
    bl linux_probe_main
    mov x8, #94
    svc #0
1:  b 1b
.size _start, .-_start
.section .note.GNU-stack,"",%progbits
