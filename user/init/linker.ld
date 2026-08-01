OUTPUT_FORMAT("elf64-littleaarch64")
OUTPUT_ARCH(aarch64)
ENTRY(_start)

USER_IMAGE_BASE = 0x0000000200000000;
USER_PAGE_SIZE = 0x1000;
USER_IMAGE_LIMIT = 0x100000;

PHDRS
{
    text PT_LOAD FLAGS(5);       /* PF_R | PF_X */
    rodata PT_LOAD FLAGS(4);     /* PF_R */
    data PT_LOAD FLAGS(6);       /* PF_R | PF_W */
    stack PT_GNU_STACK FLAGS(6); /* PF_R | PF_W, never executable */
}

SECTIONS
{
    . = USER_IMAGE_BASE;

    .text : ALIGN(USER_PAGE_SIZE)
    {
        __user_image_start = .;
        __user_text_start = .;
        KEEP(*(.text._start))
        *(.text .text.*)
        __user_text_end = .;
    } :text

    . = ALIGN(USER_PAGE_SIZE);

    .rodata : ALIGN(USER_PAGE_SIZE)
    {
        __user_rodata_start = .;
        *(.rodata .rodata.*)
        __user_rodata_end = .;
    } :rodata

    . = ALIGN(USER_PAGE_SIZE);

    .data : ALIGN(USER_PAGE_SIZE)
    {
        __user_data_start = .;
        *(.data .data.*)
        __user_data_file_end = .;
    } :data

    .bss (NOLOAD) : ALIGN(8)
    {
        __user_bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __user_bss_end = .;
        __user_image_end = .;
    } :data

    .forbidden_writable :
    {
        *(.got .got.*)
        *(.init_array .init_array.*)
        *(.fini_array .fini_array.*)
        *(.gcc_except_table .gcc_except_table.*)
    } :data

    /DISCARD/ :
    {
        *(.comment)
        *(.note .note.*)
        *(.eh_frame .eh_frame.*)
        *(.ARM.attributes)
        *(.llvm_addrsig)
        *(.debug .debug_*)
        *(.dynamic)
        *(.dynsym .dynstr)
        *(.rela .rela.*)
    }
}

ASSERT(_start == USER_IMAGE_BASE, "EL0 init entry must equal the fixed image base")
ASSERT((__user_image_start % USER_PAGE_SIZE) == 0, "EL0 init image must be page aligned")
ASSERT(__user_image_end > __user_image_start, "EL0 init image must not be empty")
ASSERT(__user_text_end > __user_text_start, "EL0 init RX segment must not be empty")
ASSERT(__user_rodata_end > __user_rodata_start, "EL0 init R-- segment must not be empty")
ASSERT(__user_data_file_end > __user_data_start, "EL0 init RW file data must not be empty")
ASSERT(__user_bss_end > __user_bss_start, "EL0 init RW segment must contain BSS")
ASSERT((__user_image_end - USER_IMAGE_BASE) <= USER_IMAGE_LIMIT,
       "EL0 init image must fit in the lower 1 MiB user image area")
ASSERT(SIZEOF(.forbidden_writable) == 0,
       "EL0 init must not contain writable, GOT, exception, or constructor sections")
ASSERT(!DEFINED(_DYNAMIC), "EL0 init must be a statically linked ET_EXEC")
