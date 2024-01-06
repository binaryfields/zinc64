// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::{Arc, Mutex};
use std::thread;

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use zinc64_core::SoundOutput;

use crate::util::CircularBuffer;

// TODO app: audio warp handling

const SCALER_MAX: i32 = 4096;
const SCALER_SHIFT: usize = 12;
const VOLUME_MAX: u8 = 100;
const SAMPLE_FORMAT_PREFERENCE: [cpal::SampleFormat; 3] = [
    cpal::SampleFormat::I16, 
    cpal::SampleFormat::U16, 
    cpal::SampleFormat::F32
];

struct AudioRendererState {
    mute: bool,
    scaler: i32,
    volume: u8,
}

pub struct AudioRenderer {
    // Resources
    stream: cpal::Stream,
    // Runtime state
    state: Arc<Mutex<AudioRendererState>>,
}

impl AudioRenderer {
    pub fn build(
        freq: i32,
        channels: u8,
        samples: u16,
        input: Arc<SoundBuffer>,
    ) -> Result<AudioRenderer, anyhow::Error> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("failed to find a default output device");
        let config = select_config(&device, freq)?;
        let state = Arc::new(
            Mutex::new(
                AudioRendererState {
                    mute: false,
                    scaler: SCALER_MAX,
                    volume: VOLUME_MAX,
               }
            )
        );

        info!("Audio Device {:?} with format {:?}", device.name()
            .unwrap_or("<error retrieving device name>".into()), config);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => make_stream::<i16>(state.clone(), input.clone(), &device, &config.into())?,
            cpal::SampleFormat::U16 => make_stream::<u16>(state.clone(), input.clone(), &device, &config.into())?,
            cpal::SampleFormat::F32 => make_stream::<f32>(state.clone(), input.clone(), &device, &config.into())?,
            sample_format => panic!("Unsupported sample format {}", sample_format),
        };

        Ok(
            AudioRenderer {
                stream,
                state,
            }
        )
    }

    pub fn start(&mut self) {
        self.play();
    }

    pub fn pause(&self) {
        self.stream
            .pause()
            .expect("failed to pause stream");
    }

    pub fn play(&mut self) {
        self.stream
            .play()
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

fn select_config(device: &cpal::Device, freq: i32) -> Result<cpal::SupportedStreamConfig, anyhow::Error> {
    let output_configs: cpal::SupportedOutputConfigs = device.supported_output_configs()?;
    let all_output_configs: Vec<cpal::SupportedStreamConfigRange> = output_configs.collect();

    // scan for best matching sample format until cpal provides a better way, e.g.:
    // https://github.com/RustAudio/cpal/issues/368
    let mut possible_configs = SAMPLE_FORMAT_PREFERENCE.iter().filter_map(|sample_format| {
        let format = cpal::SupportedStreamConfig::new(
            2,
            cpal::SampleRate(freq as u32),
            cpal::SupportedBufferSize::Unknown,
            *sample_format,
        );

        let mut matches = all_output_configs.iter().filter(|supported| {
            (supported.channels() >= format.channels()) &
                (supported.sample_format() == format.sample_format()) &
                (supported.max_sample_rate() >= format.sample_rate()) &
                (supported.min_sample_rate() <= format.sample_rate())
        });

        match matches.next()
        {
            Some(_) => Some(format),
            None => None,
        }
    });

    let config = possible_configs.next()
        .expect(&format!("No suitable audio device for any sample format: {:?}",
                         SAMPLE_FORMAT_PREFERENCE));

    Ok(config)
}

fn make_stream<T>(
    state: Arc<Mutex<AudioRendererState>>,
    input: Arc<SoundBuffer>,
    device: &cpal::Device, 
    config: &cpal::StreamConfig
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::SizedSample + cpal::FromSample<i16>,
{
    let state_cloned = state.clone();
    let input_cloned = input.clone();
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(state_cloned.clone(), input_cloned.clone(), data, channels)
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn write_data<T>(
    state: Arc<Mutex<AudioRendererState>>,
    input: Arc<SoundBuffer>,
    output: &mut [T],
    channels: usize,
) where
    T: cpal::Sample + cpal::FromSample<i16>,
{
    let state = state.lock().unwrap();
    let mut input = input.buffer.lock().unwrap();
    for frame in output.chunks_mut(channels) {
        let value = input.pop().unwrap_or(0);
        for sample in frame.iter_mut() {
            if !state.mute {
                let value = ((value as i32 * state.scaler) >> (SCALER_SHIFT as i32)) as i16;
                let formatted_value = T::from_sample(value);
                *sample = formatted_value;
            } else {
                *sample = T::from_sample(0i16);
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
