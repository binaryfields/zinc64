/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

use std::cell::RefCell;
use std::rc::Rc;

use bit_field::BitField;
use core::{Addressable, IoLine};

use super::cartridge::Cartridge;

// TODO device: expansion port test cases

pub struct ExpansionPort {
    cartridge: Option<Cartridge>,
    // I/O
    io_line: Rc<RefCell<IoLine>>,
}

impl ExpansionPort {
    pub fn new(io_line: Rc<RefCell<IoLine>>) -> ExpansionPort {
        ExpansionPort {
            cartridge: None,
            io_line,
        }
    }

    pub fn attach(&mut self, cartridge: Cartridge) {
        let mut io_value = 0u8;
        io_value.set_bit(3, cartridge.get_game());
        io_value.set_bit(4, cartridge.get_exrom());
        self.io_line.borrow_mut().set_value(io_value);
        self.cartridge = Some(cartridge);
    }

    pub fn detach(&mut self) {
        if self.cartridge.is_some() {
            self.cartridge = None;
            let mut io_value = 0u8;
            io_value.set_bit(3, true);
            io_value.set_bit(4, true);
            self.io_line.borrow_mut().set_value(io_value);
        }
    }

    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
            let mut io_value = 0u8;
            io_value.set_bit(3, cartridge.get_game());
            io_value.set_bit(4, cartridge.get_exrom());
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
