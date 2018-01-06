/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::sync::{Arc, Mutex};

use sdl2;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use zinc64::core::SoundBuffer;

// TODO app/audio: add play/resume/volume

pub struct AppAudio {
    buffer: Arc<Mutex<SoundBuffer>>,
}

impl AppAudio {
    pub fn new_device(
        sdl_audio: &sdl2::AudioSubsystem,
        sample_rate: i32,
        channels: u8,
        buffer_size: u16,
        buffer: Arc<Mutex<SoundBuffer>>
    ) -> Result<AudioDevice<AppAudio>, String> {
        let audio_spec = AudioSpecDesired {
            freq: Some(sample_rate),
            channels: Some(channels),
            samples: Some(buffer_size),
        };
        let audio_device = sdl_audio.open_playback(None, &audio_spec, |spec| {
            info!(target: "audio", "{:?}", spec);
            AppAudio { buffer }
        })?;
        Ok(audio_device)
    }
}

impl AudioCallback for AppAudio {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let mut input = self.buffer.lock().unwrap();
        for x in out.iter_mut() {
            let sample = input.pop();
            *x = sample as f32 * 0.000020; // FIXME magic value
        }
    }
}