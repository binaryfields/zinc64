// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::transmute_ptr_to_ptr))]

use core::mem;
use zinc64_core::VideoOutput;

const PIXEL_BYTES: usize = 4;

pub struct FrameBuffer {
    dim: (usize, usize),
    palette: [u32; 16],
    pixels: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, palette: [u32; 16]) -> FrameBuffer {
        FrameBuffer {
            dim: (width as usize, height as usize),
            palette,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn get_pitch(&self) -> usize {
        self.dim.0 * PIXEL_BYTES
    }

    pub fn get_pixel_data(&self) -> &[u8] {
        unsafe { mem::transmute::<&[u32], &[u8]>(self.pixels.as_ref()) }
    }

    #[allow(unused)]
    pub fn write(&mut self, x: u16, y: u16, color: u8) {
        let index = self.index(x, y);
        self.pixels[index] = self.palette[color as usize];
    }

    #[allow(unused)]
    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.dim.0 + x as usize
    }
}

impl VideoOutput for FrameBuffer {
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
