// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: https://www.c64-wiki.com/index.php/Bank_Switching

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use log::LogLevel;
use zinc64_core::{Bank, Mmu};

#[derive(Clone, Copy)]
struct Mode {
    banks: [Bank; 0x10],
}

impl Mode {
    pub fn new(config: [Bank; 7]) -> Self {
        let mut banks = [Bank::Disabled; 0x10];
        for (i, bank) in banks.iter_mut().enumerate().take(0x10) {
            *bank = match i {
                0x00 => config[0],
                0x01...0x07 => config[1],
                0x08...0x09 => config[2],
                0x0a...0x0b => config[3],
                0x0c => config[4],
                0x0d => config[5],
                0x0e...0x0f => config[6],
                _ => panic!("invalid bank {}", i),
            };
        }
        Mode { banks }
    }

    pub fn get(&self, zone: u8) -> Bank {
        self.banks[zone as usize]
    }
}

pub struct Pla {
    map: MemoryMap,
    mode: Mode,
}

impl Pla {
    pub fn new() -> Self {
        let map = MemoryMap::default();
        let configuration = map.get(0);
        Pla { map, mode: configuration }
    }
}

impl Mmu for Pla {
    fn map(&self, address: u16) -> Bank {
        let zone = address >> 12;
        self.mode.get(zone as u8)
    }

    fn switch_banks(&mut self, mode: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "mem::banks", "Switching to {}", mode);
        }
        self.mode = self.map.get(mode);
    }
}

struct MemoryMap {
    modes: [Mode; 32],
}

impl Default for MemoryMap {
    fn default() -> Self {
        let m31 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Basic,
            Bank::Ram,
            Bank::Io,
            Bank::Kernal,
        ];
        let m30_14 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Io,
            Bank::Kernal,
        ];
        let m29_13 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Io,
            Bank::Ram,
        ];
        let m28_24 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
        ];
        let m27 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Basic,
            Bank::Ram,
            Bank::Charset,
            Bank::Kernal,
        ];
        let m26_10 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Charset,
            Bank::Kernal,
        ];
        let m25_9 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Charset,
            Bank::Ram,
        ];
        let m23_16 = [
            Bank::Ram,
            Bank::Disabled,
            Bank::RomL,
            Bank::Disabled,
            Bank::Disabled,
            Bank::Io,
            Bank::RomH,
        ];
        let m15 = [
            Bank::Ram,
            Bank::Ram,
            Bank::RomL,
            Bank::Basic,
            Bank::Ram,
            Bank::Io,
            Bank::Kernal,
        ];
        let m12_8_4_0 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
        ];
        let m11 = [
            Bank::Ram,
            Bank::Ram,
            Bank::RomL,
            Bank::Basic,
            Bank::Ram,
            Bank::Charset,
            Bank::Kernal,
        ];
        let m7 = [
            Bank::Ram,
            Bank::Ram,
            Bank::RomL,
            Bank::RomH,
            Bank::Ram,
            Bank::Io,
            Bank::Kernal,
        ];
        let m6 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::RomH,
            Bank::Ram,
            Bank::Io,
            Bank::Kernal,
        ];
        let m5 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Io,
            Bank::Ram,
        ];
        let m3 = [
            Bank::Ram,
            Bank::Ram,
            Bank::RomL,
            Bank::RomH,
            Bank::Ram,
            Bank::Charset,
            Bank::Kernal,
        ];
        let m2 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::RomH,
            Bank::Ram,
            Bank::Charset,
            Bank::Kernal,
        ];
        let m1 = [
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
            Bank::Ram,
        ];
        MemoryMap {
            modes: [
                Mode::new(m12_8_4_0),
                Mode::new(m1),
                Mode::new(m2),
                Mode::new(m3),
                Mode::new(m12_8_4_0),
                Mode::new(m5),
                Mode::new(m6),
                Mode::new(m7),
                Mode::new(m12_8_4_0),
                Mode::new(m25_9),
                Mode::new(m26_10),
                Mode::new(m11),
                Mode::new(m12_8_4_0),
                Mode::new(m29_13),
                Mode::new(m30_14),
                Mode::new(m15),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m23_16),
                Mode::new(m28_24),
                Mode::new(m25_9),
                Mode::new(m26_10),
                Mode::new(m27),
                Mode::new(m28_24),
                Mode::new(m29_13),
                Mode::new(m30_14),
                Mode::new(m31),
            ],
        }
    }
}

impl MemoryMap {
    pub fn get(&self, mode: u8) -> Mode {
        match mode {
            0...31 => self.modes[mode as usize],
            _ => panic!("invalid mode {}", mode),
        }
    }
}
