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

use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use c64::C64;
use loader::Loader;

pub struct HexLoader {}

impl HexLoader {
    pub fn new() -> HexLoader {
        HexLoader {}
    }

    fn map_hex(&self, hex: char) -> u8 {
        match hex {
            '0' => 0x00,
            '1' => 0x01,
            '2' => 0x02,
            '3' => 0x03,
            '4' => 0x04,
            '5' => 0x05,
            '6' => 0x06,
            '7' => 0x07,
            '8' => 0x08,
            '9' => 0x09,
            'a' => 0x0a,
            'b' => 0x0b,
            'c' => 0x0c,
            'd' => 0x0d,
            'e' => 0x0e,
            'f' => 0x0f,
            'A' => 0x0a,
            'B' => 0x0b,
            'C' => 0x0c,
            'D' => 0x0d,
            'E' => 0x0e,
            'F' => 0x0f,
            _ => 0xff,
        }
    }

    fn parse_byte(&self, hex: &str) -> Option<u8> {
        if hex.len() == 2 {
            let hi_byte = self.map_hex(hex.chars().nth(0).unwrap());
            let lo_byte = self.map_hex(hex.chars().nth(1).unwrap());
            if hi_byte != 0xff && lo_byte != 0xff {
                Some(hi_byte << 4 | lo_byte)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_word(&self, hex: &str) -> Option<u16> {
        if hex.len() == 4 {
            if let Some(hi_byte) = self.parse_byte(&hex[0..2]) {
                if let Some(lo_byte) = self.parse_byte(&hex[2..4]) {
                    Some(((hi_byte as u16) << 8) | (lo_byte as u16))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Loader for HexLoader {
    fn load(&self, c64: &mut C64, path: &Path, offset: u16) -> Result<(), io::Error> {
        let memory = c64.get_memory();
        let mut mem = memory.borrow_mut();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let lines: Vec<_> = reader.lines().collect();
        let mut address = offset;
        let mut line_num = 0;
        for l in lines {
            line_num += 1;
            let line = l.unwrap();
            let semi = line.find(';').unwrap_or(line.len());
            let (code, comment) = line.split_at(semi);
            let tokens: Vec<&str> = code.split_whitespace().collect();
            for token in tokens {
                if !token.starts_with(".") {
                    if let Some(byte) = self.parse_byte(token) {
                        mem.write_ram(address, byte);
                        address = address.wrapping_add(1);
                    } else {
                        panic!("invalid bytecode {} at line {}", token, line_num);
                    }
                }
            }
        }
        Ok(())
    }
}
