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

use std::fs;
use std::io;
use std::io::Read;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use cpu::Cpu;
use mem::{Addressable, BaseAddr, Memory};
use io::cia;
use io::DeviceIo;
use io::Keyboard;

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

// TODO c64: load should bypass mapped io

#[allow(dead_code)]
pub struct C64 {
    cpu: Rc<RefCell<Cpu>>,
    mem: Rc<RefCell<Memory>>,
    cia1: Rc<RefCell<cia::Cia>>,
    cia2: Rc<RefCell<cia::Cia>>,
    keyboard: Rc<RefCell<Keyboard>>,
    //vid: Rc<RefCell<Vic>>,
    //sid: Rc<RefCell<Sid>>,
}

impl C64 {
    pub fn new() -> Result<C64, io::Error> {
        let mem = Rc::new(RefCell::new(
            Memory::new()?
        ));
        let cpu = Rc::new(RefCell::new(
            Cpu::new(mem.clone())
        ));
        let keyboard = Rc::new(RefCell::new(
            Keyboard::new()
        ));
        let cia1 = Rc::new(RefCell::new(
            cia::Cia::new(cia::Mode::Cia1, cpu.clone(), keyboard.clone())
        ));
        let cia2 = Rc::new(RefCell::new(
            cia::Cia::new(cia::Mode::Cia2, cpu.clone(), keyboard.clone())
        ));
        let device_io = Rc::new(RefCell::new(
            DeviceIo::new(cia1.clone(), cia2.clone())
        ));
        mem.borrow_mut().set_device_io(device_io.clone());
        cpu.borrow_mut().write(BaseAddr::IoPortDdr.addr(), 0x2f);
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 31);
        Ok(
            C64 {
                cpu: cpu.clone(),
                mem: mem.clone(),
                cia1: cia1.clone(),
                cia2: cia2.clone(),
                keyboard: keyboard.clone(),
            }
        )
    }

    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> { self.cpu.clone() }
    pub fn get_keyboard(&self) -> Rc<RefCell<Keyboard>> { self.keyboard.clone() }

    pub fn load(&mut self, path: &Path, offset: u16) -> Result<(), io::Error> {
        let mut data = Vec::new();
        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut data)?;
        let mut address = offset;
        let mut mem = self.mem.borrow_mut();
        for byte in &data {
            mem.write(address, *byte);
            address = address.wrapping_add(1);
        }
        Ok(())
    }

    pub fn load_code(&mut self, code: &Vec<u8>, offset: u16) -> Result<(), io::Error> {
        let mut address = offset;
        let mut mem = self.mem.borrow_mut();
        for byte in code {
            mem.write(address, *byte);
            address = address.wrapping_add(1);
        }
        Ok(())
    }

    pub fn step(&mut self) {
        let prev_cycles = self.cpu.borrow().get_cycles();
        self.cpu.borrow_mut().execute();
        let elapsed = self.cpu.borrow().get_cycles() - prev_cycles + 1;
        for i in 0..elapsed {
            self.cia1.borrow_mut().step();
            self.cia2.borrow_mut().step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use mem::BaseAddr;

    #[test]
    fn cpu_test() {
        let mut c64 = C64::new().unwrap();
        let cpu = c64.get_cpu();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x00);
        c64.load(Path::new("rom/6502_functional_test.bin"), 0x0400).unwrap();
        cpu.borrow_mut().set_pc(0x0400);
        let mut last_pc = 0x0000;
        loop {
            c64.step();
            if cpu.borrow_mut().get_pc() == 0x3463 {
                break;
            }
            if cpu.borrow_mut().get_pc() == last_pc {
                panic!("trap at 0x{:x}", cpu.borrow_mut().get_pc());
            }
            last_pc = cpu.borrow_mut().get_pc();
        }
    }

    #[test]
    fn mem_layout() {
        let c64 = C64::new().unwrap();
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.borrow().read(BaseAddr::Basic.addr()));
    }

    #[test]
    fn keyboard_read() {
        /*
        .c000  78         sei
        .c001  a9 ff      lda #$ff
        .c003  8d 02 dc   sta $dc02
        .c006  a9 00      lda #$00
        .c008  8d 03 dc   sta $dc03
        .c00b  a9 fd      lda #$fd
        .c00d  8d 00 dc   sta $dc00
        .c010  ad 01 dc   lda $dc01
        .c013  29 20      and #$20
        .c015  d0 f9      bne $c010
        .c017  58         cli
        .c018  60         rts
        */
        let code = [
            0x78u8,
            0xa9, 0xff,
            0x8d, 0x02, 0xdc,
            0xa9, 0x00,
            0x8d, 0x03, 0xdc,
            0xa9, 0xfd,
            0x8d, 0x00, 0xdc,
            0xad, 0x01, 0xdc,
            0x29, 0x20,
            0xd0, 0xf9,
            0x58
        ];
        let mut c64 = C64::new().unwrap();
        let cpu = c64.get_cpu();
        let keyboard = c64.get_keyboard();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x00);
        c64.load_code(&code.to_vec(), 0xc000).unwrap();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x06);
        keyboard.borrow_mut().set_row(1, !(1 << 5));
        cpu.borrow_mut().set_pc(0xc000);
        let mut last_pc = 0x0000;
        let mut branch_count = 0;
        loop {
            c64.step();
            if cpu.borrow().get_pc() == 0xc018 {
                break;
            }
            if cpu.borrow().get_pc() == 0xc015 {
                branch_count += 1;
                if branch_count > 1 {
                    panic!("trap at 0x{:x}", cpu.borrow_mut().get_pc());
                }
            }
            last_pc = cpu.borrow_mut().get_pc();
        }
    }
}
