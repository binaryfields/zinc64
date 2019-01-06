// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use bit_field::BitField;

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Standard = 0,
    Multicolor = 1,
}

pub struct Config {
    pub mode: Mode,
    pub color: u8,
    pub enabled: bool,
    pub expand_x: bool,
    pub expand_y: bool,
    pub multicolor: [u8; 2],
    pub x: u16,
    pub x_screen: u16,
    pub y: u8,
}

impl Config {
    pub fn new() -> Self {
        Config {
            mode: Mode::Standard,
            color: 0,
            enabled: false,
            expand_x: false,
            expand_y: false,
            multicolor: [0; 2],
            x: 0,
            x_screen: 0,
            y: 0,
        }
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Standard;
        self.color = 0;
        self.enabled = false;
        self.expand_x = false;
        self.expand_y = false;
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
    pub expansion_flop: bool,
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
            expansion_flop: true,
            output: None,
        }
    }

    pub fn set_data(&mut self, byte: usize, value: u8) {
        self.counter = 0;
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

    pub fn clock(&mut self, x: u16) {
        if self.display {
            if self.delay_cycles == 0 {
                if x == self.config.x_screen && self.counter == 0 {
                    self.counter = 0xffff_ff00;
                }
                if x >= self.config.x_screen && self.counter != 0 {
                    match self.config.mode {
                        Mode::Standard => {
                            self.output = self.output_pixel();
                            self.counter <<= 1;
                            self.data <<= 1;
                            if self.config.expand_x {
                                self.delay_cycles = 0b0001;
                            }
                        }
                        Mode::Multicolor => {
                            self.output = self.output_mc_pixel();
                            self.counter <<= 2;
                            self.data <<= 2;
                            self.delay_cycles = if self.config.expand_x { 0b0111 } else { 0b0001 }
                        }
                    }
                } else {
                    self.output = None;
                }
            } else {
                self.delay_cycles >>= 1;
            }
        }
    }

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
        self.expansion_flop = true;
        self.output = None;
    }

    fn output_pixel(&self) -> Option<u8> {
        if self.data.get_bit(31) {
            Some(self.config.color)
        } else {
            None
        }
    }

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
