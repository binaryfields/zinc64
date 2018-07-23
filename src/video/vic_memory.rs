// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use core::{Ram, Rom};

pub struct VicMemory {
    base_address: Rc<Cell<u16>>,
    charset: Rc<RefCell<Rom>>,
    ram: Rc<RefCell<Ram>>,
}

impl VicMemory {
    pub fn new(
        base_address: Rc<Cell<u16>>,
        charset: Rc<RefCell<Rom>>,
        ram: Rc<RefCell<Ram>>,
    ) -> VicMemory {
        VicMemory {
            base_address,
            charset,
            ram,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        let full_address =  self.base_address.get() | address;
        let zone = full_address >> 12;
        match zone {
            0x01 => self.charset.borrow().read(full_address - 0x1000),
            0x09 => self.charset.borrow().read(full_address - 0x9000),
            _ => self.ram.borrow().read(full_address),
        }
    }
}
