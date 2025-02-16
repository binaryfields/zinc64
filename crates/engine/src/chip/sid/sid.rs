// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::cast_lossless)]

use std::sync::Arc;

use log::LogLevel;
use resid;

use crate::chip::SoundOutput;

#[derive(Clone, Copy, PartialEq)]
pub enum SamplingMethod {
    Fast,
    Interpolate,
    Resample,
    ResampleFast,
}

// TODO sound: add sid output sample rate test cases

pub struct Sid {
    // Dependencies
    sound_buffer: Arc<dyn SoundOutput>,
    // Functional Units
    resid: resid::Sid,
    // Runtime State
    buffer: [i16; 8192],
    cycles: u64,
}

impl Sid {
    pub fn new(chip_model: SidModel, sound_buffer: Arc<dyn SoundOutput>) -> Self {
        info!(target: "sound", "Initializing SID");
        let resid_model = match chip_model {
            SidModel::Mos6581 => resid::ChipModel::Mos6581,
            SidModel::Mos8580 => resid::ChipModel::Mos8580,
        };
        let resid = resid::Sid::new(resid_model);
        Sid {
            sound_buffer,
            resid,
            buffer: [0i16; 8192],
            cycles: 0,
        }
    }

    pub fn enable_filter(&mut self, enabled: bool) {
        self.resid.enable_filter(enabled);
    }

    pub fn set_sampling_parameters(
        &mut self,
        sampling_method: SamplingMethod,
        clock_freq: u32,
        sample_freq: u32,
    ) {
        let resid_sampling_method = match sampling_method {
            SamplingMethod::Fast => resid::SamplingMethod::Fast,
            SamplingMethod::Interpolate => resid::SamplingMethod::Interpolate,
            SamplingMethod::Resample => resid::SamplingMethod::Resample,
            SamplingMethod::ResampleFast => resid::SamplingMethod::ResampleFast,
        };
        self.resid
            .set_sampling_parameters(resid_sampling_method, clock_freq, sample_freq);
    }

    pub fn reset(&mut self, now: u64) {
        self.resid.reset();
        self.cycles = now;
    }

    pub fn read(&mut self, reg: u8, now: u64) -> u8 {
        self.sync(now);
        self.resid.read(reg)
    }

    pub fn write(&mut self, reg: u8, value: u8, now: u64) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "sid::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        self.sync(now);
        self.resid.write(reg, value)
    }

    pub fn process_vsync(&mut self, now: u64) {
        self.sync(now);
    }

    fn sync(&mut self, now: u64) {
        if self.cycles != now {
            let delta = (now - self.cycles) as u32;
            self.clock_delta(delta);
        }
    }

    fn clock_delta(&mut self, delta: u32) {
        if delta > 0 {
            let mut delta = delta;
            while delta > 0 {
                let (samples, next_delta) = self.resid.sample(delta, &mut self.buffer[..], 1);
                self.sound_buffer.write(&self.buffer[0..samples]);
                delta = next_delta;
            }
        }
        self.cycles = self.cycles.wrapping_add(delta as u64);
    }
}

#[derive(Clone, Copy)]
pub enum SidModel {
    Mos6581,
    Mos8580,
}
