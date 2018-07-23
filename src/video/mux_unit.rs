// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

const PRIO_SCREEN_BORDER: u8 = 0;
const PRIO_FG_SPRITE: u8 = 1;
const PRIO_FG_GRAPHICS: u8 = 2;
const PRIO_BG_SPRITE: u8 = 3;
const PRIO_BG_GRAPHICS: u8 = 4;

pub struct MuxUnit {
    pub data_priority: [bool; 8],
    pub mb_collision: u8,
    pub mb_interrupt: bool,
    pub mm_collision: u8,
    pub mm_interrupt: bool,
    output: u8,
    output_priority: u8,
}

impl MuxUnit {
    pub fn new() -> Self {
        MuxUnit {
            data_priority: [false; 8],
            mb_collision: 0,
            mb_interrupt: false,
            mm_collision: 0,
            mm_interrupt: false,
            output: 0,
            output_priority: 0,
        }
    }

    pub fn compute_collisions(&mut self, sprite_output: &[Option<u8>; 8]) {
        let fg_graphics = self.output_priority == PRIO_FG_GRAPHICS;
        let mut mb_collision = self.mb_collision;
        let mut mm_collision = self.mm_collision;
        let mut mm_count = 0u8;
        for i in 0..8 {
            if sprite_output[i].is_some() {
                if fg_graphics {
                    mb_collision.set_bit(i, true);
                }
                mm_collision.set_bit(i, true);
                mm_count += 1;
            }
        }
        self.mb_interrupt = self.mb_collision == 0 && mb_collision != 0;
        self.mb_collision |= mb_collision;
        if mm_count >= 2 {
            self.mm_interrupt = self.mm_collision == 0 && mm_collision != 0;
            self.mm_collision |= mm_collision;
        }
    }

    pub fn feed_border(&mut self, border_output: u8) {
        self.output_pixel(border_output, PRIO_SCREEN_BORDER);
    }

    pub fn feed_graphics(&mut self, gfx_output: (u8, bool)) {
        if gfx_output.1 {
            self.output_pixel(gfx_output.0, PRIO_FG_GRAPHICS);
        } else {
            self.output_pixel(gfx_output.0, PRIO_BG_GRAPHICS);
        }
    }

    pub fn feed_sprites(&mut self, sprite_output: &[Option<u8>; 8]) {
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

    pub fn output(&self) -> u8 {
        self.output
    }

    pub fn reset(&mut self) {
        self.data_priority = [false; 8];
        self.mb_collision = 0;
        self.mb_interrupt = false;
        self.mm_collision = 0;
        self.mm_interrupt = false;
        self.output = 0;
        self.output_priority = 0;
    }

    fn output_pixel(&mut self, pixel: u8, priority: u8) {
        self.output = pixel;
        self.output_priority = priority;
    }

    fn output_sprite_pixel(&mut self, pixel: u8, priority: u8) {
        if priority < self.output_priority {
            self.output = pixel;
            self.output_priority = priority;
        }
    }
}
