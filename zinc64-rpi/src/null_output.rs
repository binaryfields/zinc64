// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(unused)]

use zinc64_core::{SoundOutput, VideoOutput};

pub struct NullSound;

impl SoundOutput for NullSound {
    fn reset(&self) {}
    fn write(&self, _samples: &[i16]) {}
}

pub struct NullVideo;

impl VideoOutput for NullVideo {
    fn get_dimension(&self) -> (usize, usize) {
        (0, 0)
    }
    fn reset(&mut self) {}
    fn write(&mut self, _index: usize, _color: u8) {}
}
