// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: https://github.com/PeterLemon/RaspberryPi/blob/master/Sound/PWM/12Bit/44100Hz/Stereo/DMA/kernel8.asm

use alloc::prelude::*;
use alloc::rc::Rc;
use core::mem;

use crate::memory;
use crate::util::sync::NullLock;

use super::clock::{self, Clock, ClockInstance, ClockSource};
use super::delay;
use super::dma::{self, Buffer, ControlBlockWrapper, DmaChannel, DreqPeripheralMap, DMA};
use super::gpio::{self, GPIO};
use super::interrupt::{Irq, IrqHandler};
use super::pwm::PWM;

const DMA_CHANNEL_PWM: usize = 0;

pub trait SndCallback {
    fn callback(&mut self, buffer: &mut [u32]);
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Idle,
    Playing,
    Paused,
    Error,
}

pub struct Snd<'a> {
    dev: Rc<NullLock<SndDev<'a>>>,
}

#[allow(unused)]
impl<'a> Snd<'a> {
    pub fn open(gpio: &GPIO, freq: u32, channels: usize, samples: usize, cb: Box<SndCallback>) -> Result<Snd<'a>, &'static str> {
        let dev = Rc::new(
            NullLock::new(
                SndDev::open(gpio, freq, channels, samples, cb)?
            )
        );
        Ok(Snd {
            dev
        })
    }

    pub fn close(&self) {
        crate::IRQ_CONTROL.lock(|ctl| {
            ctl.unregister(Irq::Dma0);
        });
        self.dev.lock(|dev| {
            dev.close();
        });
    }

    pub fn make_irq_handler(&self) -> impl IrqHandler + 'a {
        SndIrqHandler::new(self.dev.clone())
    }

    pub fn start(&self) {
        self.dev.lock(|dev| {
            dev.start();
        });
    }

    pub fn stop(&self) {
        self.dev.lock(|dev| {
            dev.stop();
        });
    }
}

pub struct SndIrqHandler<'a> {
    dev: Rc<NullLock<SndDev<'a>>>,
}

impl<'a> SndIrqHandler<'a> {
    pub fn new(dev: Rc<NullLock<SndDev<'a>>>) -> Self {
        SndIrqHandler {
            dev,
        }
    }
}

impl<'a> IrqHandler for SndIrqHandler<'a> {
    fn handle_interrupt(&mut self) {
        self.dev.lock(|dev| {
            dev.handle_interrupt();
        });
    }
}

#[allow(unused)]
pub struct SndDev<'a> {
    // Configuration
    freq: u32,
    channels: usize,
    samples: usize,
    sample_range: u32,
    cb: Box<SndCallback>,
    // Resources
    clock: Clock,
    dma_channel: DmaChannel,
    pwm: PWM,
    // Runtime state
    buffer: [Buffer<'a, u32>; 2],
    buffer_idx: usize,
    control_block: ControlBlockWrapper<'a>,
    control_block_2: ControlBlockWrapper<'a>,
    status: Status,
}

#[allow(unused)]
impl<'a> SndDev<'a> {
    pub fn open(gpio: &GPIO, freq: u32, channels: usize, samples: usize, cb: Box<SndCallback>) -> Result<SndDev<'a>, &'static str> {
        info!("Initializing device ...");
        let buffer = [
            Buffer::alloc(samples * channels)?,
            Buffer::alloc(samples * channels)?,
        ];
        let mut control_block = ControlBlockWrapper::alloc()?;
        let mut control_block_2 = ControlBlockWrapper::alloc()?;
        let ti = dma::TI::INTEN::True
            + dma::TI::DEST_DREQ::True
            + dma::TI::SRC_INC::True
            + dma::TI::PERMAP.val(DreqPeripheralMap::Pwm as u32);
        control_block.init(
            ti.value,
            memory::bus_address(buffer[0].buf.as_ptr() as usize),
            memory::bus_io_address(memory::map::PWM_FIF1),
            (buffer[0].buf.len() * mem::size_of::<u32>()) as u32,
            0,
        );
        control_block_2.init(
            ti.value,
            memory::bus_address(buffer[1].buf.as_ptr() as usize),
            memory::bus_io_address(memory::map::PWM_FIF1),
            (buffer[1].buf.len() * mem::size_of::<u32>()) as u32,
            0,
        );
        control_block.set_next(control_block_2.ptr());
        control_block_2.set_next(control_block.ptr());
        let clock = Clock::new(
            memory::map::CM_BASE,
            ClockInstance::Pwm,
            ClockSource::PllD,
        );
        let pwm = PWM::new(memory::map::PWM_BASE);
        let dma_channel = DmaChannel::new(memory::map::DMA_BASE, DMA_CHANNEL_PWM);
        // $1624 ; Range = 12bit 44100Hz Stereo
        // (500MHz / 2) / 44100 = 5669 = 0x1625 (~2^12 = 4096)
        let pwm_freq = clock.get_source().frequency() / clock::MIN_DIVIDER;
        let sample_range = (pwm_freq + freq / 2) / freq;
        let snd = SndDev {
            freq,
            channels,
            samples,
            sample_range,
            cb,
            clock,
            dma_channel,
            pwm,
            buffer,
            buffer_idx: 0,
            control_block,
            control_block_2,
            status: Status::Idle,
        };
        snd.init(gpio);
        info!("Device initialized.");
        Ok(snd)
    }

    fn init(&self, gpio: &GPIO) {
        // Set GPIO 40 & 45 (Phone Jack) To Alternate PWM Function 0
        gpio.GPFSEL4.modify(
            gpio::GPFSEL4::FSEL40.val(gpio::GPFSEL::Alt0 as u32)
                + gpio::GPFSEL4::FSEL45.val(gpio::GPFSEL::Alt0 as u32)
        );
        gpio.GPPUD.write(gpio::GPPUD::PUD::Off);
        delay::wait_cycles(150);
        gpio.GPPUDCLK1.write(gpio::GPREGSET1::P40::SET + gpio::GPREGSET1::P45::SET);
        delay::wait_cycles(150);
        gpio.GPPUD.set(0);
        gpio.GPPUDCLK1.set(0);
        // Start devices
        self.clock.start(clock::MIN_DIVIDER, 0, 0);
        self.pwm.start(self.sample_range);
    }

    pub fn close(&self) {
        assert_eq!(self.status, Status::Idle);
        crate::IRQ_CONTROL.lock(|ctl| {
            ctl.disable(Irq::Dma0);
            ctl.unregister(Irq::Dma0);
        });
        DMA.disable(&self.dma_channel);
        self.pwm.stop();
        delay::wait_msec(2);
        self.clock.stop();
        delay::wait_msec(2);
    }

    pub fn start(&mut self) {
        info!("Starting device ...");
        assert_eq!(self.status, Status::Idle);
        self.status = Status::Playing;
        self.control_block.dump("CB1");
        self.control_block_2.dump("CB2");
        self.fill_buffer();
        self.fill_buffer();
        self.pwm.enable_dma();
        self.pwm.set_repeat_last(false);
        DMA.enable(&self.dma_channel);
        self.dma_channel.reset();
        self.dma_channel.start(&self.control_block);
    }

    pub fn stop(&mut self) {
        info!("Stopping device ...");
        assert_ne!(self.status, Status::Idle);
        self.status = Status::Idle;
        self.dma_channel.stop();
        self.pwm.disable_dma();
    }

    fn fill_buffer(&mut self) {
        self.cb.callback(self.buffer[self.buffer_idx].buf);
        self.buffer_idx ^= 1;
    }

    pub fn handle_interrupt(&mut self) {
        assert_ne!(self.status, Status::Idle);
        assert_eq!(self.dma_channel.is_interrupt(), true);
        self.dma_channel.clear_interrupt();
        if self.dma_channel.is_error() {
            error!("DMA channel reported error");
            self.status = Status::Error;
        }
        match self.status {
            Status::Playing => {
                // self.dma_channel.dump();
                self.dma_channel.resume();
                self.fill_buffer();
            },
            _ => (),
        }
    }
}
