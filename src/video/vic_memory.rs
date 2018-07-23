// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

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

    pub fn write(&mut self, _address: u16, _value: u8) {
        panic!("writes by vic are not supported")
    }
}
