// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::result::Result;

use sdl2;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render;
use sdl2::video;
use time;

use crate::video_buffer::VideoBuffer;

pub struct VideoRenderer {
    // Configuration
    viewport_rect: Rect,
    // Resources
    canvas: render::WindowCanvas,
    texture: render::Texture,
    // Runtime state
    frame_count: u32,
    last_frame_ts: u64,
}

impl VideoRenderer {
    pub fn build(
        sdl_video: &sdl2::VideoSubsystem,
        window_size: (u32, u32),
        screen_size: (u32, u32),
        viewport_offset: (u32, u32),
        viewport_size: (u32, u32),
        fullscreen: bool,
    ) -> Result<VideoRenderer, String> {
        let mut builder = sdl_video.window("zinc64", window_size.0, window_size.1);
        builder.opengl();
        if fullscreen {
            builder.fullscreen();
        } else {
            builder.position_centered();
            builder.resizable();
        }
        let window = builder.build().map_err(|_| "failed to create window")?;
        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|_| "failed to create canvas")?;
        let texture = canvas
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
            canvas,
            texture,
            frame_count: 0,
            last_frame_ts: 0,
        };
        Ok(renderer)
    }

    pub fn render(&mut self, frame_buffer: &VideoBuffer) -> Result<(), String> {
        self.texture
            .update(
                None,
                frame_buffer.get_pixel_data(),
                frame_buffer.get_pitch(),
            )
            .map_err(|_| "failed to update texture")?;
        self.canvas.clear();
        self.canvas
            .copy(&self.texture, Some(self.viewport_rect), None)?;
        self.canvas.present();
        self.frame_count = self.frame_count.wrapping_add(1);
        self.last_frame_ts = time::precise_time_ns();
        Ok(())
    }

    pub fn toggle_fullscreen(&mut self) {
        let window = self.canvas.window_mut();
        match window.fullscreen_state() {
            video::FullscreenType::Off => {
                window.set_fullscreen(video::FullscreenType::True).unwrap();
            }
            video::FullscreenType::True | video::FullscreenType::Desktop => {
                window.set_fullscreen(video::FullscreenType::Off).unwrap();
            }
        }
    }
}
