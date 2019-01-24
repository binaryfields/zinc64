// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface

use core::mem;
use core::ops::Deref;
use core::ptr;
use core::result::Result;
use core::sync::atomic::{compiler_fence, Ordering};
use cortex_a::asm;
use register::mmio::{ReadOnly, WriteOnly};
use register::register_bitfields;

use super::MMIO_BASE;

const VIDEOCORE_MBOX: u32 = MMIO_BASE + 0x0000_B880;

#[derive(Copy, Clone)]
pub enum Channel {
    Property = 8,
}

#[derive(Copy, Clone)]
pub enum Code {
    Request = 0x0000_0000,
    ResponseSuccess = 0x8000_0000,
    ResponseFailure= 0x8000_0001,
    Unknown = 0xffff_ffff,
}

impl Code {
    pub fn from(value: u32) -> Code {
        match value {
            0x8000_0000 => Code::ResponseSuccess,
            0x8000_0001 => Code::ResponseFailure,
            _ => Code::Unknown,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Tag {
    GetBoardSerial = 0x00010004,
    GetMaxClockRate = 0x00030004,
    SetClockRate = 0x00038002,
    SetAllocateBuffer = 0x00040001,
    GetPitch = 0x00040008,
    SetPhysicalWidthHeight = 0x00048003,
    SetVirtualWidthHeight = 0x00048004,
    SetDepth = 0x00048005,
    SetPixelOrder = 0x00048006,
    SetVirtualOffset = 0x00048009,
    SetVsync = 0x0004800e,
    PropertyEnd = 0,
}

register_bitfields! { u32,
    Status [
        EMPTY OFFSET(30) NUMBITS(1) [],
        FULL  OFFSET(31) NUMBITS(1) []
    ]
}

#[repr(C)]
pub struct Registers {
    read: ReadOnly<u32>,
    _reserved_0: [u32; 5],
    status: ReadOnly<u32, Status::Register>,
    _reserved_1: u32,
    write: WriteOnly<u32>,
}

#[repr(C)]
#[repr(align(16))]
pub struct Mbox {
    pub buffer: [u32; 36],
    buffer_ptr: u64,
}

impl Mbox {
    pub fn new(buffer_ptr: u64) -> Self {
        Mbox {
            buffer: [0; 36],
            buffer_ptr,
        }
    }

    pub fn call(&mut self, channel: Channel) -> Result<(), &'static str> {
        self.upload_buffer();
        let buf_ptr = self.buffer_ptr as u32;
        assert_eq!(buf_ptr & 0x0f, 0);
        let message = buf_ptr | channel as u32;
        while self.status.is_set(Status::FULL) {
            asm::nop();
        }
        self.write.set(message);
        loop {
            while self.status.is_set(Status::EMPTY) {
                asm::nop();
            }
            if self.read.get() == message {
                self.download_buffer();
                compiler_fence(Ordering::Release);
                return match Code::from(self.buffer[1]) {
                    Code::ResponseSuccess => Ok(()),
                    Code::ResponseFailure => Err("mbox request failed"),
                    _ => Err("unknown response code"),
                };
            }
        }
    }

    pub fn call2(&mut self, channel: Channel) -> Result<(), &'static str> {
        info!("uploading buffer");
        self.upload_buffer();
        let buf_ptr = self.buffer_ptr as u32;
        assert_eq!(buf_ptr & 0x0f, 0);
        let message = buf_ptr | channel as u32;
        info!("waiting until mbox becomes empty");
        while self.status.is_set(Status::FULL) {
            asm::nop();
        }
        info!("writing mbox message");
        self.write.set(message);
        loop {
            info!("waitining until mbox not empty");
            while self.status.is_set(Status::EMPTY) {
                asm::nop();
            }
            info!("reading mbox message");
            if self.read.get() == message {
                info!("downloading buffer");
                self.download_buffer();
                compiler_fence(Ordering::Release);
                info!("checking response 0x{:08x}", self.buffer[1]);
                return match Code::from(self.buffer[1]) {
                    Code::ResponseSuccess => Ok(()),
                    Code::ResponseFailure => Err("mbox request failed"),
                    _ => Err("unknown response code"),
                };
            }
        }
    }

    pub fn property(&mut self, tag: Tag, data: &mut [u32]) -> Result<(), &'static str> {
        let len = data.len();
        assert_eq!(len <= self.buffer.len() - 6, true);
        self.buffer[0] = ((6 + len) * mem::size_of::<u32>()) as u32;
        self.buffer[1] = Code::Request as u32;
        self.buffer[2] = tag as u32;
        self.buffer[3] = len as u32 * mem::size_of::<u32>() as u32;
        self.buffer[4] = len as u32 * mem::size_of::<u32>() as u32;
        for i in 0..data.len() {
            self.buffer[5 + i] = data[i];
        }
        self.buffer[5 + len] = Tag::PropertyEnd as u32;
        compiler_fence(Ordering::Release);
        self.call(Channel::Property)?;
        for i in 0..data.len() {
            data[i] = self.buffer[5 + i];
        }
        Ok(())
    }

    fn base_ptr(&self) -> *mut u32 {
        self.buffer_ptr as *mut u32
    }

    fn download_buffer(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(self.base_ptr(), &mut self.buffer[0], self.buffer.len());
        }
    }

    fn upload_buffer(&self) {
        unsafe {
            ptr::copy_nonoverlapping(&self.buffer[0], self.base_ptr(), self.buffer.len());
        }
    }

    fn ptr() -> *const Registers {
        VIDEOCORE_MBOX as *const _
    }
}

impl Deref for Mbox {
    type Target = Registers;

    fn deref(&self) -> &Self::Target {
        unsafe { &*Self::ptr() }
    }
}
