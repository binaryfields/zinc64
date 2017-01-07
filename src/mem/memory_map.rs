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

// SPEC: https://www.c64-wiki.com/index.php/Bank_Switching

// TODO memory_map: add test cases

#[derive(Clone, Copy)]
pub enum Bank {
    Basic,
    Charset,
    Kernal,
    Io,
    Ram,
    RomH,
    RomL,
    Disabled,
}

#[derive(Clone, Copy)]
pub struct Configuration {
    banks: [Bank; 0x10],
}

impl Configuration {
    pub fn new(config: [Bank; 7]) -> Configuration {
        let mut banks = [Bank::Disabled; 0x10];
        for i in 0..0x10 {
            banks[i] = match i {
                0x00 => config[0],
                0x01 ... 0x07 => config[1],
                0x08 ... 0x09 => config[2],
                0x0a ... 0x0b => config[3],
                0x0c => config[4],
                0x0d => config[5],
                0x0e ... 0x0f => config[6],
                _ => panic!("invalid bank {}", i),
            };
        }
        Configuration {
            banks: banks,
        }
    }

    pub fn get(&self, zone: u8) -> Bank {
        self.banks[zone as usize]
    }
}

pub struct MemoryMap {
    modes: [Configuration; 31],
}

impl MemoryMap {
    pub fn new() -> MemoryMap {
        let m31 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Basic, Bank::Ram, Bank::Io, Bank::Kernal];
        let m30_14 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Io, Bank::Kernal];
        let m29_13 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Io, Bank::Ram];
        let m28_24 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram];
        let m27 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Basic, Bank::Ram, Bank::Charset, Bank::Kernal];
        let m26_10 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Charset, Bank::Kernal];
        let m25_9 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Charset, Bank::Ram];
        let m23_16 = [Bank::Ram, Bank::Disabled, Bank::RomL, Bank::Disabled, Bank::Disabled, Bank::Io, Bank::RomH];
        let m15 = [Bank::Ram, Bank::Ram, Bank::RomL, Bank::Basic, Bank::Ram, Bank::Io, Bank::Kernal];
        let m12_8_4_0 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram];
        let m11 = [Bank::Ram, Bank::Ram, Bank::RomL, Bank::Basic, Bank::Ram, Bank::Charset, Bank::Kernal];
        let m7 = [Bank::Ram, Bank::Ram, Bank::RomL, Bank::RomH, Bank::Ram, Bank::Io, Bank::Kernal];
        let m6 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::RomH, Bank::Ram, Bank::Io, Bank::Kernal];
        let m5 = [Bank::Ram, Bank::Ram, Bank::RomL, Bank::RomH, Bank::Ram, Bank::Io, Bank::Ram];
        let m3 = [Bank::Ram, Bank::Ram, Bank::RomL, Bank::RomH, Bank::Ram, Bank::Charset, Bank::Kernal];
        let m2 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::RomH, Bank::Ram, Bank::Charset, Bank::Kernal];
        let m1 = [Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram, Bank::Ram];
        MemoryMap {
            modes: [
                Configuration::new(m1),
                Configuration::new(m2),
                Configuration::new(m3),
                Configuration::new(m12_8_4_0),
                Configuration::new(m5),
                Configuration::new(m6),
                Configuration::new(m7),
                Configuration::new(m12_8_4_0),
                Configuration::new(m25_9),
                Configuration::new(m26_10),
                Configuration::new(m11),
                Configuration::new(m12_8_4_0),
                Configuration::new(m29_13),
                Configuration::new(m30_14),
                Configuration::new(m15),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m23_16),
                Configuration::new(m28_24),
                Configuration::new(m25_9),
                Configuration::new(m26_10),
                Configuration::new(m27),
                Configuration::new(m28_24),
                Configuration::new(m29_13),
                Configuration::new(m30_14),
                Configuration::new(m31),
            ]
        }
    }

    pub fn get(&self, mode: u8) -> Configuration {
        match mode {
            1 ... 31 => self.modes[(mode - 1) as usize],
            _ => panic!("invalid mode {}", mode),
        }
    }
}