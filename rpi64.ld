ENTRY(_boot_cores);

MEMORY
{
    RAM : ORIGIN = 0x80000, LENGTH = 2M
}

heap_size = 0x100000; /* 1MB */
stack_size = 0x80000; /* 512KB */

SECTIONS
{
    . = ORIGIN(RAM);
    __ro_start = .;
    .text :
    {
        KEEP(*(.text.boot)) *(.text .text.*)
    } >RAM

    .rodata :
    {
        *(.rodata .rodata.*)
    } >RAM
    . = ALIGN(4096); /* Fill up to 4KiB */
    __ro_end = .;

    .data :
    {
        *(.data .data.*)
    } >RAM

    .bss ALIGN(8) :
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } >RAM
    . = ALIGN(4096); /* Fill up to 4KiB */

    .heap ALIGN(16) :
    {
        __heap_start = .;
        . = . + heap_size;
        . = ALIGN(16);
        __heap_end = .;
    } >RAM

    .stack ALIGN(16) :
    {
        __stack_core0_start = .;
        . = . + stack_size;
        __stack_core0 = .;
        . = ALIGN(16);
        __stack_core0_end = .;
    } >RAM

    /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}
