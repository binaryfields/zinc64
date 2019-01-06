// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec;

pub struct Rom {
    data: Vec<u8>,
    offset: u16,
}

impl Rom {
    pub fn new(capacity: usize, offset: u16, pattern: u8) -> Self {
        let mut data = vec![0x00; capacity];
        for byte in &mut data {
            *byte = pattern;
        }
        Self { data, offset }
    }

    pub fn new_with_data(data: &[u8], offset: u16) -> Self {
        Rom {
            data: data.to_vec(),
            offset,
        }
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
    #[test]
    fn read_address() {
        // FIXME nostd: enable test
        // let rom = Rom::load(&Path::new("res/rom/basic.rom"), 0x0000).unwrap();
        // assert_eq!(0x94, rom.read(0x0000));
    }
}
