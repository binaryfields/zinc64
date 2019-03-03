// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(label_break_value)]
#![feature(range_contains)]

extern crate alloc;
#[macro_use]
extern crate log;

mod app;
mod audio;
mod debug;
mod device;
mod exception;
mod macros;
mod memory;
mod null_output;
mod palette;
mod sound_buffer;
mod video_buffer;
mod video_renderer;
mod util;

use core::alloc::Layout;
use core::panic::PanicInfo;
use cortex_a::asm;
use linked_list_allocator::LockedHeap;

use crate::device::console::{Console, Output};
use crate::device::interrupt::{InterruptControl, Irq};
use crate::util::sync::NullLock;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static DMA_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut CONSOLE: Console = Console::new();
static IRQ_CONTROL: NullLock<InterruptControl> = NullLock::new(
    InterruptControl::new()
);

fn start() -> ! {
    extern "C" {
        //noinspection RsStaticConstNaming
        static __exception_vectors_start: u64;
    }
    print!("Initializing console ...\n");
    {
        let dma_range = memory::dma_heap_range();
        let gpio = device::gpio::GPIO::new(memory::map::GPIO_BASE);
        let mut mbox = device::mbox::Mbox::new_with_buffer(memory::map::MBOX_BASE, dma_range.0, 64);
        let uart = device::uart::Uart::new(memory::map::UART_BASE);
        uart.init(&mut mbox, &gpio).unwrap();
        unsafe {
            CONSOLE.set_output(Output::Uart(uart));
        }
    }

    print!("Installing exception handlers ...\n");
    unsafe {
        use cortex_a::{barrier, regs::*};
        let vectors = &__exception_vectors_start as *const _ as u64;
        VBAR_EL1.set(vectors);
        barrier::isb(barrier::SY);
    }

    print!("Initializing MMU ...\n");
    unsafe {
        memory::mmu::init_page_table();
        memory::mmu::init();
    }

    print!("Initializing heap ...\n");
    unsafe {
        let heap_range = memory::app_heap_range();
        let dma_range = memory::dma_heap_range();
        ALLOCATOR.lock().init(heap_range.0, heap_range.1 - heap_range.0);
        DMA_ALLOCATOR.lock().init(dma_range.0, dma_range.1 - dma_range.0);
    }

    memory::print_mmap();

    match main() {
        Ok(_) => (),
        Err(err) => print!("ERROR: {}\n", err),
    };

    loop {
        asm::wfe()
    }
}

fn main() -> Result<(), &'static str> {
    let gpio = device::gpio::GPIO::new(memory::map::GPIO_BASE);
    let mut mbox = device::mbox::Mbox::build(memory::map::MBOX_BASE)?;

    print!("Initializing logger ...\n");
    let logger = util::logger::SimpleLogger::new();
    logger.init().map_err(|_| "failed to initialize log")?;
    let max_clock = device::board::get_max_clock_rate(&mut mbox, device::board::Clock::Arm)?;

    print!("Setting ARM clock speed to {}\n", max_clock);
    device::board::set_clock_speed(&mut mbox, device::board::Clock::Arm, max_clock)?;

    print!("Initializing interrupts ...\n");
    IRQ_CONTROL.lock(|ctl| {
        ctl.init();
    });

    print!("Initializing SD ...\n");
    let mut sd = device::sd::Sd::new(memory::map::EMMC_BASE);
    sd.init(&gpio).map_err(|_| "failed to initialize sdcard")?;

    print!("Initializing FAT32 ...\n");
    let fat32 = device::fat32::Fat32::mount(sd, 0)?;
    fat32.info();

    print!("Starting app ...\n");
    let mut app = app::App::build(&gpio, &fat32)?;
    crate::IRQ_CONTROL.lock(|ctl| {
        ctl.register(Irq::Dma0, app.audio_engine.make_irq_handler());
        ctl.enable(Irq::Dma0);
    });
    app.autostart(&fat32)?;
    app.run()?;

    util::logger::shutdown().map_err(|_| "failed to shutdown log")
}

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    print!("ERROR: Out of memory!\n");
    loop {
        asm::wfe()
    }
}

#[panic_handler]
fn on_panic(info: &PanicInfo) -> ! {
    //interrupt::disable();
    print!("ERROR: {}\n", info);
    loop {
        asm::wfe()
    }
}

raspi3_boot::entry!(start);
