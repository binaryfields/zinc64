// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::factory::AddressableFaded;
use crate::util::{IoPort, Shared};
use bit_field::BitField;

use crate::device::cartridge::Cartridge;

// DEFERRED device: expansion port test cases

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

pub struct ExpansionPort {
    cartridge: Option<Cartridge>,
    // I/O
    io_line: Shared<IoPort>,
}

impl ExpansionPort {
    pub fn new(io_line: Shared<IoPort>) -> Self {
        Self {
            cartridge: None,
            io_line,
        }
    }

    pub fn attach(&mut self, mut cartridge: Cartridge) {
        let io_line_clone = self.io_line.clone();
        cartridge.set_io_observer(Some(Box::new(move |config| {
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), config.game);
            io_value.set_bit(IoLine::Exrom.value(), config.exrom);
            io_line_clone.borrow_mut().set_value(io_value);
        })));
        self.cartridge = Some(cartridge);
    }

    pub fn detach(&mut self) {
        let mut cartridge = self.cartridge.take();
        if let Some(ref mut cartridge) = cartridge {
            cartridge.set_io_observer(None);
        }
    }

    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
        } else {
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), true);
            io_value.set_bit(IoLine::Exrom.value(), true);
            self.io_line.borrow_mut().set_value(io_value);
        }
    }
}

impl AddressableFaded for ExpansionPort {
    fn read(&mut self, address: u16) -> Option<u8> {
        self.cartridge.as_mut().and_then(|crt| crt.read(address))
    }

    fn write(&mut self, address: u16, value: u8) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.write(address, value)
        }
    }
}
