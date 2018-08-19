// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::iter::Iterator;

use core::Addressable;

// SPEC: http://ist.uwaterloo.ca/~schepers/formats/CRT.TXT

// DEFERRED device: cartridge test cases

pub enum ChipType {
    Rom,
    Ram,
    FlashRom,
}

impl ChipType {
    pub fn from(chip_type: u16) -> ChipType {
        match chip_type {
            0x00 => ChipType::Rom,
            0x01 => ChipType::Ram,
            0x02 => ChipType::FlashRom,
            _ => panic!("invalid chip type {}", chip_type),
        }
    }
}

pub struct Chip {
    pub chip_type: ChipType,
    pub bank_number: u8,
    pub offset: u16,
    pub size: u16,
    pub data: Vec<u8>,
}

#[derive(PartialEq)]
pub enum HwType {
    Normal,
    ActionReplay,
    KCSPower,
    Final3,
    SimonsBasic,
    OceanType1,
    Expert,
}

impl HwType {
    pub fn from(value: u8) -> HwType {
        match value {
            0 => HwType::Normal,
            1 => HwType::ActionReplay,
            2 => HwType::KCSPower,
            3 => HwType::Final3,
            4 => HwType::SimonsBasic,
            5 => HwType::OceanType1,
            6 => HwType::Expert,
            _ => panic!("invalid hardware type {}", value),
        }
    }
}

pub struct Cartridge {
    pub version: u16,
    pub hw_type: HwType,
    pub exrom: bool,
    pub game: bool,
    pub banks: Vec<Chip>,
    pub bank_lo: usize,
    pub bank_hi: usize,
}

impl Cartridge {
    pub fn get_exrom(&self) -> bool {
        self.exrom
    }

    pub fn get_game(&self) -> bool {
        self.game
    }

    pub fn add(&mut self, chip: Chip) {
        self.banks.push(chip);
    }

    pub fn reset(&mut self) {
        self.switch_bank(0);
    }

    fn switch_bank(&mut self, bank_number: u8) {
        let bank_lo = self
            .banks
            .iter()
            .find(|&bank| bank.bank_number == bank_number && bank.offset < 0xa000);
        let bank_hi = self
            .banks
            .iter()
            .find(|&bank| bank.bank_number == bank_number && bank.offset >= 0xa000);
        match bank_lo {
            Some(ref bank) => self.bank_lo = bank.bank_number as usize,
            None => panic!("invalid bank number {}", bank_number),
        }
        match bank_hi {
            Some(ref bank) => self.bank_hi = bank.bank_number as usize,
            None => {
                if self.banks[self.bank_lo].size >= 0x4000 {
                    self.bank_hi = self.bank_lo;
                } else {
                    panic!("invalid bank number {}", bank_number);
                }
            }
        }
    }

    // -- Device I/O

    fn read_io(&self, _address: u16) -> u8 {
        0
    }

    fn write_io(&mut self, address: u16, value: u8) {
        match address {
            0xde00 if self.hw_type == HwType::SimonsBasic => {
                self.game = value == 0x01;
                // TODO crt: update memory layout
            }
            0xde00 if self.hw_type == HwType::OceanType1 => {
                self.switch_bank(value & 0x3f);
            }
            0xdfff if self.hw_type == HwType::Final3 => {
                self.switch_bank(value - 0x40);
            }
            _ => {}
        }
    }
}

impl Addressable for Cartridge {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x8000...0x9fff => {
                let bank = &self.banks[self.bank_lo];
                bank.data[(address - bank.offset) as usize]
            }
            0xa000...0xbfff => {
                let bank = &self.banks[self.bank_hi];
                bank.data[(address - bank.offset) as usize]
            }
            0xde00...0xdfff => self.read_io(address),
            _ => panic!("invalid address {}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xde00...0xdfff => self.write_io(address, value),
            _ => panic!("writes to cartridge are not supported"),
        }
    }
}
