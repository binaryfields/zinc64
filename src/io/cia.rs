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
use std::rc::Rc;

use cpu::Cpu;

#[allow(dead_code)]
pub struct Cia {
    cpu: Rc<RefCell<Cpu>>,
    regs: [u8; 16],
}

impl Cia {
    pub fn new(cpu: Rc<RefCell<Cpu>>) -> Cia {
        Cia {
            cpu: cpu,
            regs: [0; 16],
        }
    }

    #[allow(dead_code)]
    pub fn read(&self, reg: u8) -> u8 {
        match reg {
            0x0 ... 0xf => self.regs[reg as usize],
            _ => panic!("invalid register")
        }
    }

    #[allow(dead_code, unused_variables)]
    pub fn write(&mut self, reg: u8, value: u8) {
        panic!("writes to device are not supported")
    }
}
