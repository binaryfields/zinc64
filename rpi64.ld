ENTRY(_boot_cores);

heap_size = 0x100000; /* 1MB */
stack_size = 0x80000; /* 512KB */

MEMORY
{
    RAM : ORIGIN = 0x80000, LENGTH = 2M
}

SECTIONS
{
    . = ORIGIN(RAM);

    __ro_start = .;
    .text :
    {
        KEEP(*(.text.boot)) *(.text .text.*)
    } >RAM

    .vectors ALIGN(2048) :
    {
      *(.vectors)
    } >RAM

    .rodata :
    {
        *(.rodata .rodata.*)
    } >RAM
    . = ALIGN(4096); /* Fill up to 4KiB */
    __ro_end = .;

    __rw_start = .;
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

    .heap ALIGN(4096) :
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
    . = ALIGN(4096);
    __rw_end = .;

    /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) *(.eh_frame*) }
}

PROVIDE(current_el0_synchronous   = default_exception_handler);
PROVIDE(current_el0_irq           = default_exception_handler);
PROVIDE(current_el0_serror        = default_exception_handler);
PROVIDE(current_elx_synchronous   = default_exception_handler);
PROVIDE(current_elx_irq           = default_exception_handler);
PROVIDE(current_elx_serror        = default_exception_handler);
PROVIDE(lower_aarch64_synchronous = default_exception_handler);
PROVIDE(lower_aarch64_irq         = default_exception_handler);
PROVIDE(lower_aarch64_serror      = default_exception_handler);
PROVIDE(lower_aarch32_synchronous = default_exception_handler);
PROVIDE(lower_aarch32_irq         = default_exception_handler);
PROVIDE(lower_aarch32_serror      = default_exception_handler);
