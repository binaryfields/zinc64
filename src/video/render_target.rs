/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

use std::mem;

use util::Dimension;

use super::color::Color;

const PIXEL_BYTES: usize = 4;

pub struct RenderTarget {
    dim: Dimension,
    pixels: Vec<u32>,
    sync: bool
}

impl RenderTarget {
    pub fn new(dim: Dimension) -> RenderTarget {
        RenderTarget {
            dim: dim,
            pixels: vec![0; (dim.width as usize) * (dim.height as usize)],
            sync: false,
        }
    }

    pub fn get_dimension(&self) -> Dimension { self.dim }
    pub fn get_pitch(&self) -> usize { self.dim.width as usize * PIXEL_BYTES }
    pub fn get_pixel_data(&self) -> &[u8] {
        unsafe { mem::transmute::<&[u32], &[u8]>(self.pixels.as_ref()) }
    }
    pub fn get_sync(&self) -> bool { self.sync }
    pub fn set_sync(&mut self, value: bool) { self.sync = value; }

    pub fn write(&mut self, x: u16, y: u16, color: u8) {
        let index = self.index(x, y);
        self.pixels[index] =  Color::from(color).rgb();
    }

    // -- Internal Ops

    fn index(&self, x: u16, y: u16) -> usize {
        (y as usize) * (self.dim.width as usize) + (x as usize)
    }
}
