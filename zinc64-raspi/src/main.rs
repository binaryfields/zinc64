// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![feature(alloc)]
#![feature(alloc_error_handler)]
#![no_main]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

mod app;
mod logger;

use core::alloc::Layout;
use core::panic::PanicInfo;
use cortex_a_semihosting::{debug, hprintln};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

fn main() -> ! {
    unsafe {
        ALLOCATOR.lock().init(65536, 50 * 4096);
    }
    logger::init().unwrap();
    app::run();
    logger::shutdown().unwrap();
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    hprintln!("ERROR: Out of memory!").unwrap();
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //interrupt::disable();
    hprintln!("{}", info).unwrap();
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}

raspi3_boot::entry!(main);
