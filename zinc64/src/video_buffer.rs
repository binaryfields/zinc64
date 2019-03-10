// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::transmute_ptr_to_ptr))]

use core::mem;
use zinc64_core::VideoOutput;

pub struct VideoBuffer {
    dim: (usize, usize),
    palette: [u32; 16],
    pixels: Vec<u32>,
}

impl VideoBuffer {
    pub fn new(width: u32, height: u32, palette: [u32; 16]) -> VideoBuffer {
        VideoBuffer {
            dim: (width as usize, height as usize),
            palette,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn get_pitch(&self) -> usize {
        self.dim.0 * mem::size_of::<u32>()
    }

    pub fn get_pixel_data(&self) -> &[u8] {
        unsafe { mem::transmute::<&[u32], &[u8]>(self.pixels.as_ref()) }
    }
}

impl VideoOutput for VideoBuffer {
    fn get_dimension(&self) -> (usize, usize) {
        self.dim
    }

    fn reset(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = 0x00;
        }
    }

    fn write(&mut self, index: usize, color: u8) {
        self.pixels[index] = self.palette[color as usize];
    }
}
