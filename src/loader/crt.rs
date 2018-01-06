/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read};
use std::path::Path;
use std::result::Result;
use std::str;

use byteorder::{BigEndian, ReadBytesExt};
use device::{Cartridge, Chip, ChipType, HwType};
use system::{C64, Image};
use system::autostart;

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
    #[allow(dead_code)] reserved: [u8; 6],
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

pub struct CrtLoader {}

impl CrtLoader {
    pub fn new() -> CrtLoader {
        CrtLoader {}
    }

    fn build_cartridge(&self, header: Header) -> Cartridge {
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

    fn build_chip(&self, header: ChipHeader, data: Vec<u8>) -> Chip {
        Chip {
            chip_type: ChipType::from(header.chip_type),
            bank_number: header.bank_number as u8,
            offset: header.load_address,
            size: header.image_size,
            data,
        }
    }

    fn read_chip_header(&self, rdr: &mut Read) -> io::Result<Option<ChipHeader>> {
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
            size => Err(Error::new(
                ErrorKind::UnexpectedEof,
                format!("chip header error, expected {} got {}", 4, size),
            )),
        }
    }

    fn read_data(&self, rdr: &mut Read, length: usize) -> io::Result<Vec<u8>> {
        let mut data = vec![0; length];
        rdr.read_exact(&mut data)?;
        Ok(data)
    }

    fn read_header(&self, rdr: &mut Read) -> io::Result<Header> {
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
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid chip signature"))?;
        if sig == CHIP_SIG {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, "invalid chip signature"))
        }
    }

    fn validate_header(&self, header: &Header) -> io::Result<()> {
        let sig = str::from_utf8(&header.signature)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid cartridge signature"))?;
        if sig == HEADER_SIG {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "invalid cartridge signature",
            ))
        }
    }
}

impl Loader for CrtLoader {
    fn autostart(&self, path: &Path) -> Result<autostart::AutostartMethod, io::Error> {
        let image = self.load(path)?;
        Ok(autostart::AutostartMethod::WithImage(image))
    }

    fn load(&self, path: &Path) -> Result<Box<Image>, io::Error> {
        info!(target: "loader", "Loading CRT {}", path.to_str().unwrap());
        let file = File::open(path)?;
        let mut rdr = BufReader::new(file);
        let header = self.read_header(&mut rdr)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid cartridge header"))?;
        info!(target: "loader", "Found cartridge {}, version {}.{}, type {}",
              str::from_utf8(&header.name).unwrap_or(""),
              header.version >> 8,
              header.version & 0xff,
              header.hw_type);
        self.validate_header(&header)?;
        rdr.consume((header.header_length - 0x40) as usize);
        let mut cartridge = self.build_cartridge(header);
        loop {
            let chip_header_opt = self.read_chip_header(&mut rdr)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid cartridge chip header"))?;
            match chip_header_opt {
                Some(chip_header) => {
                    info!(target: "loader", "Found chip {}, offset 0x{:x}, size {}",
                          chip_header.bank_number, chip_header.load_address, chip_header.length - 0x10);
                    self.validate_chip_header(&chip_header)?;
                    let chip_data = self.read_data(&mut rdr, (chip_header.length - 0x10) as usize)
                        .map_err(|_| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!("invalid cartridge chip {} data", chip_header.bank_number),
                            )
                        })?;
                    let chip = self.build_chip(chip_header, chip_data);
                    cartridge.add(chip);
                }
                None => {
                    break;
                }
            }
        }
        Ok(Box::new(
            CrtImage {
                cartridge: Some(cartridge),
            }
        ))
    }
}
