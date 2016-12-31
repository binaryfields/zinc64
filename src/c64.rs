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

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::result::Result;

use cpu::Cpu;
use config::Config;
use mem::{Addressable, BaseAddr, Memory};
use io::cia;
use io::DeviceIo;
use io::Keyboard;
use video::{ColorRam, RenderTarget, Vic};

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

// TODO c64: move ioport configuration to reset
// TODO c64: update test cases to use loader

#[allow(dead_code)]
pub struct C64 {
    config: Config,
    cpu: Rc<RefCell<Cpu>>,
    mem: Rc<RefCell<Memory>>,
    color_ram: Rc<RefCell<ColorRam>>,
    cia1: Rc<RefCell<cia::Cia>>,
    cia2: Rc<RefCell<cia::Cia>>,
    keyboard: Rc<RefCell<Keyboard>>,
    rt: Rc<RefCell<RenderTarget>>,
    vic: Rc<RefCell<Vic>>,
    //sid: Rc<RefCell<Sid>>,
}

impl C64 {
    pub fn new(config: Config) -> Result<C64, io::Error> {
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
        let color_ram = Rc::new(RefCell::new(
            ColorRam::new(1024)
        ));
        let rt = Rc::new(RefCell::new(
            RenderTarget::new(config.visible_size)
        ));
        let vic = Rc::new(RefCell::new(
            Vic::new(config.clone(),
                     cpu.clone(),
                     mem.clone(),
                     color_ram.clone(),
                     rt.clone())
        ));
        let device_io = Rc::new(RefCell::new(
            DeviceIo::new(cia1.clone(),
                          cia2.clone(),
                          color_ram.clone(),
                          vic.clone())
        ));
        mem.borrow_mut().set_cia2(cia2.clone());
        mem.borrow_mut().set_device_io(device_io.clone());
        cpu.borrow_mut().write(BaseAddr::IoPortDdr.addr(), 0x2f);
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 31);
        Ok(
            C64 {
                config: config,
                cpu: cpu.clone(),
                mem: mem.clone(),
                color_ram: color_ram.clone(),
                cia1: cia1.clone(),
                cia2: cia2.clone(),
                keyboard: keyboard.clone(),
                rt: rt.clone(),
                vic: vic.clone(),
            }
        )
    }

    pub fn get_config(&self) -> &Config { &self.config }
    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> { self.cpu.clone() }
    pub fn get_keyboard(&self) -> Rc<RefCell<Keyboard>> { self.keyboard.clone() }
    pub fn get_memory(&self) -> Rc<RefCell<Memory>> { self.mem.clone() }
    pub fn get_render_target(&self) -> Rc<RefCell<RenderTarget>> { self.rt.clone() }

    pub fn load(&mut self, code: &Vec<u8>, offset: u16) {
        let mut mem = self.mem.borrow_mut();
        let mut address = offset;
        for byte in code {
            mem.write_direct(address, *byte);
            address = address.wrapping_add(1);
        }
        self.cpu.borrow_mut().set_pc(offset);
    }

    pub fn reset(&mut self) {
        self.cpu.borrow_mut().reset();
    }

    pub fn step(&mut self) {
        let prev_cycles = self.cpu.borrow().get_cycles();
        self.cpu.borrow_mut().execute();
        let elapsed = self.cpu.borrow().get_cycles() - prev_cycles;
        for i in 0..(elapsed + 1) {
            self.cia1.borrow_mut().step();
            self.cia2.borrow_mut().step();
            self.vic.borrow_mut().step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use config::Config;
    use mem::BaseAddr;

    //#[test]
    fn cpu_test() {
        let mut c64 = C64::new(Config::pal()).unwrap();
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
        let c64 = C64::new(Config::pal()).unwrap();
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
        let mut c64 = C64::new(Config::pal()).unwrap();
        let cpu = c64.get_cpu();
        let keyboard = c64.get_keyboard();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x00);
        c64.load(&code.to_vec(), 0xc000);
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
