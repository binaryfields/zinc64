// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::ops::Deref;
use register::{mmio::ReadWrite, register_bitfields};

#[allow(unused)]
pub enum GPFSEL {
    In = 0b000,
    Out = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

register_bitfields! {
    u32,
    GPFSEL0 [
        FSEL0 OFFSET(0) NUMBITS(3) [],
        FSEL1 OFFSET(3) NUMBITS(3) [],
        FSEL2 OFFSET(6) NUMBITS(3) [],
        FSEL3 OFFSET(9) NUMBITS(3) [],
        FSEL4 OFFSET(12) NUMBITS(3) [],
        FSEL5 OFFSET(15) NUMBITS(3) [],
        FSEL6 OFFSET(18) NUMBITS(3) [],
        FSEL7 OFFSET(21) NUMBITS(3) [],
        FSEL8 OFFSET(24) NUMBITS(3) [],
        FSEL9 OFFSET(27) NUMBITS(3) []
    ],
    GPFSEL1 [
        FSEL10 OFFSET(0) NUMBITS(3) [],
        FSEL11 OFFSET(3) NUMBITS(3) [],
        FSEL12 OFFSET(6) NUMBITS(3) [],
        FSEL13 OFFSET(9) NUMBITS(3) [],
        FSEL14 OFFSET(12) NUMBITS(3) [],
        FSEL15 OFFSET(15) NUMBITS(3) [],
        FSEL16 OFFSET(18) NUMBITS(3) [],
        FSEL17 OFFSET(21) NUMBITS(3) [],
        FSEL18 OFFSET(24) NUMBITS(3) [],
        FSEL19 OFFSET(27) NUMBITS(3) []
    ],
    GPFSEL2 [
        FSEL20 OFFSET(0) NUMBITS(3) [],
        FSEL21 OFFSET(3) NUMBITS(3) [],
        FSEL22 OFFSET(6) NUMBITS(3) [],
        FSEL23 OFFSET(9) NUMBITS(3) [],
        FSEL24 OFFSET(12) NUMBITS(3) [],
        FSEL25 OFFSET(15) NUMBITS(3) [],
        FSEL26 OFFSET(18) NUMBITS(3) [],
        FSEL27 OFFSET(21) NUMBITS(3) [],
        FSEL28 OFFSET(24) NUMBITS(3) [],
        FSEL29 OFFSET(27) NUMBITS(3) []
    ],
    GPFSEL3 [
        FSEL30 OFFSET(0) NUMBITS(3) [],
        FSEL31 OFFSET(3) NUMBITS(3) [],
        FSEL32 OFFSET(6) NUMBITS(3) [],
        FSEL33 OFFSET(9) NUMBITS(3) [],
        FSEL34 OFFSET(12) NUMBITS(3) [],
        FSEL35 OFFSET(15) NUMBITS(3) [],
        FSEL36 OFFSET(18) NUMBITS(3) [],
        FSEL37 OFFSET(21) NUMBITS(3) [],
        FSEL38 OFFSET(24) NUMBITS(3) [],
        FSEL39 OFFSET(27) NUMBITS(3) []
    ],
    GPFSEL4 [
        FSEL40 OFFSET(0) NUMBITS(3) [],
        FSEL41 OFFSET(3) NUMBITS(3) [],
        FSEL42 OFFSET(6) NUMBITS(3) [],
        FSEL43 OFFSET(9) NUMBITS(3) [],
        FSEL44 OFFSET(12) NUMBITS(3) [],
        FSEL45 OFFSET(15) NUMBITS(3) [],
        FSEL46 OFFSET(18) NUMBITS(3) [],
        FSEL47 OFFSET(21) NUMBITS(3) [],
        FSEL48 OFFSET(24) NUMBITS(3) [],
        FSEL49 OFFSET(27) NUMBITS(3) []
    ],
    GPFSEL5 [
        FSEL50 OFFSET(0) NUMBITS(3) [],
        FSEL51 OFFSET(3) NUMBITS(3) [],
        FSEL52 OFFSET(6) NUMBITS(3) [],
        FSEL53 OFFSET(9) NUMBITS(3) []
    ],
    GPPUD [
        PUD OFFSET(0) NUMBITS(2) [
            Off = 0b00,
            PullDown = 0b01,
            PullUp = 0b10,
            Reserved = 0b11
        ]
    ],
    GPREGSET0 [
        P0 0,
        P1 1,
        P2 2,
        P3 3,
        P4 4,
        P5 5,
        P6 6,
        P7 7,
        P8 8,
        P9 9,
        P10 10,
        P11 11,
        P12 12,
        P13 13,
        P14 14,
        P15 15,
        P16 16,
        P17 17,
        P18 18,
        P19 19,
        P20 20,
        P21 21,
        P22 22,
        P23 23,
        P24 24,
        P25 25,
        P26 26,
        P27 27,
        P28 28,
        P29 29,
        P30 30,
        P31 31
    ],
    GPREGSET1 [
        P32 0,
        P33 1,
        P34 2,
        P35 3,
        P36 4,
        P37 5,
        P38 6,
        P39 7,
        P40 8,
        P41 9,
        P42 10,
        P43 11,
        P44 12,
        P45 13,
        P46 14,
        P47 15,
        P48 16,
        P49 17,
        P50 18,
        P51 19,
        P52 20,
        P53 21
    ]
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct RegisterBlock {
    pub GPFSEL0: ReadWrite<u32, GPFSEL0::Register>,
    pub GPFSEL1: ReadWrite<u32, GPFSEL1::Register>,
    pub GPFSEL2: ReadWrite<u32, GPFSEL2::Register>,
    pub GPFSEL3: ReadWrite<u32, GPFSEL3::Register>,
    pub GPFSEL4: ReadWrite<u32, GPFSEL4::Register>,
    pub GPFSEL5: ReadWrite<u32, GPFSEL5::Register>,
    __reserved_0: u32,
    pub GPSET0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPSET1: ReadWrite<u32, GPREGSET1::Register>,
    __reserved_1: u32,
    pub GPCLR0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPCLR1: ReadWrite<u32, GPREGSET1::Register>,
    __reserved_2: u32,
    pub GPLEV0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPLEV1: ReadWrite<u32, GPREGSET1::Register>,
    __reserved_3: u32,
    pub GPEDS0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPEDS1: ReadWrite<u32, GPREGSET1::Register>,
    __reserved_4: [u32; 7],
    pub GPHEN0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPHEN1: ReadWrite<u32, GPREGSET1::Register>,
    __reserved_5: [u32; 10],
    pub GPPUD: ReadWrite<u32, GPPUD::Register>,
    pub GPPUDCLK0: ReadWrite<u32, GPREGSET0::Register>,
    pub GPPUDCLK1: ReadWrite<u32, GPREGSET1::Register>,
}

pub struct GPIO {
    base_addr: usize
}

impl GPIO {
    pub fn new(base_addr: usize) -> Self {
        GPIO {
            base_addr
        }
    }

    fn ptr(&self) -> *const RegisterBlock {
        self.base_addr as *const _
    }
}

impl Deref for GPIO {
    type Target = RegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}
