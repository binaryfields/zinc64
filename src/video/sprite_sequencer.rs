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

use bit_field::BitField;

pub struct Config {
    pub color: u8,
    pub data_priority: bool,
    pub enabled: bool,
    pub expand_x: bool,
    pub expand_y: bool,
    pub mode: bool,
    pub multicolor: [u8; 2],
    pub x: u16,
    pub x_screen: u16,
    pub y: u8,
}

impl Config {
    pub fn new() -> Self {
        Config {
            color: 0,
            data_priority: false,
            enabled: false,
            expand_x: false,
            expand_y: false,
            mode: false,
            multicolor: [0; 2],
            x: 0,
            x_screen: 0,
            y: 0,
        }
    }

    pub fn reset(&mut self) {
        self.color = 0;
        self.data_priority = false;
        self.enabled = false;
        self.expand_x = false;
        self.expand_y = false;
        self.mode = false;
        self.multicolor = [0; 2];
        self.x = 0;
        self.x_screen = 0;
        self.y = 0;
    }
}

pub struct SpriteSequencer {
    // Configuration
    pub config: Config,
    // Runtime State
    counter: u32,
    data: u32,
    pub display: bool,
    pub dma: bool,
    pub expansion_ff: bool,
    mc_cycle: bool,
    output: Option<u8>,
}

impl SpriteSequencer {
    pub fn new() -> Self {
        SpriteSequencer {
            // Configuration
            config: Config::new(),
            // Runtime State
            counter: 0,
            data: 0,
            display: false,
            dma: false,
            expansion_ff: true,
            mc_cycle: false,
            output: None,
        }
    }

    pub fn set_data(&mut self, byte: usize, value: u8) {
        match byte {
            0 => {
                self.data.set_bits(24..32, value as u32);
            },
            1 => {
                self.data.set_bits(16..24, value as u32);
            },
            2 => {
                self.data.set_bits(8..16, value as u32);
            },
            _ => panic!("invalid sprite data index {}", byte),
        }
    }

    #[inline]
    pub fn clock(&mut self, x: u16) {
        if self.display {
            if x == self.config.x_screen && self.counter == 0 {
                self.counter = 0xffffff00;
            }
            if x >= self.config.x_screen && self.counter != 0 {
                if !self.mc_cycle {
                    self.output = if !self.config.mode {
                        self.output_pixel()
                    } else {
                        self.mc_cycle = true;
                        self.output_mc_pixel()
                    };
                    if !self.mc_cycle {
                        self.data = self.data << 1;
                        self.counter = self.counter << 1;
                    } else {
                        self.data = self.data << 2;
                        self.counter = self.counter << 2;
                    }
                } else {
                    self.mc_cycle = false;
                }
            }
        }
    }

    #[inline]
    pub fn output(&self) -> Option<u8> {
        self.output
    }

    pub fn reset(&mut self) {
        // Configuration
        self.config.reset();
        // Runtime State
        self.counter = 0;
        self.data = 0;
        self.display = false;
        self.dma = false;
        self.expansion_ff = true;
        self.mc_cycle = false;
        self.output = None;
    }

    #[inline]
    fn output_pixel(&self) -> Option<u8> {
        if self.data.get_bit(31) {
            Some(self.config.color)
        } else {
            None
        }
    }

    #[inline]
    fn output_mc_pixel(&self) -> Option<u8> {
        match self.data >> 30 {
            0 => None,
            1 => Some(self.config.multicolor[0]),
            2 => Some(self.config.color),
            3 => Some(self.config.multicolor[1]),
            _ => panic!("invalid sprite color source {}", self.data >> 30),
        }
    }
}
