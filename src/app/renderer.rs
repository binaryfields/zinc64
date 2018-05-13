/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::result::Result;

use sdl2;
use sdl2::pixels;
use sdl2::render;
use sdl2::video;
use time;
use zinc64::core::FrameBuffer;
use zinc64::core::geo;

pub struct Renderer {
    canvas: render::WindowCanvas,
    #[allow(dead_code)]
    creator: render::TextureCreator<video::WindowContext>,
    texture: render::Texture,
    frame: u32,
    last_frame_ts: u64,
}

impl Renderer {
    pub fn new(
        sdl_video: &sdl2::VideoSubsystem,
        window_size: geo::Size,
        screen_size: geo::Size,
        fullscreen: bool,
    ) -> Result<Renderer, String> {
        let mut builder = sdl_video.window("zinc64", window_size.width, window_size.height);
        builder.opengl();
        if fullscreen {
            builder.fullscreen();
        } else {
            builder.position_centered();
            builder.resizable();
        }
        let window = builder.build().unwrap();
        let canvas = window.into_canvas().build().unwrap();
        let creator = canvas.texture_creator();
        let texture = creator
            .create_texture_streaming(
                pixels::PixelFormatEnum::ARGB8888,
                screen_size.width,
                screen_size.height,
            )
            .unwrap();
        let renderer = Renderer {
            canvas,
            creator,
            texture,
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
        self.canvas.copy(&self.texture, None, None)?;
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
