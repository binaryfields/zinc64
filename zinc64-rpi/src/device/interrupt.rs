// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use core::ops::Deref;
use register::mmio::{ReadOnly, ReadWrite};

use crate::memory;

// SPEC: BCM2837 ARM Peripherals v2.1 - 7 Interrupts
// SPEC: https://github.com/raspberrypi/linux/blob/rpi-3.12.y/arch/arm/mach-bcm2708/include/mach/platform.h

const ALL_BITS_MASK: u32 = 0xffff_ffff;
const NR_IRQS: usize = 32; // FIXME 32 + 32 + 20;

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum Irq {
    Timer0 = 0,
    Timer1 = 1,
    Timer2 = 2,
    Timer3 = 3,
    Codec0 = 4,
    Codec1 = 5,
    Codec2 = 6,
    VcJepg = 7,
    Isp = 8,
    VcUsb = 9,
    Vc3d = 10,
    Transposer = 11,
    MulticoreSync0 = 12,
    MulticoreSync1 = 13,
    MulticoreSync2 = 14,
    MulticoreSync3 = 15,
    Dma0 = 16,
    Dma1 = 17,
    VcDma2 = 18,
    VcDma3 = 19,
    Dma4 = 20,
    Dma5 = 21,
    Dma6 = 22,
    Dma7 = 23,
    Dma8 = 24,
    Dma9 = 25,
    Dma10 = 26,
    Dma11 = 27,
    Dma12= 28,
    Aux = 29,
    Arm = 30,
    VpUdma = 31,
}

pub trait IrqHandler {
    fn handle_interrupt(&mut self);
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct RegisterBlock {
    pending: ReadOnly<u32>,
    pending_1: ReadOnly<u32>,
    pending_2: ReadOnly<u32>,
    fiq_control: ReadWrite<u32>,
    enable_1: ReadWrite<u32>,
    enable_2: ReadWrite<u32>,
    enable_basic: ReadWrite<u32>,
    disable_1: ReadWrite<u32>,
    disable_2: ReadWrite<u32>,
    disable_basic: ReadWrite<u32>,
}

pub struct InterruptControl {
    base_addr: usize,
    handlers: [Option<Box<IrqHandler>>; NR_IRQS],
}

impl InterruptControl {
    pub const fn new() -> Self {
        InterruptControl {
            base_addr: memory::map::INTERRUPT_BASE + 0x200,
            handlers: [
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
            ],
        }
    }

    pub fn disable_irqs() {
        unsafe { asm!("msr DAIFSet, #2" :::: "volatile") }
    }

    pub fn enable_irqs() {
        unsafe { asm!("msr DAIFClr, #2" :::: "volatile") }
    }

    pub fn init(&self) {
        self.fiq_control.set(0);
        self.disable_1.set(ALL_BITS_MASK);
        self.disable_2.set(ALL_BITS_MASK);
        self.disable_basic.set(ALL_BITS_MASK);
        Self::enable_irqs();
    }

    pub fn disable(&self, irq: Irq) {
        self.disable_1.set(1 << (irq as u32));
    }

    pub fn enable(&self, irq: Irq) {
        self.enable_1.set(1 << (irq as u32));
    }

    pub fn handle_irq(&mut self) {
        if let Some(handler) = &mut self.handlers[Irq::Dma0 as usize] { // FIXME
            handler.handle_interrupt();
        }
    }

    pub fn register<H: IrqHandler + 'static>(&mut self, irq: Irq, handler: H) {
        self.handlers[irq as usize] = Some(Box::new(handler));
    }

    pub fn unregister(&mut self, irq: Irq) {
        self.handlers[irq as usize] = None;
    }

    fn ptr(&self) -> *const RegisterBlock {
        self.base_addr as *const _
    }
}

impl Deref for InterruptControl {
    type Target = RegisterBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

