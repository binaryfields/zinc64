// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::path::Path;
use std::ptr;

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
    glyphs: Vec<u8>,
    size: (u32, u32),
    offset: usize,
    bytes_per_glyph: usize,
}

impl Font {
    #[allow(unused)]
    pub fn load(path: &Path, size: (u32, u32)) -> Result<Font, String> {
        assert_eq!(size.0 % 8, 0);
        let glyphs = reader::read_file(path)?;
        Ok(Font {
            glyphs,
            size,
            offset: 0,
            bytes_per_glyph: (size.0 / 8 * size.1) as usize,
        })
    }

    pub fn load_psf(path: &Path) -> Result<Font, String> {
        let data = reader::read_file(path)?;
        let header: PcfHeader = unsafe { ptr::read(data.as_ptr() as *const _) };
        Ok(Font {
            glyphs: data,
            size: (header.width, header.height),
            offset: header.header_size as usize,
            bytes_per_glyph: header.bytes_per_glyph as usize,
        })
    }

    pub fn get_glyph(&self, ch: u8) -> &[u8] {
        let offset = self.offset + ch as usize * self.bytes_per_glyph;
        &self.glyphs[offset..(offset + self.bytes_per_glyph as usize)]
    }

    pub fn get_height(&self) -> u32 {
        self.size.1
    }

    pub fn get_width(&self) -> u32 {
        self.size.0
    }
}
