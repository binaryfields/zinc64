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

extern crate alloc;
#[macro_use]
extern crate log;

mod app;
mod debug;
mod exception;
mod geo;
mod hal;
mod reader;
mod logger;
mod null_output;
mod palette;
mod video_buffer;

use core::alloc::Layout;
use core::panic::PanicInfo;
use cortex_a::asm;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static UART: hal::uart::Uart = hal::uart::Uart::new();

fn start() -> ! {
    extern "C" {
        static __exception_vectors_start: u64;
        static __heap_start: u64;
        static __heap_end: u64;
    }
    let mut mbox = hal::mbox::Mbox::new(878 * 1024 * 1024);
    let gpio = hal::gpio::GPIO::new();

    match UART.init(&mut mbox, &gpio) {
        Ok(_) => UART.puts("\nUART is live!\n"),
        Err(_) => loop {
            asm::wfe()
        },
    }

    UART.puts("Installing exception handlers ...\n");
    unsafe {
        use cortex_a::{barrier, regs::*};
        let vectors = &__exception_vectors_start as *const _ as u64;
        VBAR_EL1.set(vectors);
        barrier::isb(barrier::SY);
    }

    UART.puts("Initializing MMU ...\n");
    unsafe {
        hal::mmu::init_page_table();
        hal::mmu::init();
    }

    UART.puts("Running POST ...\n");
    run_post(&mut mbox);

    UART.puts("Initializing heap ...\n");
    unsafe {
        let heap_start = &__heap_start as *const _ as usize;
        let heap_size = &__heap_end as *const _ as usize - &__heap_start as *const _ as usize;
        ALLOCATOR.lock().init(heap_start, heap_size);
        UART.puts("Initialized heap at 0x");
        UART.hex(heap_start as u64);
        UART.puts(" size 0x");
        UART.hex(heap_size as u64);
        UART.puts("\n");
    }

    UART.puts("Initializing logger ...\n");
    let logger = logger::SimpleLogger::new(&UART);
    logger.init().unwrap();

    match main(mbox) {
        Ok(_) => (),
        Err(err) => error!("{}", err),
    };

    logger::shutdown().unwrap();
    loop {
        asm::wfe()
    }
}

fn main(mut mbox: hal::mbox::Mbox) -> Result<(), &'static str> {
    let max_clock = hal::board::get_max_clock_rate(&mut mbox, hal::board::Clock::Arm)?;
    info!("Setting ARM clock speed to {}", max_clock);
    hal::board::set_clock_speed(&mut mbox, hal::board::Clock::Arm, max_clock)?;
    info!("Starting app ...");
    let mut app = app::App::build(mbox)?;
    app.run()
}

fn run_post(mbox: &mut hal::mbox::Mbox) {
    UART.puts("Getting board serial ...\n");
    match hal::board::get_serial(mbox)  {
        Ok(serial) => {
            UART.puts("Board serial ");
            UART.hex(serial);
            UART.puts("\n");
        },
        Err(err) => loop {
            UART.puts(err);
            UART.puts("\n");
            asm::wfe()
        },
    }
    UART.puts("Testing atomic ops ...\n");
    let result =  unsafe {
        let mut lock: u32 = 0;
        let result = core::intrinsics::atomic_cxchg_acq(&mut lock as *mut u32, 0, 1);
        result.1
    };
    if result {
        UART.puts("Atomic op returned true\n");
    } else {
        UART.puts("Atomic op returned false\n");
    }
}

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    UART.puts("Out of memory!\n");
    loop {
        asm::wfe()
    }
}

#[panic_handler]
fn on_panic(info: &PanicInfo) -> ! {
    //interrupt::disable();
    error!("{}", info);
    loop {
        asm::wfe()
    }
}

raspi3_boot::entry!(start);
