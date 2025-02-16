// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::chip::{Ram, Rom};
use crate::signal::{IrqLine, Pin};

pub struct VicBus<'a> {
    pub ram: &'a Ram,
    pub charset: &'a Rom,
    pub color_ram: &'a Ram,
    pub bank: u16,
    pub ba: &'a mut Pin,
    pub irq: &'a mut IrqLine,
}

impl VicBus<'_> {
    pub fn read(&self, address: u16) -> u8 {
        let full_address = self.bank | address;
        let zone = full_address >> 12;
        match zone {
            0x01 => self.charset.read(full_address - 0x1000),
            0x09 => self.charset.read(full_address - 0x9000),
            _ => self.ram.read(full_address),
        }
    }
}
