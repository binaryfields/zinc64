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

mod bin;
mod crt;
mod hex;

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use c64::C64;
use mem::Memory;

pub trait Loader {
    fn load(&self, c64: &mut C64, path: &Path, offset: u16) -> Result<(), io::Error>;
}

pub struct Loaders {}

impl Loaders {
    pub fn new(ext: Option<&str>) -> Box<Loader> {
        match ext {
            Some("bin") => Box::new(bin::BinLoader::new()),
            Some("crt") => Box::new(crt::CrtLoader::new()),
            Some("hex") => Box::new(hex::HexLoader::new()),
            _ => panic!("invalid loader {}", ext.unwrap_or(""))
        }
    }
}

