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

use std::io;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use log::LogLevel;
use util::Addressable;

use super::{Bank, Configuration, MemoryMap, Rom};

// Spec: COMMODORE 64 MEMORY MAPS p. 263
// Design:
//   Inspired by UAE memory address64k/bank concepts.
//   We define Addressable trait to represent a bank of memory and use memory configuration
//   based on zones that can be mapped to different banks. CPU uses IoPort @ 0x0001 to reconfigure
//   memory layout.

pub struct Memory {
    // Addressable
    basic: Box<Addressable>,
    charset: Rc<RefCell<Addressable>>,
    device_mem: Rc<RefCell<Addressable>>,
    expansion_port: Rc<RefCell<Addressable>>,
    kernal: Box<Addressable>,
    ram: Rc<RefCell<Addressable>>,
    // Configuration
    map: MemoryMap,
    configuration: Configuration,
}

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
        charset: Rc<RefCell<Addressable>>,
        device_mem: Rc<RefCell<Addressable>>,
        expansion_port: Rc<RefCell<Addressable>>,
        ram: Rc<RefCell<Addressable>>,
    ) -> Result<Memory, io::Error> {
        let basic = Box::new(Rom::load(
            Path::new("res/rom/basic.rom"),
            BaseAddr::Basic.addr(),
        )?);
        let kernal = Box::new(Rom::load(
            Path::new("res/rom/kernal.rom"),
            BaseAddr::Kernal.addr(),
        )?);
        let map = MemoryMap::new();
        let configuration = map.get(1);
        Ok(Memory {
            basic,
            charset,
            device_mem,
            expansion_port,
            kernal,
            ram,
            map,
            configuration,
        })
    }

    pub fn switch_banks(&mut self, mode: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "mem::banks", "Switching to {}", mode);
        }
        self.configuration = self.map.get(mode);
    }
}

impl Addressable for Memory {
    fn read(&self, address: u16) -> u8 {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.borrow().read(address),
            Bank::Basic => self.basic.read(address),
            Bank::Charset => self.charset
                .borrow()
                .read(address - BaseAddr::Charset.addr()),
            Bank::Kernal => self.kernal.read(address),
            Bank::RomL => self.expansion_port.borrow().read(address),
            Bank::RomH => self.expansion_port.borrow().read(address),
            Bank::Io => self.device_mem.borrow().read(address),
            Bank::Disabled => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.borrow_mut().write(address, value),
            Bank::Basic => self.ram.borrow_mut().write(address, value),
            Bank::Charset => self.ram.borrow_mut().write(address, value),
            Bank::Kernal => self.ram.borrow_mut().write(address, value),
            Bank::RomL => self.ram.borrow_mut().write(address, value),
            Bank::RomH => self.ram.borrow_mut().write(address, value),
            Bank::Io => self.device_mem.borrow_mut().write(address, value),
            Bank::Disabled => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::Bank;
    use mem::Addressable;

    #[test]
    fn new_memory() {
        let mem = Memory::new().unwrap();
        for bank in &mem.cpu_map {
            assert_eq!(Bank::Ram, *bank);
        }
    }

    #[test]
    fn read_basic_rom() {
        let mut mem = Memory::new().unwrap();
        mem.switch_banks(31);
        assert_eq!(0x94, mem.read(BaseAddr::Basic.addr()));
    }

    #[test]
    fn write_page_0() {
        let mut mem = Memory::new().unwrap();
        mem.write(0x00f0, 0xff);
        assert_eq!(0xff, mem.ram.read(0x00f0));
    }

    #[test]
    fn write_page_1() {
        let mut mem = Memory::new().unwrap();
        mem.write(0x0100, 0xff);
        assert_eq!(0xff, mem.ram.read(0x0100));
    }

    #[test]
    fn switch_banks_mode_24() {
        let mut mem = Memory::new().unwrap();
        mem.switch_banks(24);
        assert_eq!(Bank::Ram, mem.cpu_map[0x0]);
        assert_eq!(Bank::Ram, mem.cpu_map[0x9]);
        assert_eq!(Bank::Ram, mem.cpu_map[0xa]);
        assert_eq!(Bank::Ram, mem.cpu_map[0xb]);
        assert_eq!(Bank::Ram, mem.cpu_map[0xd]);
        assert_eq!(Bank::Ram, mem.cpu_map[0xe]);
        assert_eq!(Bank::Ram, mem.cpu_map[0xf]);
    }

    #[test]
    fn switch_banks_mode_31() {
        let mut mem = Memory::new().unwrap();
        mem.switch_banks(31);
        assert_eq!(Bank::Ram, mem.cpu_map[0x0]);
        assert_eq!(Bank::Ram, mem.cpu_map[0x9]);
        assert_eq!(Bank::Basic, mem.cpu_map[0xa]);
        assert_eq!(Bank::Basic, mem.cpu_map[0xb]);
        assert_eq!(Bank::Io, mem.cpu_map[0xd]);
        assert_eq!(Bank::Kernal, mem.cpu_map[0xe]);
        assert_eq!(Bank::Kernal, mem.cpu_map[0xf]);
    }
}
