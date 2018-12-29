// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::{Addressable, Chip, Ram};

pub struct Mmio {
    cia_1: Rc<RefCell<dyn Chip>>,
    cia_2: Rc<RefCell<dyn Chip>>,
    color_ram: Rc<RefCell<Ram>>,
    expansion_port: Rc<RefCell<dyn Addressable>>,
    sid: Rc<RefCell<dyn Chip>>,
    vic: Rc<RefCell<dyn Chip>>,
}

impl Mmio {
    pub fn new(
        cia_1: Rc<RefCell<dyn Chip>>,
        cia_2: Rc<RefCell<dyn Chip>>,
        color_ram: Rc<RefCell<Ram>>,
        expansion_port: Rc<RefCell<dyn Addressable>>,
        sid: Rc<RefCell<dyn Chip>>,
        vic: Rc<RefCell<dyn Chip>>,
    ) -> Self {
        Self {
            cia_1,
            cia_2,
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
            0xd800...0xdbff => self.color_ram.borrow().read(address - 0xd800),
            0xdc00...0xdcff => self.cia_1.borrow_mut().read((address & 0x000f) as u8),
            0xdd00...0xddff => self.cia_2.borrow_mut().read((address & 0x000f) as u8),
            0xde00...0xdfff => self.expansion_port.borrow().read(address),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xd000...0xd3ff => self.vic.borrow_mut().write((address & 0x003f) as u8, value),
            0xd400...0xd7ff => self.sid.borrow_mut().write((address & 0x001f) as u8, value),
            0xd800...0xdbff => self.color_ram.borrow_mut().write(address - 0xd800, value),
            0xdc00...0xdcff => self
                .cia_1
                .borrow_mut()
                .write((address & 0x000f) as u8, value),
            0xdd00...0xddff => self
                .cia_2
                .borrow_mut()
                .write((address & 0x000f) as u8, value),
            0xde00...0xdfff => self.expansion_port.borrow_mut().write(address, value),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }
}
