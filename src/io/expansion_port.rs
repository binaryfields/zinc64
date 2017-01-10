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

use device::Cartridge;
use mem::{Addressable, Memory};

pub struct ExpansionPortIo {
    pub game: bool,
    pub exrom: bool,
}

impl ExpansionPortIo {
    pub fn new() -> ExpansionPortIo {
        ExpansionPortIo {
            game: true,
            exrom: true,
        }
    }

    pub fn reset(&mut self) {
        self.game = true;
        self.exrom = true;
    }

    pub fn update(&mut self, game: bool, exrom: bool) {
        self.game = game;
        self.exrom = exrom;
    }
}

pub struct ExpansionPort {
    mem: Rc<RefCell<Memory>>,
    cartridge: Option<Cartridge>,
    io: Rc<RefCell<ExpansionPortIo>>,
}

impl ExpansionPort {
    pub fn new(expansion_port_io: Rc<RefCell<ExpansionPortIo>>,
               mem: Rc<RefCell<Memory>>) -> ExpansionPort {
        ExpansionPort {
            mem: mem,
            cartridge: None,
            io: expansion_port_io,
        }
    }

    pub fn attach(&mut self, cartridge: Cartridge) {
        self.io.borrow_mut().update(cartridge.get_game(), cartridge.get_exrom());
        self.cartridge = Some(cartridge);
        self.mem.borrow_mut().switch_banks();
    }

    pub fn detach(&mut self) {
        if self.cartridge.is_some() {
            self.cartridge = None;
            self.io.borrow_mut().reset();
            self.mem.borrow_mut().switch_banks();
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
            self.io.borrow_mut().update(cartridge.get_game(), cartridge.get_exrom());
            self.mem.borrow_mut().switch_banks();
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

    #[allow(unused_variables)]
    fn write(&mut self, address: u16, value: u8) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.write(address, value)
        }
    }
}
