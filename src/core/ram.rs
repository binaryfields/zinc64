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

pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new(capacity: usize) -> Ram {
        info!(target: "mem", "Initializing RAM with capacity {}", capacity);
        Ram {
            data: vec![0x00; capacity],
        }
    }

    pub fn fill(&mut self, pattern: u8) {
        for i in 0..self.data.len() {
            self.data[i] = pattern;
        }
    }

    pub fn load(&mut self, data: &Vec<u8>, offset: u16) {
        let mut address = offset;
        for byte in data {
            self.write(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    #[inline]
    pub fn read(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    #[inline]
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