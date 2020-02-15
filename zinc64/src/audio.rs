// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::{Arc, Mutex};
use std::thread;

use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use zinc64_core::SoundOutput;

use crate::util::CircularBuffer;

// TODO app: audio warp handling

const SCALER_MAX: i32 = 4096;
const SCALER_SHIFT: usize = 12;
const VOLUME_MAX: u8 = 100;

struct AudioRendererState {
    mute: bool,
    scaler: i32,
    volume: u8,
}

pub struct AudioRenderer {
    // Resources
    device: cpal::Device,
    event_loop: Arc<cpal::EventLoop>,
    stream_id: cpal::StreamId,
    // Runtime state
    buffer: Arc<SoundBuffer>,
    state: Arc<Mutex<AudioRendererState>>,
}

impl AudioRenderer {
    pub fn build(
        freq: i32,
        channels: u8,
        samples: u16,
        buffer: Arc<SoundBuffer>,
    ) -> Result<AudioRenderer, anyhow::Error> {
        let host = cpal::default_host();
        let event_loop = host.event_loop();

        let format = cpal::Format {
            channels: 2,
            sample_rate: cpal::SampleRate(freq as u32),
            data_type: cpal::SampleFormat::I16,
        };

        // brute force search until it cpal provides a better way, e.g.:
        // https://github.com/RustAudio/cpal/issues/368
        let mut good_devices = host.devices()?.map(|dev| {
            let s_id = event_loop.build_output_stream(&dev, &format);
            match s_id {
                Err(_) => {
                    eprintln!("Couldn't build desired stream with device {}",
                              dev.name().unwrap_or("<error retrieving device name>".into()));
                    None
                },
                Ok(s_id) => { Some((dev, s_id)) }
            }
        });

        let (device, stream_id) = good_devices.find(|x| x.is_some())
            .expect(&format!("No suitable audio device for format: {:?}", format)).unwrap();

        let state = Arc::new(Mutex::new(AudioRendererState {
            mute: false,
            scaler: SCALER_MAX,
            volume: VOLUME_MAX,
        }));
        Ok(AudioRenderer {
            device,
            event_loop: Arc::new(event_loop),
            stream_id,
            buffer,
            state,
        })
    }

    pub fn start(&self) {
        let input = self.buffer.clone();
        let state = self.state.clone();
        let event_loop = self.event_loop.clone();
        thread::spawn(move || {
            event_loop.run(move |id, result| {
                let data = match result {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("an error occurred on stream {:?}: {}", id, err);
                        return;
                    }
                };
                match data {
                    cpal::StreamData::Output {
                        buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer),
                    } => {
                        write_data(state.clone(), input.clone(), &mut buffer, 2);
                        // FIXME format.channels as usize
                    }
                    _ => (),
                }
            })
        });
    }

    pub fn pause(&self) {
        self.event_loop
            .pause_stream(self.stream_id.clone())
            .expect("failed to pause stream");
    }

    pub fn play(&mut self) {
        self.event_loop
            .play_stream(self.stream_id.clone())
            .expect("failed to play stream");
    }

    pub fn set_volume(&mut self, volume: u8) {
        let mut state = self.state.lock().unwrap();
        state.scaler = (volume as i32 * SCALER_MAX) / VOLUME_MAX as i32;
        state.volume = volume;
    }

    pub fn toggle_mute(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.mute = !state.mute;
    }
}

fn write_data(
    state: Arc<Mutex<AudioRendererState>>,
    input: Arc<SoundBuffer>,
    output: &mut [i16],
    channels: usize,
) {
    let state = state.lock().unwrap();
    let mut input = input.buffer.lock().unwrap();
    for frame in output.chunks_mut(channels) {
        let value = input.pop().unwrap_or(0);
        for sample in frame.iter_mut() {
            if !state.mute {
                *sample = ((value as i32 * state.scaler) >> (SCALER_SHIFT as i32)) as i16;
            } else {
                *sample = 0;
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
