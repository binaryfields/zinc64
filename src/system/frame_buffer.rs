// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::VideoOutput;
use std::mem;

const PIXEL_BYTES: usize = 4;

pub struct FrameBuffer {
    dim: (usize, usize),
    palette: [u32; 16],
    pixels: Vec<u32>,
    sync: bool,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32, palette: [u32; 16]) -> FrameBuffer {
        FrameBuffer {
            dim: (width as usize, height as usize),
            palette,
            pixels: vec![0; (width * height) as usize],
            sync: false,
        }
    }

    pub fn get_pitch(&self) -> usize {
        self.dim.0 * PIXEL_BYTES
    }

    pub fn get_pixel_data(&self) -> &[u8] {
        unsafe { mem::transmute::<&[u32], &[u8]>(self.pixels.as_ref()) }
    }

    pub fn get_sync(&self) -> bool {
        self.sync
    }

    pub fn set_sync(&mut self, value: bool) {
        self.sync = value;
    }

    pub fn reset(&mut self) {
        for i in 0..self.pixels.len() {
            self.pixels[i] = 0x00;
        }
        self.sync = false;
    }

    pub fn write(&mut self, x: u16, y: u16, color: u8) {
        let index = self.index(x, y);
        self.pixels[index] = self.palette[color as usize];
    }

    fn index(&self, x: u16, y: u16) -> usize {
        y as usize * self.dim.0 + x as usize
    }
}

impl VideoOutput for FrameBuffer {
    fn set_sync(&mut self, value: bool) {
        FrameBuffer::set_sync(self, value);
    }

    fn write(&mut self, x: u16, y: u16, color: u8) {
        FrameBuffer::write(self, x, y, color);
    }
}
