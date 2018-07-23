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

use std::cell::RefCell;
use std::rc::Rc;

use core::{IoPort, Ram, Rom};

pub struct VicMemory {
    charset: Rc<RefCell<Rom>>,
    cia_2_port_a: Rc<RefCell<IoPort>>,
    ram: Rc<RefCell<Ram>>,
}

impl VicMemory {
    pub fn new(
        charset: Rc<RefCell<Rom>>,
        cia_2_port_a: Rc<RefCell<IoPort>>,
        ram: Rc<RefCell<Ram>>,
    ) -> VicMemory {
        VicMemory {
            charset,
            cia_2_port_a,
            ram,
        }
    }

    #[inline]
    pub fn read(&self, address: u16) -> u8 {
        let cia2_port_a = self.cia_2_port_a.borrow().get_value();
        let full_address = ((!cia2_port_a & 0x03) as u16) << 14 | address;
        let zone = full_address >> 12;
        match zone {
            0x01 => self.charset.borrow().read(full_address - 0x1000),
            0x09 => self.charset.borrow().read(full_address - 0x9000),
            _ => self.ram.borrow().read(full_address),
        }
    }

    #[inline]
    pub fn write(&mut self, _address: u16, _value: u8) {
        panic!("writes by vic are not supported")
    }
}
