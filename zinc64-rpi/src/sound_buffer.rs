// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64_core::SoundOutput;

use crate::util::circular_buffer::CircularBuffer;
use crate::util::sync::NullLock;

pub struct SoundBuffer {
    buffer: NullLock<CircularBuffer<i16>>,
}

impl SoundBuffer {
    pub fn new(length: usize) -> Self {
        let buffer = NullLock::new(
            CircularBuffer::new(length)
        );
        SoundBuffer { buffer }
    }

    pub fn get_data(&self) -> &NullLock<CircularBuffer<i16>> {
        &self.buffer
    }
}

impl SoundOutput for SoundBuffer {
    fn reset(&self) {
        self.buffer.lock(|buf| {
            buf.reset();
        });
    }

    fn write(&self, samples: &[i16]) {
        self.buffer.lock(|buf| {
            for sample in samples {
                buf.push(*sample);
                buf.push(*sample);
            }
        });
    }
}
