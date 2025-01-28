// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ops::Deref;
use core::result::Result;
use core::slice;
use cortex_a::asm;
use register::{
    mmio::{ReadOnly, ReadWrite},
    register_bitfields,
};

use crate::memory;

// SPEC: BCM2837 ARM Peripherals v2.1 - 4 DMA Controller

pub struct Buffer<'a, T> {
    pub buf: &'a mut [T],
}

impl<'a, T> Buffer<'a, T> {
    pub fn alloc(len: usize) -> Result<Buffer<'a, T>, &'static str> {
        let size = len * mem::size_of::<T>();
        let layout = Layout::from_size_align(size, 8).map_err(|_| "invalid buffer alignment")?;
        unsafe {
            let ptr = crate::DMA_ALLOCATOR.alloc_zeroed(layout);
            if ptr.is_null() {
                return Err("failed to allocate buffer");
            }
            Ok(Buffer {
                buf: slice::from_raw_parts_mut(ptr as *mut T, len),
            })
        }
    }
}

impl<'a, T> Drop for Buffer<'a, T> {
    fn drop(&mut self) {
        let size = self.buf.len() * mem::size_of::<T>();
        let layout = Layout::from_size_align(size, 8).unwrap();
        unsafe {
            crate::DMA_ALLOCATOR.dealloc(self.buf.as_mut_ptr() as *mut u8, layout);
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct ControlBlock {
    pub transfer_information: u32,
    pub source_address: u32,
    pub destination_address: u32,
    pub transfer_length: u32,
    pub stride: u32,
    pub next_control_block: u32,
    pub reserved: [u32; 2],
}

impl ControlBlock {
    pub fn init(&mut self, ti: u32, src: u32, dst: u32, len: u32, stride: u32) {
        self.transfer_information = ti;
        self.source_address = src;
        self.destination_address = dst;
        self.transfer_length = len;
        self.stride = stride;
        self.next_control_block = 0;
        self.reserved[0] = 0;
        self.reserved[1] = 0;
    }
}

pub struct ControlBlockWrapper<'a> {
    pub cb: &'a mut ControlBlock,
}

impl<'a> ControlBlockWrapper<'a> {
    pub fn alloc() -> Result<ControlBlockWrapper<'a>, &'static str> {
        let size = mem::size_of::<ControlBlock>();
        let layout = Layout::from_size_align(size, 32).map_err(|_| "invalid buffer alignment")?;
        unsafe {
            let ptr = crate::DMA_ALLOCATOR.alloc_zeroed(layout);
            if ptr.is_null() {
                return Err("failed to allocate buffer");
            }
            let cb_ptr = mem::transmute::<*mut u8, *mut ControlBlock>(ptr);
            Ok(ControlBlockWrapper { cb: &mut *cb_ptr })
        }
    }

    pub fn dump(&self, id: &str) {
        info!("{} @ 0x{:08x}", id, self.ptr() as usize);
        info!(
            "ti=0x{:08x} src=0x{:08x} dst=0x{:08x} len=0x{:x} str=0x{:x} next=0x{:08x}",
            self.cb.transfer_information,
            self.cb.source_address,
            self.cb.destination_address,
            self.cb.transfer_length,
            self.cb.stride,
            self.cb.next_control_block,
        );
    }

    pub fn init(&mut self, ti: u32, src: u32, dst: u32, len: u32, stride: u32) {
        self.cb.init(ti, src, dst, len, stride);
    }

    pub fn ptr(&self) -> *const ControlBlock {
        &*self.cb as *const _
    }

    pub fn set_next(&mut self, cb_ptr: *const ControlBlock) {
        self.cb.next_control_block = memory::bus_address(cb_ptr as usize);
    }
}

impl<'a> Drop for ControlBlockWrapper<'a> {
    fn drop(&mut self) {
        let size = mem::size_of::<ControlBlock>();
        let layout = Layout::from_size_align(size, 32).unwrap();
        unsafe {
            crate::DMA_ALLOCATOR.dealloc(self.cb as *mut ControlBlock as *mut u8, layout);
        }
    }
}

#[allow(unused)]
pub enum DreqPeripheralMap {
    Dsi = 1,
    PcmTx = 2,
    PcmRx = 3,
    Smi = 4,
    Pwm = 5,
    SpiTx = 6,
    SpiRx = 7,
    SpiSlaveTx = 8,
    SpiSlaveRx = 9,
    Emmc = 11,
    UartTx = 12,
    SdHost = 13,
    UartRx = 14,
    Dsi1 = 15,
    SlimBus = 16,
    Hdmi = 17,
}

register_bitfields! { u32,
    CS [
        ACTIVE OFFSET(0) NUMBITS(1) [],
        END OFFSET(1) NUMBITS(1) [],
        INT OFFSET(2) NUMBITS(1) [],
        DREQ OFFSET(3) NUMBITS(1) [],
        PAUSED OFFSET(4) NUMBITS(1) [],
        DREQ_STOPS_DMA OFFSET(5) NUMBITS(1) [],
        WAITING_FOR_OUTSTANDING_WRITES OFFSET(6) NUMBITS(1) [],
        ERROR OFFSET(8) NUMBITS(1) [],
        PRIORITY OFFSET(16) NUMBITS(4) [],
        PANIC_PRIORITY OFFSET(20) NUMBITS(4) [],
        WAIT_FOR_OUTSTANDING_WRITES OFFSET(28) NUMBITS(1) [],
        DISDEBUG OFFSET(29) NUMBITS(1) [],
        ABORT OFFSET(30) NUMBITS(1) [],
        RESET OFFSET(31) NUMBITS(1) []
    ],
    TI [
        INTEN OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        TDMODE OFFSET(1) NUMBITS(1) [
            Linear = 0,
            TwoD = 1
        ],
        WAIT_RESP OFFSET(3) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        DEST_INC OFFSET(4) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        DEST_WIDTH OFFSET(5) NUMBITS(1) [
            Use32Bit = 0,
            Use128Bit= 1
        ],
        DEST_DREQ OFFSET(6) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        DEST_IGNORE OFFSET(7) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        SRC_INC OFFSET(8) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        SRC_WIDTH OFFSET(9) NUMBITS(1) [
            Use32Bit = 0,
            Use128Bit= 1
        ],
        SRC_DREQ OFFSET(10) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        SRC_IGNORE OFFSET(11) NUMBITS(1) [
            False = 0,
            True = 1
        ],
        BURST_LENGTH OFFSET(12) NUMBITS(4) [],
        PERMAP OFFSET(16) NUMBITS(5) [],
        WAITS OFFSET(21) NUMBITS(5) [],
        NO_WIDE_BURSTS OFFSET(26) NUMBITS(1) []
    ],
    TXFR_LEN [
        XLENGTH OFFSET(0) NUMBITS(16) [],
        YLENGTH OFFSET(16) NUMBITS(14) []
    ],
    STRIDE [
        S_STRIDE OFFSET(0) NUMBITS(16) [],
        D_STRIDE OFFSET(16) NUMBITS(16) []
    ]
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct DmaRegisterBlock {
    int_status: ReadWrite<u32>,
    _reserved: [u32; 3],
    enable: ReadWrite<u32>,
}

pub struct Dma;

#[allow(unused)]
impl Dma {
    pub fn disable(&self, channel: &DmaChannel) {
        self.enable
            .set(self.enable.get() & !(1 << channel.get_instance()));
    }

    pub fn enable(&self, channel: &DmaChannel) {
        self.enable
            .set(self.enable.get() | (1 << channel.get_instance()));
    }

    pub fn is_interrupt(&self, channel: &DmaChannel) -> bool {
        DMA.int_status.get() & (1 << channel.get_instance()) != 0
    }

    fn ptr(&self) -> *const DmaRegisterBlock {
        memory::map::DMA_REG_BASE as *const _
    }
}

impl Deref for Dma {
    type Target = DmaRegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

pub static DMA: Dma = Dma {};

#[allow(non_snake_case)]
#[repr(C)]
pub struct DmaChannelRegisterBlock {
    cs: ReadWrite<u32, CS::Register>,
    conblk_ad: ReadWrite<u32>,
    ti: ReadOnly<u32, TI::Register>,
    source_ad: ReadOnly<u32>,
    dest_ad: ReadOnly<u32>,
    txfr_len: ReadOnly<u32, TXFR_LEN::Register>,
    stride: ReadOnly<u32, STRIDE::Register>,
    nextconbk: ReadOnly<u32>,
    debug: ReadWrite<u32>,
}

pub struct DmaChannel {
    base_addr: usize,
    channel: usize,
}

// TODO dmaman: alloc_channel, free_channel

#[allow(unused)]
impl DmaChannel {
    pub fn new(base_addr: usize, channel: usize) -> Self {
        assert!(channel < 15);
        DmaChannel {
            base_addr: base_addr + channel * 0x100,
            channel,
        }
    }

    pub fn dump(&self) {
        info!(
            "DMA {} @ cs=0x{:08x} actv={} end={} int={} err={} paus={}",
            self.channel,
            self.cs.get(),
            self.is_busy(),
            self.is_complete(),
            self.is_interrupt(),
            self.is_error(),
            self.is_paused()
        );
        info!(
            "ti=0x{:08x} src=0x{:08x} dst=0x{:08x} len=0x{:x} str=0x{:x} next=0x{:08x}",
            self.ti.get(),
            self.source_ad.get(),
            self.dest_ad.get(),
            self.txfr_len.get(),
            self.stride.get(),
            self.nextconbk.get(),
        );
    }

    pub fn get_instance(&self) -> u32 {
        self.channel as u32
    }

    pub fn is_busy(&self) -> bool {
        self.cs.is_set(CS::ACTIVE)
    }

    pub fn is_complete(&self) -> bool {
        self.cs.is_set(CS::END)
    }

    pub fn is_error(&self) -> bool {
        self.cs.is_set(CS::ERROR)
    }

    pub fn is_interrupt(&self) -> bool {
        self.cs.is_set(CS::INT)
    }

    pub fn is_paused(&self) -> bool {
        self.cs.is_set(CS::PAUSED)
    }

    pub fn clear_interrupt(&self) {
        self.cs.write(CS::INT::SET);
    }

    #[allow(unused)]
    pub fn pause(&self) {
        if self.cs.is_set(CS::ACTIVE) {
            self.cs.write(CS::ACTIVE::CLEAR);
        }
    }

    pub fn reset(&self) {
        self.cs.write(CS::RESET::SET);
        while self.cs.is_set(CS::RESET) {
            asm::nop();
        }
    }

    #[allow(unused)]
    pub fn resume(&self) {
        if !self.cs.is_set(CS::ACTIVE) {
            self.cs.write(CS::ACTIVE::SET);
        }
    }

    pub fn start(&self, control_block: &ControlBlockWrapper) {
        assert!(!self.cs.is_set(CS::INT));
        self.conblk_ad
            .set(memory::bus_address(control_block.ptr() as usize));
        self.cs.write(
            CS::ACTIVE::SET, //+ CS::PRIORITY.val(1)
                             //+ CS::PANIC_PRIORITY.val(15)
                             //+ CS::WAIT_FOR_OUTSTANDING_WRITES::SET
        );
    }

    pub fn stop(&self) {
        self.conblk_ad.set(0);
    }

    fn ptr(&self) -> *const DmaChannelRegisterBlock {
        self.base_addr as *const _
    }
}

impl Deref for DmaChannel {
    type Target = DmaChannelRegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}
