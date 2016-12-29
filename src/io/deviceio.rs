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

use io::cia::Cia;
use mem::Addressable;
use video::Vic;

pub struct DeviceIo {
    cia1: Rc<RefCell<Cia>>,
    cia2: Rc<RefCell<Cia>>,
    vic: Rc<RefCell<Vic>>,
}

impl DeviceIo {
    pub fn new(cia1: Rc<RefCell<Cia>>, cia2: Rc<RefCell<Cia>>, vic: Rc<RefCell<Vic>>) -> DeviceIo {
        DeviceIo {
            cia1: cia1,
            cia2: cia2,
            vic: vic,
        }
    }
}

impl Addressable for DeviceIo {
    fn read(&self, address: u16) -> u8 {
        match address {
            0xd000 ... 0xd3ff => self.vic.borrow_mut().read((address & 0x003f) as u8),
            0xd400 ... 0xd7ff => 0x00, // sid
            0xd800 ... 0xdbff => 0x00, // color ram
            0xdc00 ... 0xdcff => self.cia1.borrow_mut().read((address & 0x000f) as u8),
            0xdd00 ... 0xddff => self.cia2.borrow_mut().read((address & 0x000f) as u8),
            0xde00 ... 0xdeff => 0x00, // I/O 1
            0xdf00 ... 0xdfff => 0x00, // I/O 2
            _ => panic!("invalid address")
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xd000 ... 0xd3ff => self.vic.borrow_mut().write((address & 0x003f) as u8, value),
            0xd400 ... 0xd7ff => {}, // sid
            0xd800 ... 0xdbff => {}, // color ram
            0xdc00 ... 0xdcff => self.cia1.borrow_mut().write((address & 0x000f) as u8, value),
            0xdd00 ... 0xddff => self.cia2.borrow_mut().write((address & 0x000f) as u8, value),
            0xde00 ... 0xdeff => {}, // I/O 1
            0xdf00 ... 0xdfff => {}, // I/O 2
            _ => panic!("invalid address")
        }
    }
}
