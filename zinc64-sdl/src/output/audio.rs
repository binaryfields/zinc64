// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::Arc;

use sdl2;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

use super::SoundBuffer;

// TODO app: audio warp handling

const SCALER_MAX: i32 = 4096;
const SCALER_SHIFT: usize = 12;
const VOLUME_MAX: u8 = 100;

pub struct AppAudio {
    buffer: Arc<SoundBuffer>,
    mute: bool,
    scaler: i32,
    volume: u8,
}

impl AppAudio {
    pub fn new_device(
        sdl_audio: &sdl2::AudioSubsystem,
        sample_rate: i32,
        channels: u8,
        buffer_size: u16,
        buffer: Arc<SoundBuffer>,
    ) -> Result<AudioDevice<AppAudio>, String> {
        let audio_spec = AudioSpecDesired {
            freq: Some(sample_rate),
            channels: Some(channels),
            samples: Some(buffer_size),
        };
        let audio_device = sdl_audio.open_playback(None, &audio_spec, |spec| {
            info!(target: "audio", "{:?}", spec);
            AppAudio {
                buffer,
                mute: false,
                scaler: SCALER_MAX,
                volume: VOLUME_MAX,
            }
        })?;
        Ok(audio_device)
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.scaler = (volume as i32 * SCALER_MAX) / VOLUME_MAX as i32;
        self.volume = volume;
    }

    pub fn toggle_mute(&mut self) {
        let mute = self.mute;
        self.mute = !mute;
    }
}

impl AudioCallback for AppAudio {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        if !self.mute {
            self.buffer.copy(out, self.scaler, SCALER_SHIFT);
        } else {
            for x in out.iter_mut() {
                *x = 0;
            }
        }
    }
}
