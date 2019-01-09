// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// https://docs.rust-embedded.org/book/collections/index.html
// https://github.com/rust-embedded/cortex-m-rt/blob/master/examples/qemu.rs

#![feature(alloc)]
#![feature(alloc_error_handler)]
#![no_main]
#![no_std]

extern crate alloc;
extern crate panic_halt;

mod app;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use cortex_m_rt ::entry;
use cortex_m_semihosting::{debug, hprintln};

const HEAP_SIZE: usize = 32 * 1024;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[entry]
fn main() -> ! {
    // Initialize the allocator BEFORE you use it
    unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

    hprintln!("Starting emulator ...").unwrap();
    //app::run();
    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}

#[alloc_error_handler]
fn on_oom(_layout: Layout) -> ! {
    //asm::bkpt();
    hprintln!("ERROR: Out of memory!").unwrap();
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
