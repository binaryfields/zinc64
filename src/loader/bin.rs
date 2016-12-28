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
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use c64::C64;
use loader::Loader;

// TODO loader: add binloader test cases

pub struct BinLoader {}

impl BinLoader {
    pub fn new() -> BinLoader {
        BinLoader {}
    }
}

impl Loader for BinLoader {
    fn load(&self, c64: &C64, path: &Path, offset: u16) -> Result<(), io::Error> {
        let memory = c64.get_memory();
        let mut data = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut data)?;
        let mut mem = memory.borrow_mut();
        let mut address = offset;
        for byte in data {
            mem.write_direct(address, byte);
            address = address.wrapping_add(1);
        }
        Ok(())
    }
}