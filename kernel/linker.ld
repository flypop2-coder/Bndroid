ENTRY(_start)

SECTIONS
{
    . = 0x40080000;
    __kernel_start = .;

    .text : ALIGN(4K) {
        __text_start = .;
        KEEP(*(.text.boot))
        KEEP(*(.text.exceptions))
        *(.text .text.*)
        __text_end = .;
    }

    .rodata : ALIGN(4K) {
        __rodata_start = .;
        *(.rodata .rodata.*)
        . = ALIGN(8);
        __usercopy_ex_table_start = .;
        KEEP(*(.usercopy_ex_table))
        __usercopy_ex_table_end = .;
        __rodata_end = .;
    }

    .data : ALIGN(4K) {
        __data_start = .;
        *(.data .data.*)
        __data_end = .;
    }

    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4K);
        __bss_end = .;
    }

    . = ALIGN(4K);
    __kernel_end = .;
}

ASSERT((__bss_start % 8) == 0, "BSS start must be 8-byte aligned")
ASSERT((__bss_end % 8) == 0, "BSS end must be 8-byte aligned")
ASSERT((__kernel_start % 4096) == 0, "kernel start must be page aligned")
ASSERT((__text_start % 4096) == 0, "text start must be page aligned")
ASSERT((__rodata_start % 4096) == 0, "rodata start must be page aligned")
ASSERT((__data_start % 4096) == 0, "data start must be page aligned")
ASSERT((__bss_start % 4096) == 0, "BSS start must be page aligned")
ASSERT((__bss_end % 4096) == 0, "BSS end must be page aligned")
ASSERT((__kernel_end % 4096) == 0, "kernel end must be page aligned")
ASSERT((__usercopy_ex_table_start % 8) == 0, "user-copy exception table must be aligned")
ASSERT((__usercopy_ex_table_end - __usercopy_ex_table_start) == 80, "user-copy exception table must contain exactly two entries")
ASSERT(__bss_start <= 0x40600000, "executable or read-only kernel sections escaped the 6 MiB W^X page window")
ASSERT(__kernel_end <= __kernel_bss_limit, "writable kernel BSS escaped its profile-bounded non-executable RAM window")
