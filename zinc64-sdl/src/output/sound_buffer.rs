// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::sync::Mutex;

use zinc64_core::SoundOutput;

use crate::util::CircularBuffer;

pub struct SoundBuffer {
    buffer: Mutex<CircularBuffer<i16>>,
}

impl SoundBuffer {
    pub fn new(length: usize) -> Self {
        let buffer = Mutex::new(CircularBuffer::new(length));
        SoundBuffer { buffer }
    }

    pub fn copy(&self, out: &mut [i16], scaler: i32, scaler_shift: usize) {
        let mut input = self.buffer.lock().unwrap();
        if input.len() < out.len() {
            debug!(target: "app", "audio callback underflow {}/{}", out.len(), input.len());
        }
        for x in out.iter_mut() {
            *x = ((input.pop() as i32 * scaler) >> scaler_shift) as i16;
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
