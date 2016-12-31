/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::io;
use std::cell::RefCell;
use std::path::Path;
use std::option::Option;
use std::rc::Rc;
use std::result::Result;

use io::DeviceIo;
use io::cia::Cia;
use mem::Addressable;
use mem::Ram;
use mem::Rom;

// Spec: COMMODORE 64 MEMORY MAPS p. 263
// Design:
//   Inspired by UAE memory address64k/bank concepts.
//   We define Addressable trait to represent a bank of memory and use memory configuration
//   based on zones that can be mapped to different banks. CPU uses IoPort @ 0x0001 to reconfigure
//   memory layout.

pub struct Memory {
    cpu_map: [Bank; 16],
    vic_map: [Bank; 16],
    ram: Box<Addressable>,
    basic: Box<Addressable>,
    charset: Box<Addressable>,
    charset_vic: Box<Addressable>, // FIXME remove this hack
    kernal: Box<Addressable>,
    cia2: Option<Rc<RefCell<Cia>>>,
    device_io: Option<Rc<RefCell<DeviceIo>>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Bank {
    Ram = 1,
    Basic = 2,
    Charset = 3,
    Kernal = 4,
    Io = 5
}

enum ControlLine {
    LORAM = 1 << 0,
    HIRAM = 1 << 1,
    CHAREN = 1 << 2,
    // GAME = 1 << 3,
    // EXROM = 1 << 4,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum BaseAddr {
    IoPortDdr = 0x0000,
    IoPort = 0x0001,
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
    pub fn new() -> Result<Memory, io::Error> {
        let basic = Box::new(Rom::load(Path::new("rom/basic.rom"), BaseAddr::Basic.addr())?);
        let charset = Box::new(Rom::load(Path::new("rom/characters.rom"), BaseAddr::Charset.addr())?);
        let charset_vic = Box::new(Rom::load(Path::new("rom/characters.rom"), 0)?);
        let kernal = Box::new(Rom::load(Path::new("rom/kernal.rom"), BaseAddr::Kernal.addr())?);
        let mut vic_map = [Bank::Ram; 16];
        vic_map[0x1] = Bank::Charset;
        vic_map[0x9] = Bank::Charset;
        Ok(Memory {
            cpu_map: [Bank::Ram; 16],
            vic_map: vic_map,
            ram: Box::new(Ram::new(0x10000)),
            basic: basic,
            charset: charset,
            charset_vic: charset_vic,
            kernal: kernal,
            cia2: None,
            device_io: None,
        })
    }

    pub fn set_cia2(&mut self, cia: Rc<RefCell<Cia>>) { self.cia2 = Some(cia); }
    pub fn set_device_io(&mut self, device_io: Rc<RefCell<DeviceIo>>) {
        self.device_io = Some(device_io);
    }

    pub fn switch_banks(&mut self, mode: u8) {
        let loram = self.test_control(mode, ControlLine::LORAM);
        let hiram = self.test_control(mode, ControlLine::HIRAM);
        let charen = self.test_control(mode, ControlLine::CHAREN);
        for zone in 0x0..0x10 {
            let bank = match zone {
                0x0 ... 0x9 => Bank::Ram,
                0xa ... 0xb => if loram && hiram { Bank::Basic } else { Bank::Ram },
                0xc => Bank::Ram,
                0xd => {
                    if !hiram && !charen {
                        Bank::Ram
                    } else if !charen {
                        Bank::Charset
                    } else {
                        Bank::Io
                    }
                },
                0xe ... 0xf => if hiram { Bank::Kernal } else { Bank::Ram },
                _ => panic!("invalid zone")
            };
            self.cpu_map[zone] = bank;
        }
    }

    pub fn vic_read(&self, address: u16) -> u8 {
        if let Some(ref cia2) = self.cia2 {
            let port_a = cia2.borrow_mut().read(0x00);
            let full_address = ((!port_a & 0x03) as u16) << 14 | address;
            let zone = (full_address & 0xf000) >> 12;
            let bank = self.vic_map[zone as usize];
            match bank {
                Bank::Ram => self.ram.read(full_address),
                Bank::Charset => {
                    if zone == 0x1 {
                        self.charset_vic.read(full_address - 0x1000)
                    } else {
                        self.charset_vic.read(full_address - 0x9000)
                    }
                },
                _ => panic!("invalid bank {}", bank as u8),
            }
        } else {
            panic!("cia2 is not set");
        }
    }

    pub fn write_direct(&mut self, address: u16, value: u8) {
        self.ram.write(address, value);
    }

    fn test_control(&self, mode: u8, line: ControlLine) -> bool {
        if mode & (line as u8) != 0 { true } else { false }
    }
}


impl Addressable for Memory {
    fn read(&self, address: u16) -> u8 {
        let zone = (address & 0xf000) >> 12;
        let bank = self.cpu_map[zone as usize];
        match bank {
            Bank::Ram => self.ram.read(address),
            Bank::Basic => self.basic.read(address),
            Bank::Charset => self.charset.read(address),
            Bank::Kernal => self.kernal.read(address),
            Bank::Io => {
                match self.device_io {
                    Some(ref io) => io.borrow().read(address),
                    None => panic!("invalid device io"),
                }
            },
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        let zone = (address & 0xf000) >> 12;
        let bank = self.cpu_map[zone as usize];
        match bank {
            Bank::Ram => self.ram.write(address, value),
            Bank::Basic => self.ram.write(address, value),
            Bank::Charset => self.ram.write(address, value),
            Bank::Kernal => self.ram.write(address, value),
            Bank::Io => {
                match self.device_io {
                    Some(ref io) => io.borrow_mut().write(address, value),
                    None => panic!("invalid device io"),
                }
            },
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
