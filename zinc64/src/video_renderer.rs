// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::result::Result;

use sdl2;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render;
use time;
use zinc64_core::Shared;

use crate::video_buffer::VideoBuffer;
use sdl2::render::Canvas;
use sdl2::video::Window;

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

    pub fn render(&mut self, window: &mut render::WindowCanvas) -> Result<(), String> {
        self.viewport_tex
            .update(
                None,
                self.video_buffer.borrow().get_pixel_data(),
                self.video_buffer.borrow().get_pitch(),
            )
            .map_err(|_| "failed to update texture")?;
        window.clear();
        window.copy(&self.viewport_tex, Some(self.viewport_rect), None)?;
        window.present();
        self.frame_count = self.frame_count.wrapping_add(1);
        self.last_frame_ts = time::precise_time_ns();
        Ok(())
    }
}
