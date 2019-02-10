// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub mod mmu;

use crate::print;

pub mod map {
    pub const DMA_HEAP_SIZE: usize = 0x0020_0000;
    pub const VC_MEM_SIZE: usize = 0x0800_0000;

    pub const MMIO_BASE: usize = 0x3F00_0000;
    pub const MMIO_END: usize = MMIO_BASE + 0x0100_0000;
    pub const MBOX_BASE: usize = MMIO_BASE + 0x0000_b880;
    pub const GPIO_BASE: usize = MMIO_BASE + 0x0020_0000;
    pub const UART_BASE: usize = MMIO_BASE + 0x0020_1000;
    pub const EMMC_BASE: usize = MMIO_BASE + 0x0030_0000;
}

pub fn print_mmap() {
    let kernel_ro_range = ro_range();
    let kernel_rw_range = rw_range();
    let app_range = app_heap_range();
    let dma_range = dma_heap_range();
    let vc_range = vc_range();
    let mmio_range = mmio_range();
    print!("Memory layout:\n");
    print!("0x{:08x} - 0x{:08x} - Kernel code\n", kernel_ro_range.0, kernel_ro_range.1);
    print!("0x{:08x} - 0x{:08x} - Kernel data\n", kernel_rw_range.0, kernel_rw_range.1);
    print!("0x{:08x} - 0x{:08x} - App heap\n", app_range.0, app_range.1);
    print!("0x{:08x} - 0x{:08x} - DMA heap\n", dma_range.0, dma_range.1);
    print!("0x{:08x} - 0x{:08x} - VC memory\n", vc_range.0, vc_range.1);
    print!("0x{:08x} - 0x{:08x} - Device IO\n", mmio_range.0, mmio_range.1);
}

pub fn app_heap_range() -> (usize, usize) {
    let heap_range = heap_range();
    (heap_range.0, heap_range.1 - map::DMA_HEAP_SIZE)
}

pub fn dma_heap_range() -> (usize, usize) {
    let heap_range = heap_range();
    (heap_range.1 - map::DMA_HEAP_SIZE, heap_range.1)
}

pub fn heap_range() -> (usize, usize) {
    extern "C" {
        //noinspection RsStaticConstNaming
        static __heap_start: u64;
    }
    let vc_range = vc_range();
    unsafe {
        (&__heap_start as *const _ as usize, vc_range.0)
    }
}

pub fn mmio_range() -> (usize, usize) {
    (map::MMIO_BASE, map::MMIO_END)
}

#[allow(unused)]
pub fn ro_range() -> (usize, usize) {
    extern "C" {
        //noinspection RsStaticConstNaming
        static __ro_start: u64;
        //noinspection RsStaticConstNaming
        static __ro_end: u64;
    }
    unsafe {
        (&__ro_start as *const _ as usize, &__ro_end as *const _ as usize)
    }
}

#[allow(unused)]
pub fn rw_range() -> (usize, usize) {
    extern "C" {
        //noinspection RsStaticConstNaming
        static __rw_start: u64;
        //noinspection RsStaticConstNaming
        static __rw_end: u64;
    }
    unsafe {
        (&__rw_start as *const _ as usize, &__rw_end as *const _ as usize)
    }
}

pub fn vc_range() -> (usize, usize) {
    (map::MMIO_BASE - map::VC_MEM_SIZE, map::MMIO_BASE)
}

