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

use mem::{Addressable, Cartridge, Memory};

pub struct ExpansionPortIo {
    pub game: bool,
    pub exrom: bool,
}

impl ExpansionPortIo {
    pub fn new() -> ExpansionPortIo {
        ExpansionPortIo {
            game: true,
            exrom: true,
        }
    }

    pub fn reset(&mut self) {
        self.game = true;
        self.exrom = true;
    }

    pub fn update(&mut self, game: bool, exrom: bool) {
        self.game = game;
        self.exrom = exrom;
    }
}

pub struct ExpansionPort {
    mem: Rc<RefCell<Memory>>,
    cartridge: Option<Cartridge>,
    io: Rc<RefCell<ExpansionPortIo>>,
}

impl ExpansionPort {
    pub fn new(expansion_port_io: Rc<RefCell<ExpansionPortIo>>,
               mem: Rc<RefCell<Memory>>) -> ExpansionPort {
        ExpansionPort {
            mem: mem,
            cartridge: None,
            io: expansion_port_io,
        }
    }

    pub fn get_io(&self) -> Rc<RefCell<ExpansionPortIo>> {
        self.io.clone()
    }

    pub fn attach(&mut self, cartridge: Cartridge) {
        self.io.borrow_mut().update(cartridge.get_game(), cartridge.get_exrom());
        self.cartridge = Some(cartridge);
        self.mem.borrow_mut().switch_banks();
    }

    pub fn detach(&mut self) {
        if self.cartridge.is_some() {
            self.cartridge = None;
            self.io.borrow_mut().reset();
            self.mem.borrow_mut().switch_banks();
        }
    }

    pub fn reset(&mut self) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.reset();
            self.io.borrow_mut().update(cartridge.get_game(), cartridge.get_exrom());
            self.mem.borrow_mut().switch_banks();
        }
    }
}

impl Addressable for ExpansionPort {
    fn read(&self, address: u16) -> u8 {
        if let Some(ref cartridge) = self.cartridge {
            cartridge.read(address)
        } else {
            0
        }
    }

    #[allow(unused_variables)]
    fn write(&mut self, address: u16, value: u8) {
        if let Some(ref mut cartridge) = self.cartridge {
            cartridge.write(address, value)
        }
    }
}
