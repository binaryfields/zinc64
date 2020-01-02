// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::transmute_ptr_to_ptr))]

use std::result::Result;

use core::mem;
use sdl2;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render::{self, Canvas};
use sdl2::video::Window;
use time;
use zinc64_core::{Shared, VideoOutput};

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

pub struct VideoRenderer {
    // Configuration
    viewport_rect: Rect,
    // Resources
    video_buffer: Shared<VideoBuffer>,
    viewport_tex: render::Texture,
    // Runtime state
    frame_count: u32,
    last_frame_ts: u64,
}

impl VideoRenderer {
    pub fn build(
        window: &Canvas<Window>,
        screen_size: (u32, u32),
        viewport_offset: (u32, u32),
        viewport_size: (u32, u32),
        video_buffer: Shared<VideoBuffer>,
    ) -> Result<VideoRenderer, String> {
        let texture = window
            .texture_creator()
            .create_texture_streaming(
                pixels::PixelFormatEnum::ARGB8888,
                screen_size.0,
                screen_size.1,
            )
            .map_err(|_| "failed to create texture")?;
        let viewport_rect = Rect::new(
            viewport_offset.0 as i32,
            viewport_offset.1 as i32,
            viewport_size.0,
            viewport_size.1,
        );
        let renderer = VideoRenderer {
            viewport_rect,
            video_buffer,
            viewport_tex: texture,
            frame_count: 0,
            last_frame_ts: 0,
        };
        Ok(renderer)
    }

    pub fn render(&mut self, canvas: &mut render::WindowCanvas) -> Result<(), String> {
        self.viewport_tex
            .update(
                None,
                self.video_buffer.borrow().get_pixel_data(),
                self.video_buffer.borrow().get_pitch(),
            )
            .map_err(|_| "failed to update texture")?;
        canvas.clear();
        canvas.copy(&self.viewport_tex, Some(self.viewport_rect), None)?;
        self.frame_count = self.frame_count.wrapping_add(1);
        self.last_frame_ts = time::precise_time_ns();
        Ok(())
    }
}
