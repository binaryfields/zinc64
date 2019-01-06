// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec;

pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0x00; capacity],
        }
    }

    pub fn fill(&mut self, pattern: u8) {
        for i in 0..self.data.len() {
            self.data[i] = pattern;
        }
    }

    pub fn load(&mut self, data: &[u8], offset: u16) {
        let mut address = offset;
        for byte in data {
            self.write(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.data[address as usize] = value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_capacity() {
        let ram = Ram::new(0x10000);
        assert_eq!(0x10000, ram.data.len());
    }

    #[test]
    fn read_address() {
        let mut ram = Ram::new(0x10000);
        ram.fill(0xfe);
        assert_eq!(0xfe, ram.read(0xffff));
    }

    #[test]
    fn write_address() {
        let mut ram = Ram::new(0x10000);
        ram.fill(0xfe);
        ram.write(0x0001, 31);
        assert_eq!(31, ram.read(0x0001));
    }
}
