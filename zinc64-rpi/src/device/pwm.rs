// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::ops::Deref;
use register::{mmio::ReadWrite, register_bitfields};

// SPEC: BCM2837 ARM Peripherals v2.1 - 9 Pulse Width Modulator

register_bitfields! { u32,
    Control [
        PWEN1 OFFSET(0) NUMBITS(1) [],
        MODE1 OFFSET(1) NUMBITS(1) [],
        RPTL1 OFFSET(2) NUMBITS(1) [],
        SBIT1 OFFSET(3) NUMBITS(1) [],
        POLA1 OFFSET(4) NUMBITS(1) [],
        USEF1 OFFSET(5) NUMBITS(1) [],
        CLRF1 OFFSET(6) NUMBITS(1) [],
        MSEN1 OFFSET(7) NUMBITS(1) [],
        PWEN2 OFFSET(8) NUMBITS(1) [],
        MODE2 OFFSET(9) NUMBITS(1) [],
        RPTL2 OFFSET(10) NUMBITS(1) [],
        SBIT2 OFFSET(11) NUMBITS(1) [],
        POLA2 OFFSET(12) NUMBITS(1) [],
        USEF2 OFFSET(13) NUMBITS(1) [],
        MSEN2 OFFSET(15) NUMBITS(1) []
    ],
    Status [
        FULL1 OFFSET(0) NUMBITS(1) [],
        EMPT1 OFFSET(1) NUMBITS(1) [],
        WERR1 OFFSET(2) NUMBITS(1) [],
        RERR1 OFFSET(3) NUMBITS(1) [],
        GAPO1 OFFSET(4) NUMBITS(1) [],
        GAPO2 OFFSET(5) NUMBITS(1) [],
        GAPO3 OFFSET(6) NUMBITS(1) [],
        GAPO4 OFFSET(7) NUMBITS(1) [],
        BERR OFFSET(8) NUMBITS(1) [],
        STA1 OFFSET(9) NUMBITS(1) [],
        STA2 OFFSET(10) NUMBITS(1) [],
        STA3 OFFSET(11) NUMBITS(1) [],
        STA4 OFFSET(12) NUMBITS(1) []
    ],
    DMAC [
        DREQ OFFSET(0) NUMBITS(8) [],
        PANIC OFFSET(8) NUMBITS(8) [],
        ENAB OFFSET(31) NUMBITS(1) []
    ]
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct RegisterBlock {
    control: ReadWrite<u32, Control::Register>,
    status: ReadWrite<u32, Status::Register>,
    dmac: ReadWrite<u32, DMAC::Register>,
    _reserved_0: u32,
    range_1: ReadWrite<u32>,
    data_1: ReadWrite<u32>,
    fifo_1: ReadWrite<u32>,
    _reserved_1: u32,
    range_2: ReadWrite<u32>,
    data_2: ReadWrite<u32>,
}

pub struct PWM {
    base_addr: usize,
}

impl PWM {
    pub fn new(base_addr: usize) -> Self {
        PWM { base_addr }
    }

    pub fn disable_dma(&self) {
        self.dmac.write(DMAC::ENAB::CLEAR);
    }

    pub fn enable_dma(&self) {
        self.dmac
            .write(DMAC::ENAB::SET + DMAC::DREQ.val(1) + DMAC::PANIC.val(0));
    }

    pub fn set_repeat_last(&self, enabled: bool) {
        if enabled {
            self.control
                .modify(Control::RPTL1::SET + Control::RPTL2::SET);
        } else {
            self.control
                .modify(Control::RPTL1::CLEAR + Control::RPTL2::CLEAR);
        }
    }

    pub fn start(&self, range: u32) {
        self.range_1.set(range);
        self.range_2.set(range);
        self.control.write(
            Control::PWEN1::SET
                + Control::USEF1::SET
                + Control::PWEN2::SET
                + Control::USEF2::SET
                + Control::CLRF1::SET,
        );
    }

    pub fn stop(&self) {
        self.dmac.set(0);
        self.control.set(0);
    }

    fn ptr(&self) -> *const RegisterBlock {
        self.base_addr as *const _
    }
}

impl Deref for PWM {
    type Target = RegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}
