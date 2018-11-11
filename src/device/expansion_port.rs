// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::RefCell;
use std::rc::Rc;

use bit_field::BitField;
use crate::core::{Addressable, IoPort};

use super::cartridge::Cartridge;

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
    io_line: Rc<RefCell<IoPort>>,
}

impl ExpansionPort {
    pub fn new(io_line: Rc<RefCell<IoPort>>) -> Self {
        Self {
            cartridge: None,
            io_line,
        }
    }

    pub fn attach(&mut self, cartridge: Cartridge) {
        let mut io_value = 0u8;
        io_value.set_bit(IoLine::Game.value(), cartridge.get_game());
        io_value.set_bit(IoLine::Exrom.value(), cartridge.get_exrom());
        self.io_line.borrow_mut().set_value(io_value);
        self.cartridge = Some(cartridge);
    }

    pub fn detach(&mut self) {
        if self.cartridge.is_some() {
            self.cartridge = None;
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), true);
            io_value.set_bit(IoLine::Exrom.value(), true);
            self.io_line.borrow_mut().set_value(io_value);
        }
    }

    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), cartridge.get_game());
            io_value.set_bit(IoLine::Exrom.value(), cartridge.get_exrom());
            self.io_line.borrow_mut().set_value(io_value);
        } else {
            let mut io_value = 0u8;
            io_value.set_bit(IoLine::Game.value(), true);
            io_value.set_bit(IoLine::Exrom.value(), true);
            self.io_line.borrow_mut().set_value(io_value);
        }
    }
}

impl Addressable for ExpansionPort {
    fn read(&self, address: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read(address)
        } else {
            0
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.write(address, value)
        }
    }
}
