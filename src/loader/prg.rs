/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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