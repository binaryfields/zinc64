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
use io::Cia;

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

// TODO c64: load should bypass mapped io

#[allow(dead_code)]
pub struct C64 {
    cpu: Rc<RefCell<Cpu>>,
    mem: Rc<RefCell<Memory>>,
    cia1: Rc<RefCell<Cia>>,
    cia2: Rc<RefCell<Cia>>,
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
        let cia1 = Rc::new(RefCell::new(
            Cia::new(cpu.clone())
        ));
        let cia2 = Rc::new(RefCell::new(
            Cia::new(cpu.clone())
        ));
        cpu.borrow_mut().write(BaseAddr::IoPortDdr.addr(), 0x2f);
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 31);
        Ok(
            C64 {
                cpu: cpu.clone(),
                mem: mem.clone(),
                cia1: cia1.clone(),
                cia2: cia2.clone(),
            }
        )
    }

    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> {
        self.cpu.clone()
    }

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

    pub fn step(&mut self) {
        self.cpu.borrow_mut().execute();
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
    fn c64_mem_layout() {
        let c64 = C64::new().unwrap();
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.borrow().read(BaseAddr::Basic.addr()));
    }
}
