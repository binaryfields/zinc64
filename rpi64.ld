ENTRY(_boot_cores);

stack_size = 0x80000; /* 512KB */

SECTIONS
{
    . = 0x80000;
    __ro_start = .;
    .text :
    {
        KEEP(*(.text.boot)) *(.text .text.*)
    }

    .vectors ALIGN(2048) :
    {
      *(.vectors)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4096);
    __ro_end = .;

    __rw_start = .;
    .data :
    {
        *(.data .data.*)
    }

    .bss ALIGN(8) :
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }
    __rw_end = .;

    .stack ALIGN(16) :
    {
        __stack_core0_start = .;
        . = . + stack_size;
        __stack_core0 = .;
        . = ALIGN(16);
        __stack_core0_end = .;
    }

    . = ALIGN(0x200000);
    __heap_start = .;

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
