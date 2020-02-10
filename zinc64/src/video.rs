// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::transmute_ptr_to_ptr))]

use std::rc::Rc;
use std::result::Result;

use cgmath;
use cgmath::num_traits::zero;
use cgmath::{vec2, Vector2};
use zinc64_core::{Shared, VideoOutput};

use crate::app::AppState;
use crate::framework::Context;
use crate::gfx::{gl, sprite, Color, Rect, RectI};

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

    pub fn get_pixel_data(&self) -> &[u8] {
        unsafe {
            let len = self.pixels.len() * core::mem::size_of::<u32>();
            core::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, len)
        }
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
    // Dependencies
    video_buffer: Shared<VideoBuffer>,
    // Resources
    batch: sprite::Batch,
    texture: Rc<gl::Texture>,
}

impl VideoRenderer {
    pub fn build(ctx: &mut Context, state: &mut AppState) -> Result<VideoRenderer, String> {
        let screen_size = state.c64.get_config().model.frame_buffer_size;
        let viewport_offset = state.c64.get_config().model.viewport_offset;
        let viewport_size = state.c64.get_config().model.viewport_size;
        let video_buffer = state.video_buffer.clone();
        let viewport = Rect::new(
            vec2(viewport_offset.0 as f32, viewport_offset.1 as f32),
            vec2(viewport_size.0 as f32, viewport_size.1 as f32),
        );
        let window_size = ctx.platform.windowed_context.window().inner_size();
        info!("Renderer viewport {:?}", viewport);
        let gl = &mut ctx.platform.gl;
        let texture_size = vec2(screen_size.0, screen_size.1).cast::<i32>().unwrap();
        let texture = Rc::new(gl.create_texture(texture_size)?);
        let mut batch = sprite::Batch::new(gl, 1)?;
        batch.set_projection(gl, viewport, false);
        batch.set_viewport(
            gl,
            RectI::new(
                zero(),
                Vector2::new(window_size.width as i32, window_size.height as i32),
            ),
        );
        let renderer = VideoRenderer {
            video_buffer,
            batch,
            texture,
        };
        Ok(renderer)
    }

    pub fn update_viewport(&mut self, ctx: &mut Context, width: i32, height: i32) {
        self.batch.set_viewport(
            &mut ctx.platform.gl,
            RectI::new(zero(), vec2(width, height)),
        );
    }

    pub fn render(&mut self, ctx: &mut Context) -> Result<(), String> {
        let gl = &mut ctx.platform.gl;
        let tex_size = self.texture.size.cast::<f32>().unwrap();
        gl.set_texture_data(&self.texture, self.video_buffer.borrow().get_pixel_data());
        gl.clear(Color::BLACK);

        self.batch.begin(gl, Some(self.texture.clone()));
        self.batch.push(
            gl,
            Rect::from_points(zero(), tex_size),
            Rect::from_points(zero(), vec2(1.0, 1.0)),
            Color::WHITE,
        );
        self.batch.end(gl);

        Ok(())
    }
}
