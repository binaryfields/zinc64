// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::ops::Deref;
use cortex_a::asm;
use register::{mmio::ReadWrite, register_bitfields};

// SPEC: BCM2837 ARM Peripherals v2.1 - 6.3 General Purpose GPIO Clocks

pub const MIN_DIVIDER: u32 = 2;

const CLOCK_PASSWORD: u32 = 0x5a;
const KHZ_MULTIPLIER: u32 = 1_000;
const MHZ_MULTIPLIER: u32 = 1_000_000;

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum ClockInstance {
    Generic = 0,
    Vpu,
    Sys,
    Peria,
    Perii,
    H264,
    Isp,
    V3d,
    Camera0,
    Camera1,
    Ccp2,
    Dsi0e,
    Dsi0p,
    Dpi,
    Gp0,
    Gp1,
    Gp2,
    Hsm,
    Otp,
    Pcm,
    Pwm,
    Slim,
    Smi,
    Tcnt,
    Tec,
    Td0,
    Td1,
    Tsens,
    Timer,
    Uart,
    Vec,
}

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum ClockSource {
    // 19.2 MHz
    Oscillator = 1,
    TestDebug0 = 2,
    TestDebug1 = 3,
    PllA = 4,
    // 1000 MHz
    PllC = 5,
    // 500 MHz
    PllD = 6,
    // 216 MHz
    HdmiAux = 7,
    Gnd = 8,
}

impl ClockSource {
    pub fn frequency(&self) -> u32 {
        match self {
            ClockSource::Oscillator => 19200 * KHZ_MULTIPLIER,
            ClockSource::PllC => 1000 * MHZ_MULTIPLIER,
            ClockSource::PllD => 500 * MHZ_MULTIPLIER,
            ClockSource::HdmiAux => 216 * MHZ_MULTIPLIER,
            _ => 0,
        }
    }
}

register_bitfields! {
    u32,
    Control [
        SRC OFFSET(0) NUMBITS(4) [],
        ENAB OFFSET(4) NUMBITS(1) [],
        KILL OFFSET(5) NUMBITS(1) [],
        BUSY OFFSET(7) NUMBITS(1) [],
        FLIP OFFSET(8) NUMBITS(1) [],
        MASH OFFSET(9) NUMBITS(2) [],
        PASSWD OFFSET(24) NUMBITS(8) []
    ],
    Divisor [
        DIVF OFFSET(0) NUMBITS(12) [],
        DIVI OFFSET(12) NUMBITS(12) [],
        PASSWD OFFSET(24) NUMBITS(8) []
    ]
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct RegisterBlock {
    control: ReadWrite<u32, Control::Register>,
    divisor: ReadWrite<u32, Divisor::Register>,
}

pub struct Clock {
    base_addr: usize,
    source: ClockSource,
}

impl Clock {
    pub fn new(base_addr: usize, instance: ClockInstance, source: ClockSource) -> Self {
        Clock {
            base_addr: base_addr + (instance as usize) * 0x8,
            source,
        }
    }

    pub fn get_source(&self) -> ClockSource {
        self.source
    }

    pub fn start(&self, div_i: u32, div_f: u32, mash: u32) {
        self.divisor.write(
            Divisor::DIVI.val(div_i)
                + Divisor::DIVF.val(div_f)
                + Divisor::PASSWD.val(CLOCK_PASSWORD)
        );
        self.control.write(
            Control::ENAB::SET
                + Control::SRC.val(self.source as u32)
                + Control::MASH.val(mash)
                + Control::PASSWD.val(CLOCK_PASSWORD)
        );
    }

    pub fn stop(&self) {
        self.control.write(
            Control::PASSWD.val(CLOCK_PASSWORD)
                + Control::KILL::SET
        );
        while self.control.is_set(Control::BUSY) {
            asm::nop();
        }
    }

    fn ptr(&self) -> *const RegisterBlock {
        self.base_addr as *const _
    }
}

impl Deref for Clock {
    type Target = RegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}
