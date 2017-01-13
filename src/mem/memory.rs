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
use std::option::Option;
use std::rc::Rc;
use std::result::Result;

use cpu::CpuIo;
use io::{Cia, ExpansionPort, ExpansionPortIo};
use log::LogLevel;
use util::bit;

use super::{Addressable, Bank, Configuration, DeviceIo, MemoryMap, Ram, Rom};

// Spec: COMMODORE 64 MEMORY MAPS p. 263
// Design:
//   Inspired by UAE memory address64k/bank concepts.
//   We define Addressable trait to represent a bank of memory and use memory configuration
//   based on zones that can be mapped to different banks. CPU uses IoPort @ 0x0001 to reconfigure
//   memory layout.

pub struct Memory {
    // Dependencies
    cpu_io: Rc<RefCell<CpuIo>>,
    expansion_port_io: Rc<RefCell<ExpansionPortIo>>,
    cia2: Option<Rc<RefCell<Cia>>>,
    device_io: Option<Rc<RefCell<DeviceIo>>>,
    expansion_port: Option<Rc<RefCell<ExpansionPort>>>,
    // Configuration
    map: MemoryMap,
    configuration: Configuration,
    // Private Addressable
    basic: Box<Addressable>,
    charset: Box<Addressable>,
    kernal: Box<Addressable>,
    ram: Ram,
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
    pub fn new(capacity: usize,
               cpu_io: Rc<RefCell<CpuIo>>,
               expansion_port_io: Rc<RefCell<ExpansionPortIo>>) -> Result<Memory, io::Error> {
        let map = MemoryMap::new();
        let configuration = map.get(1);
        let basic = Box::new(Rom::load(Path::new("res/rom/basic.rom"), BaseAddr::Basic.addr())?);
        let charset = Box::new(Rom::load(Path::new("res/rom/characters.rom"), 0)?);
        let kernal = Box::new(Rom::load(Path::new("res/rom/kernal.rom"), BaseAddr::Kernal.addr())?);
        let ram = Ram::new(capacity);
        Ok(Memory {
            cpu_io: cpu_io,
            expansion_port_io: expansion_port_io,
            cia2: None,
            device_io: None,
            expansion_port: None,
            map: map,
            configuration: configuration,
            basic: basic,
            charset: charset,
            kernal: kernal,
            ram: ram,
        })
    }

    pub fn set_cia2(&mut self, cia: Rc<RefCell<Cia>>) {
        self.cia2 = Some(cia);
    }
    pub fn set_device_io(&mut self, device_io: Rc<RefCell<DeviceIo>>) {
        self.device_io = Some(device_io);
    }
    pub fn set_expansion_port(&mut self, expansion_port: Rc<RefCell<ExpansionPort>>) {
        self.expansion_port = Some(expansion_port);
    }

    pub fn reset(&mut self) {
        self.ram.reset();
    }

    pub fn switch_banks(&mut self) {
        let loram = bit::bit_set(0, self.cpu_io.borrow().loram);
        let hiram = bit::bit_set(1, self.cpu_io.borrow().hiram);
        let charen = bit::bit_set(2, self.cpu_io.borrow().charen);
        let game = bit::bit_set(3, self.expansion_port_io.borrow().game);
        let exrom = bit::bit_set(4, self.expansion_port_io.borrow().exrom);
        let mode = loram | hiram | charen | game | exrom;
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "mem::banks", "Switching to {}", mode);
        }
        self.configuration = self.map.get(mode);
    }

    pub fn write_ram(&mut self, address: u16, value: u8) {
        self.ram.write(address, value);
    }

    // -- VIC Memory Ops

    pub fn vic_read(&self, address: u16) -> u8 {
        if let Some(ref cia2) = self.cia2 {
            let port_a = cia2.borrow_mut().read(0x00);
            let full_address = ((!port_a & 0x03) as u16) << 14 | address;
            let zone = (full_address & 0xf000) >> 12;
            match zone {
                0x01 => self.charset.read(full_address - 0x1000),
                0x09 => self.charset.read(full_address - 0x9000),
                _ => self.ram.read(full_address),
            }
        } else {
            panic!("cia2 not set")
        }
    }
}


impl Addressable for Memory {
    fn read(&self, address: u16) -> u8 {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.read(address),
            Bank::Basic => self.basic.read(address),
            Bank::Charset => self.charset.read(address - BaseAddr::Charset.addr()),
            Bank::Kernal => self.kernal.read(address),
            Bank::RomL => if let Some(ref expansion_port) = self.expansion_port {
                expansion_port.borrow().read(address)
            } else {
                panic!("expansion port not set")
            },
            Bank::RomH => if let Some(ref expansion_port) = self.expansion_port {
                expansion_port.borrow().read(address)
            } else {
                panic!("expansion port not set")
            },
            Bank::Io => if let Some(ref device_io) = self.device_io {
                device_io.borrow().read(address)
            } else {
                panic!("device io not set")
            },
            Bank::Disabled => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        let zone = address >> 12;
        match self.configuration.get(zone as u8) {
            Bank::Ram => self.ram.write(address, value),
            Bank::Basic => self.ram.write(address, value),
            Bank::Charset => self.ram.write(address, value),
            Bank::Kernal => self.ram.write(address, value),
            Bank::RomL => self.ram.write(address, value),
            Bank::RomH => self.ram.write(address, value),
            Bank::Io => if let Some(ref device_io) = self.device_io {
                device_io.borrow_mut().write(address, value)
            } else {
                panic!("device io not set")
            },
            Bank::Disabled => {},
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
