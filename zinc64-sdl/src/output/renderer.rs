// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::result::Result;

use sdl2;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render;
use sdl2::video;
use time;
use zinc64::system::FrameBuffer;

pub struct Renderer {
    canvas: render::WindowCanvas,
    #[allow(dead_code)]
    creator: render::TextureCreator<video::WindowContext>,
    texture: render::Texture,
    viewport_rect: Rect,
    frame: u32,
    last_frame_ts: u64,
}

impl Renderer {
    pub fn build(
        sdl_video: &sdl2::VideoSubsystem,
        window_size: (u32, u32),
        screen_size: (u32, u32),
        viewport_offset: (u32, u32),
        viewport_size: (u32, u32),
        fullscreen: bool,
    ) -> Result<Renderer, String> {
        let mut builder = sdl_video.window("zinc64", window_size.0, window_size.1);
        builder.opengl();
        if fullscreen {
            builder.fullscreen();
        } else {
            builder.position_centered();
            builder.resizable();
        }
        let window = builder.build().unwrap();
        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .unwrap();
        let creator = canvas.texture_creator();
        let texture = creator
            .create_texture_streaming(
                pixels::PixelFormatEnum::ARGB8888,
                screen_size.0,
                screen_size.1,
            )
            .unwrap();
        let viewport_rect = Rect::new(
            viewport_offset.0 as i32,
            viewport_offset.1 as i32,
            viewport_size.0,
            viewport_size.1,
        );
        let renderer = Renderer {
            canvas,
            creator,
            texture,
            viewport_rect,
            frame: 0,
            last_frame_ts: 0,
        };
        Ok(renderer)
    }

    pub fn render(&mut self, frame_buffer: &FrameBuffer) -> Result<(), String> {
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
        self.frame = self.frame.wrapping_add(1);
        self.last_frame_ts = time::precise_time_ns();
        Ok(())
    }

    pub fn toggle_fullscreen(&mut self) {
        let window = self.canvas.window_mut();
        match window.fullscreen_state() {
            video::FullscreenType::Off => {
                window.set_fullscreen(video::FullscreenType::True).unwrap();
            }
            video::FullscreenType::True => {
                window.set_fullscreen(video::FullscreenType::Off).unwrap();
            }
            _ => panic!("invalid fullscreen mode"),
        }
    }
}
