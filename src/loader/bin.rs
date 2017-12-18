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

use cpu::TickFn;
use loader::{Image, Loader};
use loader::autostart;
use system::C64;

struct BinImage {
    data: Vec<u8>,
    offset: u16,
}

impl Image for BinImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting BIN image");
        let cpu = c64.get_cpu();
        let tick_fn: TickFn = Box::new(move || {});
        cpu.borrow_mut().write(0x0001, 0, &tick_fn);
        c64.load(&self.data, self.offset);
        cpu.borrow_mut().set_pc(self.offset);
    }

    #[allow(unused_variables)]
    fn unmount(&mut self, c64: &mut C64) {}
}

pub struct BinLoader {
    offset: u16,
}

impl BinLoader {
    pub fn new(offset: u16) -> BinLoader {
        BinLoader { offset: offset }
    }
}

impl Loader for BinLoader {
    fn autostart(&self, path: &Path) -> Result<autostart::Method, io::Error> {
        let image = self.load(path)?;
        Ok(autostart::Method::WithBinImage(image))
    }

    fn load(&self, path: &Path) -> Result<Box<Image>, io::Error> {
        info!(target: "loader", "Loading BIN {}", path.to_str().unwrap());
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Box::new(BinImage {
            data: data,
            offset: self.offset,
        }))
    }
}
