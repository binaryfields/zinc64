/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::result::Result;

pub struct Rom {
    data: Vec<u8>,
    offset: u16,
}

impl Rom {
    pub fn new(capacity: usize, offset: u16, pattern: u8) -> Self {
        let mut data = vec![0x00; capacity];
        for i in 0..data.len() {
            data[i] = pattern;
        }
        Self { data, offset }
    }

    pub fn load(path: &Path, offset: u16) -> Result<Rom, io::Error> {
        info!(target: "mem", "Loading ROM {:?}", path.to_str().unwrap());
        let mut data = Vec::new();
        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut data)?;
        Ok(Rom { data, offset })
    }

    pub fn read(&self, address: u16) -> u8 {
        self.data[(address - self.offset) as usize]
    }

    pub fn write(&mut self, _address: u16, _value: u8) {
        panic!("writes to rom are not supported")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_address() {
        let rom = Rom::load(&Path::new("res/rom/basic.rom"), 0x0000).unwrap();
        assert_eq!(0x94, rom.read(0x0000));
    }
}
