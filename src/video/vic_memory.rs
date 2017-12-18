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

use util::Addressable;

pub struct VicMemory {
    charset: Rc<RefCell<Addressable>>,
    ram: Rc<RefCell<Addressable>>,
    cia2_port_a: u8,
}

impl VicMemory {
    pub fn new(charset: Rc<RefCell<Addressable>>, ram: Rc<RefCell<Addressable>>) -> VicMemory {
        VicMemory {
            charset,
            ram,
            cia2_port_a: 0,
        }
    }

    pub fn set_cia_port_a(&mut self, value: u8) {
        self.cia2_port_a = value;
    }
}

impl Addressable for VicMemory {
    fn read(&self, address: u16) -> u8 {
        let full_address = ((!self.cia2_port_a & 0x03) as u16) << 14 | address;
        let zone = (full_address & 0xf000) >> 12;
        match zone {
            0x01 => self.charset.borrow().read(full_address - 0x1000),
            0x09 => self.charset.borrow().read(full_address - 0x9000),
            _ => self.ram.borrow().read(full_address),
        }
    }

    #[allow(unused_variables)]
    fn write(&mut self, address: u16, value: u8) {
        panic!("writes by vic are not supported")
    }
}
