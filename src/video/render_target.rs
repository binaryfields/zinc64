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

use video::color::Color;
use util::Dimension;

pub struct RenderTarget {
    dim: Dimension,
    pixels: Vec<u32>
}

impl RenderTarget {
    pub fn new(dim: Dimension) -> RenderTarget {
        RenderTarget {
            dim: dim,
            pixels: vec![0; (dim.width as usize) * (dim.height as usize)],
        }
    }

    pub fn dimension(&self) -> Dimension {
        self.dim
    }

    pub fn read(&self, x: u16, y: u16) -> u32 {
        let index = self.index(x, y);
        self.pixels[index]
    }

    pub fn write(&mut self, x: u16, y: u16, color: u8) {
        let index = self.index(x, y);
        self.pixels[index] = Color::from(color).rgb();
    }

    fn index(&self, x: u16, y: u16) -> usize {
        (y as usize) * (self.dim.width as usize) + (x as usize)
    }
}
