// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec;
use byteorder::BigEndian;
use core::str;
use zinc64_emu::device::{Cartridge, Chip, ChipType, HwType};
use zinc64_emu::system::autostart;
use zinc64_emu::system::{Image, C64};

use crate::io::{self, Reader, ReadBytesExt};
use super::Loader;

// SPEC: http://ist.uwaterloo.ca/~schepers/formats/CRT.TXT

static HEADER_SIG: &'static str = "C64 CARTRIDGE   ";
static CHIP_SIG: &'static str = "CHIP";

struct Header {
    signature: [u8; 16],
    header_length: u32,
    version: u16,
    hw_type: u16,
    exrom_line: u8,
    game_line: u8,
    #[allow(dead_code)]
    reserved: [u8; 6],
    name: [u8; 32],
}

struct ChipHeader {
    signature: [u8; 4],
    length: u32,
    chip_type: u16,
    bank_number: u16,
    load_address: u16,
    image_size: u16,
}

struct CrtImage {
    cartridge: Option<Cartridge>,
}

impl Image for CrtImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting CRT image");
        c64.attach_cartridge(self.cartridge.take().unwrap());
    }
    fn unmount(&mut self, c64: &mut C64) {
        c64.detach_cartridge();
    }
}

pub struct CrtLoader;

impl CrtLoader {
    pub fn new() -> impl Loader {
        Self {}
    }

    fn build_cartridge(&self, header: &Header) -> Cartridge {
        Cartridge {
            version: header.version,
            hw_type: HwType::from(header.hw_type as u8),
            exrom: header.exrom_line != 0,
            game: header.game_line != 0,
            banks: Vec::new(),
            bank_lo: 0,
            bank_hi: 0,
        }
    }

    fn build_chip(&self, header: &ChipHeader, data: Vec<u8>) -> Chip {
        Chip {
            chip_type: ChipType::from(header.chip_type),
            bank_number: header.bank_number as u8,
            offset: header.load_address,
            size: header.image_size,
            data,
        }
    }

    fn read_chip_header(&self, rdr: &mut dyn Reader) -> io::Result<Option<ChipHeader>> {
        let mut signature = [0u8; 4];
        match rdr.read(&mut signature)? {
            0 => Ok(None),
            4 => {
                let header = ChipHeader {
                    signature,
                    length: rdr.read_u32::<BigEndian>()?,
                    chip_type: rdr.read_u16::<BigEndian>()?,
                    bank_number: rdr.read_u16::<BigEndian>()?,
                    load_address: rdr.read_u16::<BigEndian>()?,
                    image_size: rdr.read_u16::<BigEndian>()?,
                };
                Ok(Some(header))
            }
            size => Err(format!("chip header error, expected {} got {}", 4, size)),
        }
    }

    fn read_data(&self, rdr: &mut dyn Reader, length: usize) -> io::Result<Vec<u8>> {
        let mut data = vec![0; length];
        rdr.read_exact(&mut data)?;
        Ok(data)
    }

    fn read_header(&self, rdr: &mut dyn Reader) -> io::Result<Header> {
        let mut signature = [0u8; 16];
        let mut reserved = [0u8; 6];
        let mut name = [0u8; 32];
        let header = Header {
            signature: {
                rdr.read_exact(&mut signature)?;
                signature
            },
            header_length: rdr.read_u32::<BigEndian>()?,
            version: rdr.read_u16::<BigEndian>()?,
            hw_type: rdr.read_u16::<BigEndian>()?,
            exrom_line: rdr.read_u8()?,
            game_line: rdr.read_u8()?,
            reserved: {
                rdr.read_exact(&mut reserved)?;
                reserved
            },
            name: {
                rdr.read_exact(&mut name)?;
                name
            },
        };
        Ok(header)
    }

    fn validate_chip_header(&self, header: &ChipHeader) -> io::Result<()> {
        let sig = str::from_utf8(&header.signature)
            .map_err(|_| "invalid chip signature".to_owned())?;
        if sig == CHIP_SIG {
            Ok(())
        } else {
            Err("invalid chip signature".to_owned())
        }
    }

    fn validate_header(&self, header: &Header) -> io::Result<()> {
        let sig = str::from_utf8(&header.signature)
            .map_err(|_| "invalid cartridge signature".to_owned())?;
        if sig == HEADER_SIG {
            Ok(())
        } else {
            Err("invalid cartridge signature".to_owned())
        }
    }
}

impl Loader for CrtLoader {
    fn autostart(&self, reader: &mut dyn Reader) -> io::Result<autostart::AutostartMethod> {
        let image = self.load(reader)?;
        Ok(autostart::AutostartMethod::WithImage(image))
    }

    fn load(&self, reader: &mut dyn Reader) -> io::Result<Box<dyn Image>> {
        info!(target: "loader", "Loading CRT");
        let header = self
            .read_header(reader)
            .map_err(|_| "invalid cartridge header".to_owned())?;
        info!(target: "loader", "Found cartridge {}, version {}.{}, type {}",
              str::from_utf8(&header.name).unwrap_or(""),
              header.version >> 8,
              header.version & 0xff,
              header.hw_type);
        self.validate_header(&header)?;
        reader.consume((header.header_length - 0x40) as usize);
        let mut cartridge = self.build_cartridge(&header);
        loop {
            let chip_header_opt = self
                .read_chip_header(reader)
                .map_err(|_| "invalid cartridge chip header".to_owned())?;
            match chip_header_opt {
                Some(chip_header) => {
                    info!(target: "loader", "Found chip {}, offset 0x{:x}, size {}",
                          chip_header.bank_number, chip_header.load_address, chip_header.length - 0x10);
                    self.validate_chip_header(&chip_header)?;
                    let chip_data = self
                        .read_data(reader, (chip_header.length - 0x10) as usize)
                        .map_err(|_| format!("invalid cartridge chip {} data", chip_header.bank_number))?;
                    let chip = self.build_chip(&chip_header, chip_data);
                    cartridge.add(chip);
                }
                None => {
                    break;
                }
            }
        }
        Ok(Box::new(CrtImage {
            cartridge: Some(cartridge),
        }))
    }
}
