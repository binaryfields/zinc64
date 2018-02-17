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

use core::{Addressable, MemoryController, Ram, Rom};
use log::LogLevel;

use super::{Bank, Configuration, MemoryMap};

// Spec: COMMODORE 64 MEMORY MAPS p. 263
// Design:
//   Inspired by UAE memory address64k/bank concepts.
//   We define Addressable trait to represent a bank of memory and use memory configuration
//   based on zones that can be mapped to different banks. CPU uses IoPort @ 0x0001 to reconfigure
//   memory layout.

pub struct Memory {
    // Configuration
    map: MemoryMap,
    configuration: Configuration,
    // Addressable
    basic: Rc<RefCell<Rom>>,
    charset: Rc<RefCell<Rom>>,
    expansion_port: Rc<RefCell<Addressable>>,
    io: Box<Addressable>,
    kernal: Rc<RefCell<Rom>>,
    ram: Rc<RefCell<Ram>>,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum BaseAddr {
    Basic = 0xa000,
    Charset = 0xd000,
    Kernal = 0xe000,
}

impl BaseAddr {
    pub fn addr(&self) -> u16 {
        *self as u16
    }
}

impl Memory {
    pub fn new(
        expansion_port: Rc<RefCell<Addressable>>,
        io: Box<Addressable>,
        ram: Rc<RefCell<Ram>>,
        rom_basic: Rc<RefCell<Rom>>,
        rom_charset: Rc<RefCell<Rom>>,
        rom_kernal: Rc<RefCell<Rom>>,
    ) -> Memory {
        let map = MemoryMap::new();
        let configuration = map.get(1);
        Memory {
            map,
            configuration,
            basic: rom_basic,
            charset: rom_charset,
            expansion_port,
            io,
            kernal: rom_kernal,
            ram,
        }
    }
}

impl MemoryController for Memory {
    #[inline]
    fn switch_banks(&mut self, mode: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "mem::banks", "Switching to {}", mode);
        }
        self.configuration = self.map.get(mode);
    }

    // I/O

    #[inline]
    fn read(&self, address: u16) -> u8 {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.borrow().read(address),
            Bank::Basic => self.basic.borrow().read(address),
            Bank::Charset => self.charset
                .borrow()
                .read(address - BaseAddr::Charset.addr()),
            Bank::Kernal => self.kernal.borrow().read(address),
            Bank::RomL => self.expansion_port.borrow().read(address),
            Bank::RomH => self.expansion_port.borrow().read(address),
            Bank::Io => self.io.read(address),
            Bank::Disabled => 0,
        }
    }

    #[inline]
    fn write(&mut self, address: u16, value: u8) {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.borrow_mut().write(address, value),
            Bank::Basic => self.ram.borrow_mut().write(address, value),
            Bank::Charset => self.ram.borrow_mut().write(address, value),
            Bank::Kernal => self.ram.borrow_mut().write(address, value),
            Bank::RomL => self.ram.borrow_mut().write(address, value),
            Bank::RomH => self.ram.borrow_mut().write(address, value),
            Bank::Io => self.io.write(address, value),
            Bank::Disabled => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{Ram, Rom};

    impl Addressable for Ram {
        fn read(&self, address: u16) -> u8 {
            self.read(address)
        }

        fn write(&mut self, address: u16, value: u8) {
            self.write(address, value);
        }
    }

    fn setup_memory() -> Memory {
        let basic = Rc::new(RefCell::new(Rom::new(0x1000, BaseAddr::Basic.addr(), 0x10)));
        let charset = Rc::new(RefCell::new(Rom::new(0x1000, 0x0000, 0x11)));
        let kernal = Rc::new(RefCell::new(Rom::new(
            0x1000,
            BaseAddr::Kernal.addr(),
            0x12,
        )));
        let mut mmio = Box::new(Ram::new(0x10000));
        mmio.fill(0x22);
        let expansion_port = Rc::new(RefCell::new(Ram::new(0x1000)));
        expansion_port.borrow_mut().fill(0x33);
        let ram = Rc::new(RefCell::new(Ram::new(0x10000)));
        ram.borrow_mut().fill(0x44);
        Memory::new(expansion_port, mmio, ram, basic, charset, kernal)
    }

    #[test]
    fn read_basic() {
        let mut mem = setup_memory();
        mem.switch_banks(31);
        assert_eq!(0x10, mem.read(BaseAddr::Basic.addr()));
    }

    #[test]
    fn read_charset() {
        let mut mem = setup_memory();
        mem.switch_banks(27);
        assert_eq!(0x11, mem.read(BaseAddr::Charset.addr()));
    }

    #[test]
    fn read_io() {
        let mut mem = setup_memory();
        mem.switch_banks(31);
        assert_eq!(0x22, mem.read(0xd000));
    }

    #[test]
    fn read_kernal() {
        let mut mem = setup_memory();
        mem.switch_banks(31);
        assert_eq!(0x12, mem.read(BaseAddr::Kernal.addr()));
    }

    #[test]
    fn write_page_0() {
        let mut mem = setup_memory();
        mem.write(0x00f0, 0xff);
        assert_eq!(0xff, mem.ram.borrow().read(0x00f0));
    }

    #[test]
    fn write_page_1() {
        let mut mem = setup_memory();
        mem.write(0x0100, 0xff);
        assert_eq!(0xff, mem.ram.borrow().read(0x0100));
    }
}
