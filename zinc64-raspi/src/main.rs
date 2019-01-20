// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(core_intrinsics)]
#![no_main]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

mod app;
mod debug;
mod geo;
mod hal;
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

fn start() -> ! {
    extern "C" {
        static mut __heap_start: u64;
        static mut __heap_end: u64;
    }
    let mut mbox = hal::mbox::Mbox::new();
    let gpio = hal::gpio::GPIO::new();
    let uart = hal::uart::Uart::new();
    match uart.init(&mut mbox, &gpio) {
        Ok(_) => uart.puts("\n[0] UART is live!\n"),
        Err(_) => loop {
            asm::wfe()
        },
    }
    uart.puts("[0] Initializing MMU ...\n");
    unsafe {
        hal::mmu::init_page_table();
        hal::mmu::init();
    }
    uart.puts("[0] Running POST ...\n");
    post(&mut mbox, &uart);
    uart.puts("[0] Initializing heap ...\n");
    unsafe {
        ALLOCATOR.lock().init(
            &__heap_start as *const _ as usize,
            &__heap_end as *const _ as usize - &__heap_start as *const _ as usize);
    }
    uart.puts("[0] Initializing logger ...\n");
    let logger = logger::SimpleLogger::new(uart);
    logger.init().unwrap();
    unsafe {
        info!("Initialized heap at 0x{:08x} size 0x{:08x}",
              &__heap_start as *const _ as u64,
              &__heap_end as *const _ as usize - &__heap_start as *const _ as usize);
    }
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

fn post(mbox: &mut hal::mbox::Mbox, uart: &hal::uart::Uart) {
    uart.puts("[0] Getting board serial ...\n");
    match hal::board::get_serial(mbox)  {
        Ok(serial) => {
            uart.puts("[0] Board serial ");
            uart.hex(serial);
            uart.puts("\n");
        },
        Err(err) => loop {
            uart.puts(err);
            uart.puts("\n");
            asm::wfe()
        },
    }
    uart.puts("[0] Testing atomic ops ...\n");
    let result =  unsafe {
        let mut lock: u32 = 0;
        let result = core::intrinsics::atomic_cxchg_acq(&mut lock as *mut u32, 0, 1);
        result.1
    };
    if result {
        uart.puts("[0] Atomic op returned true\n");
    } else {
        uart.puts("[0] Atomic op returned false\n");
    }
}

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    error!("Out of memory!");
    loop {
        asm::wfe()
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //interrupt::disable();
    error!("{}", info);
    loop {
        asm::wfe()
    }
}

raspi3_boot::entry!(start);
