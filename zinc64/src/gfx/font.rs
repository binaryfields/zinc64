// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::path::Path;
use std::ptr;

use bit_field::BitField;
use cgmath::prelude::*;
use cgmath::{vec2, Vector2};

use crate::gfx::{Color, Rect};
use crate::util::reader;

#[repr(C, packed)]
struct PcfHeader {
    pub magic: u32,
    pub version: u32,
    pub header_size: u32,
    pub flags: u32,
    pub num_glyph: u32,
    pub bytes_per_glyph: u32,
    pub height: u32,
    pub width: u32,
    pub glyphs: u8,
}

pub struct Font {
    size: Vector2<u32>,
    num_glyph: u32,
    glyphs: Vec<u8>,
    offset: usize,
    bytes_per_glyph: usize,
    tex_glyph_size: Vector2<f32>,
    tex_size: Vector2<f32>,
}

impl Font {
    pub fn load_psf(path: &Path) -> Result<Font, String> {
        let data = reader::read_file(path)?;
        let header: PcfHeader = unsafe { ptr::read(data.as_ptr() as *const _) };
        let size = Vector2::new(header.width, header.height);
        let tex_size = Vector2::new(header.num_glyph * header.width, header.height)
            .cast::<f32>()
            .unwrap();
        let tex_glyph_size = size.cast::<f32>().unwrap().div_element_wise(tex_size);
        Ok(Font {
            num_glyph: header.num_glyph as u32,
            size,
            glyphs: data,
            offset: header.header_size as usize,
            bytes_per_glyph: header.bytes_per_glyph as usize,
            tex_glyph_size,
            tex_size,
        })
    }

    pub fn get_glyph(&self, ch: u32) -> &[u8] {
        let offset = self.offset + ch as usize * self.bytes_per_glyph;
        &self.glyphs[offset..(offset + self.bytes_per_glyph as usize)]
    }

    pub fn get_glypth_count(&self) -> u32 {
        self.num_glyph
    }

    pub fn get_size(&self) -> Vector2<u32> {
        self.size
    }

    pub fn get_height(&self) -> u32 {
        self.size.y
    }

    pub fn get_width(&self) -> u32 {
        self.size.x
    }

    pub fn get_tex_coords(&self, ch: u32) -> Rect {
        let origin = Vector2::new((ch * self.size.x) as f32, self.size.y as f32);
        Rect::new(
            origin.div_element_wise(self.tex_size),
            vec2(self.tex_glyph_size.x, -self.tex_glyph_size.y),
        )
    }

    pub fn as_rgba(&self) -> Vec<u32> {
        let color_1 = Color::WHITE.rgba();
        let color_0 = Color::TRANSPARENT.rgba();
        let stride = self.num_glyph * self.size.x;
        let mut buffer = vec![0; (stride * self.size.y) as usize];
        for ch in 0..self.num_glyph {
            let mut glyph = self.get_glyph(ch);
            let mut pos = 0usize;
            for y in 0..self.size.y {
                let offset = (y * stride + ch * self.size.x) as usize;
                for x in 0..self.size.x {
                    let pixel = glyph[0].get_bit(pos);
                    pos += 1;
                    if pos % 8 == 0 {
                        glyph = &glyph[1..];
                        pos = 0;
                    }
                    buffer[offset + (7 - x) as usize] = if pixel { color_1 } else { color_0 };
                }
            }
        }
        return buffer;
    }
}
