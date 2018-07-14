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

const PRIO_SCREEN_BORDER: u8 = 0;
const PRIO_FG_SPRITE: u8 = 1;
const PRIO_FG_GRAPHICS: u8 = 2;
const PRIO_BG_SPRITE: u8 = 3;
const PRIO_BG_GRAPHICS: u8 = 4;

pub struct MuxUnit {
    pub data_priority: [bool; 8],
    output: u8,
    output_priority: u8,
}

impl MuxUnit {
    pub fn new() -> Self {
        MuxUnit {
            data_priority: [false; 8],
            output: 0,
            output_priority: 0,
        }
    }

    #[inline]
    pub fn feed_border(&mut self, border_output: u8) {
        self.output_pixel(border_output, PRIO_SCREEN_BORDER);
    }

    #[inline]
    pub fn feed_graphics(&mut self, gfx_output: (u8, bool)) {
        if gfx_output.1 {
            self.output_pixel(gfx_output.0, PRIO_FG_GRAPHICS);
        } else {
            self.output_pixel(gfx_output.0, PRIO_BG_GRAPHICS);
        }
    }

    #[inline]
    pub fn feed_sprites(&mut self, sprite_output: [Option<u8>; 8]) {
        for i in 0..8 {
            if let Some(output) = sprite_output[i] {
                if !self.data_priority[i] {
                    self.output_sprite_pixel(output, PRIO_FG_SPRITE);
                } else {
                    self.output_sprite_pixel(output, PRIO_BG_SPRITE);
                }
            }
        }
    }

    #[inline]
    pub fn output(&self) -> u8 {
        self.output
    }

    pub fn reset(&mut self) {
        self.data_priority = [false; 8];
        self.output = 0;
        self.output_priority = 0;
    }

    #[inline]
    fn output_pixel(&mut self, pixel: u8, priority: u8) {
        self.output = pixel;
        self.output_priority = priority;
    }

    #[inline]
    fn output_sprite_pixel(&mut self, pixel: u8, priority: u8) {
        if priority < self.output_priority {
            self.output = pixel;
            self.output_priority = priority;
        }
    }
}