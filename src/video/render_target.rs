/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::mem;
use video::color::Color;
use util::Dimension;

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
