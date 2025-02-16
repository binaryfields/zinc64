// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;
use log::LogLevel;

// SPEC: http://ist.uwaterloo.ca/~schepers/formats/CRT.TXT

// DEFERRED device: cartridge test cases

pub struct Chip {
    pub chip_type: ChipType,
    pub bank_number: u8,
    pub offset: u16,
    pub size: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HwType {
    Normal,
    EasyFlash,
    Final3,
    GameSystem,
    MagicDesk,
    SimonsBasic,
    OceanType1,
}

impl HwType {
    pub fn from(value: u8) -> HwType {
        match value {
            0 => HwType::Normal,
            3 => HwType::Final3,
            4 => HwType::SimonsBasic,
            5 => HwType::OceanType1,
            15 => HwType::GameSystem,
            19 => HwType::MagicDesk,
            32 => HwType::EasyFlash,
            _ => panic!("invalid hardware type {}", value),
        }
    }

    pub fn is_mirrowed(&self) -> bool {
        matches!(
            *self,
            HwType::OceanType1 | HwType::MagicDesk | HwType::Normal
        )
    }
}

pub struct IoConfig {
    pub exrom: bool,
    pub game: bool,
}

impl IoConfig {
    pub fn new() -> Self {
        IoConfig {
            exrom: true,
            game: true,
        }
    }
}

impl Default for IoConfig {
    fn default() -> Self {
        Self::new()
    }
}

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

#[allow(unused)]
pub struct Cartridge {
    version: u16,
    hw_type: HwType,
    exrom: bool,
    game: bool,
    banks: [Option<Chip>; 64],
    is_mirrowed: bool,
    // Runtime state
    bank_lo: Option<usize>,
    bank_hi: Option<usize>,
    io_config: IoConfig,
    reg_value: u8,
}

impl Cartridge {
    pub fn new(version: u16, hw_type: HwType, exrom: bool, game: bool) -> Self {
        Cartridge {
            version,
            hw_type,
            exrom,
            game,
            banks: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            is_mirrowed: hw_type.is_mirrowed(),
            bank_lo: None,
            bank_hi: None,
            io_config: IoConfig::new(),
            reg_value: 0,
        }
    }

    pub fn io_config(&self) -> &IoConfig {
        &self.io_config
    }

    pub fn add(&mut self, chip: Chip) {
        let bank_num = chip.bank_number as usize;
        self.banks[bank_num] = Some(chip);
    }

    pub fn reset(&mut self) {
        self.bank_lo = None;
        self.bank_hi = None;
        self.io_config = IoConfig {
            exrom: self.exrom,
            game: self.game,
        };
        if !self.banks.is_empty() {
            self.switch_bank(0);
        }
    }

    pub fn read(&mut self, address: u16) -> Option<u8> {
        match address {
            0x8000..=0x9fff => {
                if let Some(bank_num) = self.bank_lo {
                    let bank = self.banks[bank_num].as_ref().unwrap();
                    Some(bank.data[(address - 0x8000) as usize])
                } else {
                    None
                }
            }
            0xa000..=0xbfff => {
                if let Some(bank_num) = self.bank_hi {
                    let bank = self.banks[bank_num].as_ref().unwrap();
                    if bank.offset == 0x8000 {
                        Some(bank.data[(address - 0x8000) as usize])
                    } else {
                        Some(bank.data[(address - 0xa000) as usize])
                    }
                } else {
                    None
                }
            }
            0xde00..=0xdfff => Some(self.read_io(address)),
            _ => panic!("invalid address {:04x}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xde00..=0xdfff => self.write_io(address, value),
            _ => panic!("writes to cartridge are not supported"),
        }
    }

    fn read_io(&mut self, address: u16) -> u8 {
        if self.hw_type == HwType::GameSystem
            && let 0xde00..=0xdeff = address
        {
            self.switch_bank((address & 0x3f) as u8);
        }
        self.reg_value
    }

    fn write_io(&mut self, address: u16, value: u8) -> bool {
        self.reg_value = value;
        let mut io_changed = false;
        match self.hw_type {
            HwType::EasyFlash => {
                if address == 0xde00 {
                    self.switch_bank(value & 0x3f);
                }
            }
            HwType::Final3 => {
                if address == 0xde00 {
                    self.switch_bank(value - 0x40);
                }
            }
            HwType::MagicDesk => {
                if address == 0xde00 {
                    if !value.get_bit(7) {
                        self.switch_bank(value & 0x3f);
                        self.io_config.exrom = self.exrom;
                        self.io_config.game = self.game;
                        io_changed = true;
                    } else {
                        self.io_config.exrom = true;
                        self.io_config.game = true;
                        io_changed = true;
                    }
                }
            }
            HwType::Normal => {
                if address == 0xde00 {
                    self.switch_bank(value & 0x3f);
                }
            }
            HwType::OceanType1 => {
                if address == 0xde00 {
                    if value.get_bit(7) {
                        self.switch_bank(value & 0x3f);
                    } else {
                        panic!("should not be here");
                    }
                }
            }
            HwType::SimonsBasic if address == 0xde00 => {
                self.io_config.game = value == 0x01;
                io_changed = true;
            }
            _ => {}
        }
        io_changed
    }

    fn switch_bank(&mut self, bank_number: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cart::banks", "Switching to bank {} game {} exrom {}", bank_number, self.io_config.game, self.io_config.exrom);
        }
        if let Some(ref bank) = self.banks[bank_number as usize] {
            match bank.offset {
                0x8000 => {
                    self.bank_lo = Some(bank.bank_number as usize);
                    if self.is_mirrowed {
                        self.bank_hi = self.bank_lo;
                    } else {
                        self.bank_hi = None;
                    }
                }
                0xa000 => {
                    self.bank_hi = Some(bank.bank_number as usize);
                    if self.is_mirrowed {
                        self.bank_lo = self.bank_hi;
                    } else {
                        self.bank_lo = None;
                    }
                }
                _ => panic!("invalid load address {:04x}", bank.bank_number),
            }
        } else {
            panic!("invalid bank number {}", bank_number);
        }
    }
}
