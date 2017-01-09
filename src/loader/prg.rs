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

use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use std::result::Result;

use byteorder::{LittleEndian, ReadBytesExt};
use c64::C64;
use loader::{Autostart, Image, Loader};
use loader::autostart;

pub struct PrgImage {
    data: Vec<u8>,
    offset: u16,
}

impl Image for PrgImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting PRG image");
        c64.load(&self.data, self.offset);
    }
    fn unmount(&mut self, c64: &mut C64) {}
}

pub struct PrgLoader {}

impl PrgLoader {
    pub fn new() -> PrgLoader {
        PrgLoader {}
    }
}

impl Loader for PrgLoader {
    fn autostart(&self, path: &Path) -> Result<autostart::Method, io::Error> {
        let image = self.load(path)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(autostart::Method::WithAutostart(Some(autostart)))
    }

    fn load(&self, path: &Path) -> Result<Box<Image>, io::Error> {
        info!(target: "loader", "Loading PRG {}", path.to_str().unwrap());
        let mut file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(
            Box::new(
                PrgImage {
                    data: data,
                    offset: offset,
                }
            )
        )
    }
}