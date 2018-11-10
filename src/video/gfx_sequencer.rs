// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

#[derive(Copy, Clone)]
pub enum Mode {
    // (ECM/BMM/MCM=0/0/0)
    Text = 0x00,
    // (ECM/BMM/MCM=0/0/1)
    McText = 0x01,
    // (ECM/BMM/MCM=0/1/0)
    Bitmap = 0x02,
    // (ECM/BMM/MCM=0/1/1)
    McBitmap = 0x03,
    // (ECM/BMM/MCM=1/0/0)
    EcmText = 0x04,
    // (ECM/BMM/MCM=1/0/1)
    InvalidText = 0x05,
    // (ECM/BMM/MCM=1/1/0)
    InvalidBitmap1 = 0x06,
    // (ECM/BMM/MCM=1/1/1)
    InvalidBitmap2 = 0x07,
}

impl Mode {
    pub fn from(mode: u8) -> Mode {
        match mode {
            0x00 => Mode::Text,
            0x01 => Mode::McText,
            0x02 => Mode::Bitmap,
            0x03 => Mode::McBitmap,
            0x04 => Mode::EcmText,
            0x05 => Mode::InvalidText,
            0x06 => Mode::InvalidBitmap1,
            0x07 => Mode::InvalidBitmap2,
            _ => panic!("invalid mode {}", mode),
        }
    }

    pub fn value(self) -> u8 {
        self as u8
    }
}

pub struct Config {
    pub mode: Mode,
    pub bg_color: [u8; 4],
}

impl Config {
    pub fn new() -> Self {
        Config {
            mode: Mode::Text,
            bg_color: [0; 4],
        }
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Text;
        self.bg_color = [0x06, 0, 0, 0];
    }
}

pub struct GfxSequencer {
    pub config: Config,
    c_data: u8,
    c_color: u8,
    g_data: u8,
    data: u8,
    mc_cycle: bool,
    output: (u8, bool),
}

impl GfxSequencer {
    pub fn new() -> Self {
        GfxSequencer {
            config: Config::new(),
            c_data: 0,
            c_color: 0,
            g_data: 0,
            data: 0,
            mc_cycle: false,
            output: (0, false),
        }
    }

    pub fn set_data(&mut self, c_data: u8, c_color: u8, g_data: u8) {
        self.c_data = c_data;
        self.c_color = c_color;
        self.g_data = g_data;
    }

    pub fn clock(&mut self) {
        if !self.mc_cycle {
            match self.config.mode {
                Mode::Text => self.output = self.output_text(),
                Mode::McText => {
                    self.mc_cycle = self.c_color.get_bit(3);
                    self.output = self.output_text_mc()
                }
                Mode::Bitmap => self.output = self.output_bitmap(),
                Mode::McBitmap => {
                    self.mc_cycle = true;
                    self.output = self.output_bitmap_mc()
                }
                Mode::EcmText => self.output = self.output_text_ecm(),
                Mode::InvalidText => {
                    panic!("unsupported graphics mode {}", self.config.mode.value())
                }
                Mode::InvalidBitmap1 => self.output = (0, false),
                Mode::InvalidBitmap2 => self.output = (0, false),
            };
            self.data <<= if !self.mc_cycle { 1 } else { 2 };
        } else {
            self.mc_cycle = false;
        }
    }

    pub fn load_data(&mut self) {
        self.data = self.g_data;
    }

    pub fn output(&self) -> (u8, bool) {
        self.output
    }

    pub fn reset(&mut self) {
        self.config.reset();
        self.c_data = 0;
        self.c_color = 0;
        self.g_data = 0;
        self.data = 0;
        self.mc_cycle = false;
        self.output = (0, false);
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Color from bits 0-3 of c-data    |
     | "1": Color from bits 4-7 of c-data    |
     +---------------------------------------+
    */

    fn output_bitmap(&self) -> (u8, bool) {
        if self.data.get_bit(7) {
            (self.c_data >> 4, true)
        } else {
            (self.c_data & 0x0f, false)
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      |
     | "01": Color from bits 4-7 of c-data   |
     | "10": Color from bits 0-3 of c-data   |
     | "11": Color from bits 8-11 of c-data  |
     +---------------------------------------+
    */

    fn output_bitmap_mc(&self) -> (u8, bool) {
        match self.data >> 6 {
            0 => (self.config.bg_color[0], false),
            1 => (self.c_data >> 4, false),
            2 => (self.c_data & 0x0f, true),
            3 => (self.c_color, true),
            _ => panic!("invalid color source {}", self.data >> 6),
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    fn output_text(&self) -> (u8, bool) {
        if self.data.get_bit(7) {
            (self.c_color, true)
        } else {
            (self.config.bg_color[0], false)
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Depending on bits 6/7 of c-data  |
     |      00: Background color 0 ($d021)   |
     |      01: Background color 1 ($d022)   |
     |      10: Background color 2 ($d023)   |
     |      11: Background color 3 ($d024)   |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    fn output_text_ecm(&self) -> (u8, bool) {
        if self.data.get_bit(7) {
            (self.c_color, true)
        } else {
            (self.config.bg_color[(self.c_data >> 6) as usize], false)
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       | MC flag = 0
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-10 of c-data   |
     +---------------------------------------+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      | MC flag = 1
     | "01": Background color 1 ($d022)      |
     | "10": Background color 2 ($d023)      |
     | "11": Color from bits 8-10 of c-data  |
     +---------------------------------------+
    */

    fn output_text_mc(&self) -> (u8, bool) {
        if self.c_color.get_bit(3) {
            match self.data >> 6 {
                0 => (self.config.bg_color[0], false),
                1 => (self.config.bg_color[1], false),
                2 => (self.config.bg_color[2], true),
                3 => (self.c_color & 0x07, true),
                _ => panic!("invalid color source {}", self.data >> 6),
            }
        } else {
            self.output_text()
        }
    }
}
