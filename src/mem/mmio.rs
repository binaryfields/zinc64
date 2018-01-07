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

use core::{Addressable, Chip, Ram};

pub struct Mmio {
    cia1: Rc<RefCell<Chip>>,
    cia2: Rc<RefCell<Chip>>,
    color_ram: Rc<RefCell<Ram>>,
    expansion_port: Rc<RefCell<Addressable>>,
    sid: Rc<RefCell<Chip>>,
    vic: Rc<RefCell<Chip>>,
}

impl Mmio {
    pub fn new(
        cia1: Rc<RefCell<Chip>>,
        cia2: Rc<RefCell<Chip>>,
        color_ram: Rc<RefCell<Ram>>,
        expansion_port: Rc<RefCell<Addressable>>,
        sid: Rc<RefCell<Chip>>,
        vic: Rc<RefCell<Chip>>,
    ) -> Mmio {
        Mmio {
            cia1,
            cia2,
            color_ram,
            expansion_port,
            sid,
            vic,
        }
    }
}

impl Addressable for Mmio {
    fn read(&self, address: u16) -> u8 {
        match address {
            0xd000...0xd3ff => self.vic.borrow_mut().read((address & 0x003f) as u8),
            0xd400...0xd7ff => self.sid.borrow_mut().read((address & 0x001f) as u8),
            0xd800...0xdbff => self.color_ram.borrow().read((address - 0xd800)),
            0xdc00...0xdcff => self.cia1.borrow_mut().read((address & 0x000f) as u8),
            0xdd00...0xddff => self.cia2.borrow_mut().read((address & 0x000f) as u8),
            0xde00...0xdfff => self.expansion_port.borrow().read(address),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xd000...0xd3ff => self.vic.borrow_mut().write((address & 0x003f) as u8, value),
            0xd400...0xd7ff => self.sid.borrow_mut().write((address & 0x001f) as u8, value),
            0xd800...0xdbff => self.color_ram.borrow_mut().write(address - 0xd800, value),
            0xdc00...0xdcff => self.cia1
                .borrow_mut()
                .write((address & 0x000f) as u8, value),
            0xdd00...0xddff => self.cia2
                .borrow_mut()
                .write((address & 0x000f) as u8, value),
            0xde00...0xdfff => self.expansion_port.borrow_mut().write(address, value),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }
}
