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

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Standard = 0,
    Multicolor = 1,
}

pub struct Config {
    pub color: u8,
    pub data_priority: bool,
    pub enabled: bool,
    pub expand_x: bool,
    pub expand_y: bool,
    pub mode: Mode,
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
            mode: Mode::Standard,
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
        self.mode = Mode::Standard;
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
    delay_cycles: u8,
    pub display: bool,
    pub dma: bool,
    pub expansion_ff: bool,
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
            delay_cycles: 0,
            display: false,
            dma: false,
            expansion_ff: true,
            output: None,
        }
    }

    pub fn set_data(&mut self, byte: usize, value: u8) {
        match byte {
            0 => {
                self.data.set_bits(24..32, value as u32);
            }
            1 => {
                self.data.set_bits(16..24, value as u32);
            }
            2 => {
                self.data.set_bits(8..16, value as u32);
            }
            _ => panic!("invalid sprite data index {}", byte),
        }
    }

    #[inline]
    pub fn clock(&mut self, x: u16) {
        if self.display {
            if self.delay_cycles == 0 {
                if x == self.config.x_screen && self.counter == 0 {
                    self.counter = 0xffffff00;
                }
                if x >= self.config.x_screen && self.counter != 0 {
                    match self.config.mode {
                        Mode::Standard => {
                            self.output = self.output_pixel();
                            self.counter = self.counter << 1;
                            self.data = self.data << 1;
                            if self.config.expand_x {
                                self.delay_cycles = 0b0001;
                            }
                        },
                        Mode::Multicolor => {
                            self.output = self.output_mc_pixel();
                            self.counter = self.counter << 2;
                            self.data = self.data << 2;
                            self.delay_cycles = if self.config.expand_x {
                                0b0111
                            } else {
                                0b0001
                            }
                        }
                    }
                } else {
                    self.output = None;
                }
            } else {
                self.delay_cycles = self.delay_cycles >> 1;
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
        self.delay_cycles = 0;
        self.display = false;
        self.dma = false;
        self.expansion_ff = true;
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
