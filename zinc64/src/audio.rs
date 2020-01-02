// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::{Arc, Mutex};

use sdl2;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use zinc64_core::SoundOutput;

use crate::util::CircularBuffer;

// TODO app: audio warp handling

const SCALER_MAX: i32 = 4096;
const SCALER_SHIFT: usize = 12;
const VOLUME_MAX: u8 = 100;

pub struct AudioRenderer {
    // Resources
    buffer: Arc<SoundBuffer>,
    // Runtime state
    mute: bool,
    scaler: i32,
    volume: u8,
}

impl AudioRenderer {
    pub fn build_device(
        sdl_audio: &sdl2::AudioSubsystem,
        freq: i32,
        channels: u8,
        samples: u16,
        buffer: Arc<SoundBuffer>,
    ) -> Result<AudioDevice<AudioRenderer>, String> {
        let audio_spec = AudioSpecDesired {
            freq: Some(freq),
            channels: Some(channels),
            samples: Some(samples),
        };
        let audio_device = sdl_audio.open_playback(None, &audio_spec, |spec| {
            info!(target: "audio", "{:?}", spec);
            let mut renderer = AudioRenderer {
                buffer,
                mute: false,
                scaler: SCALER_MAX,
                volume: VOLUME_MAX,
            };
            renderer.set_volume(VOLUME_MAX);
            renderer
        })?;
        Ok(audio_device)
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.scaler = (volume as i32 * SCALER_MAX) / VOLUME_MAX as i32;
        self.volume = volume;
    }

    pub fn toggle_mute(&mut self) {
        self.mute = !self.mute;
    }
}

impl AudioCallback for AudioRenderer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        if !self.mute {
            let mut input = self.buffer.buffer.lock().unwrap();
            if input.len() < out.len() {
                debug!(target: "app", "audio callback underflow {}/{}", out.len(), input.len());
            }
            for x in out.iter_mut() {
                let sample = input.pop().unwrap_or(0);
                *x = ((sample as i32 * self.scaler) >> (SCALER_SHIFT as i32)) as i16;
            }
        } else {
            for x in out.iter_mut() {
                *x = 0;
            }
        }
    }
}

pub struct SoundBuffer {
    buffer: Mutex<CircularBuffer<i16>>,
}

impl SoundBuffer {
    pub fn new(length: usize) -> Self {
        SoundBuffer {
            buffer: Mutex::new(CircularBuffer::new(length)),
        }
    }
}

impl SoundOutput for SoundBuffer {
    fn reset(&self) {
        let mut output = self.buffer.lock().unwrap();
        output.reset();
    }

    fn write(&self, samples: &[i16]) {
        let mut output = self.buffer.lock().unwrap();
        for sample in samples {
            output.push(*sample);
        }
    }
}
