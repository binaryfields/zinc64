// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::cell::Cell;
use core::result::Result;

use crate::device::gpio::GPIO;
use crate::device::interrupt::IrqHandler;
use crate::device::snd::{Snd, SndCallback};
use crate::sound_buffer::SoundBuffer;

const SCALER_MAX: u32 = 4096;
const SCALER_SHIFT: usize = 12;
const VOLUME_MAX: u8 = 100;

pub struct AudioCallback(pub Rc<AudioRenderer>);

impl SndCallback for AudioCallback {
    fn callback(&mut self, out: &mut [u32]) {
        self.0.write(out);
    }
}

pub struct AudioEngine<'a> {
    renderer: Rc<AudioRenderer>,
    snd_dev: Snd<'a>,
}

impl<'a> AudioEngine<'a> {
    pub fn build(
        gpio: &GPIO,
        freq: u32,
        samples: usize,
        buffer: Arc<SoundBuffer>,
    ) -> Result<AudioEngine<'a>, &'static str> {
        let renderer = Rc::new(
            AudioRenderer::new(buffer)
        );
        let snd_dev = Snd::open(
            &gpio,
            freq,
            2,
            samples,
            Box::new(AudioCallback(renderer.clone())),
        )?;
        Ok(AudioEngine {
            renderer,
            snd_dev,
        })
    }

    pub fn make_irq_handler(&self) -> impl IrqHandler + 'a {
        self.snd_dev.make_irq_handler()
    }

    #[allow(unused)]
    pub fn renderer(&self) -> &Rc<AudioRenderer> {
        &self.renderer
    }

    pub fn start(&self) {
        self.snd_dev.start();
    }
}

pub struct AudioRenderer {
    // Resources
    buffer: Arc<SoundBuffer>,
    // Runtime State
    mute: bool,
    scaler: u32,
    volume: u8,
    #[allow(unused)]
    pos: Cell<usize>,
}

#[allow(unused)]
impl AudioRenderer {
    pub fn new(buffer: Arc<SoundBuffer>) -> Self {
        let mut renderer = AudioRenderer {
            buffer,
            mute: false,
            scaler: 0,
            volume: 0,
            pos: Cell::new(0),
        };
        renderer.set_volume(VOLUME_MAX);
        renderer
    }

    pub fn is_mute(&self) -> bool {
        self.mute
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.scaler = (volume as u32 * SCALER_MAX) / VOLUME_MAX as u32;
        self.volume = volume;
    }

    pub fn toggle_mute(&mut self) {
        self.mute = !self.mute;
    }

    pub fn write(&self, out: &mut [u32]) {
        if !self.mute {
            self.copy(out);
        } else {
            for x in out.iter_mut() {
                *x = 0;
            }
        }
    }

    #[inline]
    fn convert_sample(&self, value: i16) -> u32 {
        let mut sample = value as i32;
        sample += 1 << 15;
        sample >>= 4; // FIXME
        sample as u32
    }

    fn copy(&self, out: &mut [u32]) {
        self.buffer.get_data().lock(|buf| {
            if buf.len() < out.len() {
                debug!("audio callback underflow {}/{}", out.len(), buf.len());
            }
            for x in out.iter_mut() {
                let sample = self.convert_sample(buf.pop());
                *x = (sample * self.scaler) >> SCALER_SHIFT;
            }
        });
    }

    /*
    fn test_callback(&self, buffer: &mut [u32]) {
        // info!("XXX Refilling buffer {}", self.pos.get());
        let samples = unsafe {
            core::slice::from_raw_parts(RES_SAMPLE.as_ptr() as *const u16, RES_SAMPLE.len() / 2)
        };
        for (i, value) in buffer.iter_mut().enumerate() {
            let index = self.pos.get() + i;
            if index < samples.len() {
                *value = samples[index] as u32;
            } else {
                *value = 0;
            }
        }

        let mut new_pos = self.pos.get() + buffer.len();
        if new_pos >= samples.len() {
            new_pos = 0;
        }
        self.pos.set(new_pos);
    }
    */
}

// static RES_SAMPLE: &[u8] = include_bytes!("../../res/sample.bin");
