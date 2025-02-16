// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

use crate::peripheral::Cartridge;
use crate::peripheral::cartridge::IoConfig;

// DEFERRED device: expansion port test cases

pub struct ExpansionPort {
    cartridge: Option<Cartridge>,
    // I/O
    io_lines: u8,
}

impl ExpansionPort {
    pub fn new() -> Self {
        Self {
            cartridge: None,
            io_lines: 0,
        }
    }

    pub fn io_lines(&self) -> u8 {
        self.io_lines
    }

    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
            self.io_lines = io_lines(cartridge.io_config());
        } else {
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), true);
            io_value.set_bit(IoLine::Exrom.value(), true);
            self.io_lines = io_value;
        }
    }

    pub fn attach(&mut self, cartridge: Cartridge) {
        self.cartridge = Some(cartridge);
    }

    pub fn detach(&mut self) {
        self.cartridge = None;
    }

    pub fn read(&mut self, address: u16) -> Option<u8> {
        self.cartridge.as_mut().and_then(|crt| crt.read(address))
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        if let Some(ref mut cartridge) = self.cartridge
            && cartridge.write(address, value)
        {
            self.io_lines = io_lines(cartridge.io_config());
            return true;
        }
        false
    }
}

impl Default for ExpansionPort {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
enum IoLine {
    Game = 3,
    Exrom = 4,
}

impl IoLine {
    pub fn value(self) -> usize {
        self as usize
    }
}

fn io_lines(config: &IoConfig) -> u8 {
    let mut io_value = 0u8;
    io_value.set_bit(IoLine::Game.value(), config.game);
    io_value.set_bit(IoLine::Exrom.value(), config.exrom);
    io_value
}
