// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::fs::File;
use std::io;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::Path;
use std::result::Result;
use std::str;

use byteorder::{LittleEndian, ReadBytesExt};
use zinc64::device::Tape;
use zinc64::system::autostart;
use zinc64::system::{Autostart, AutostartMethod, Image, C64};

use super::Loader;

// SPEC: http://ist.uwaterloo.ca/~schepers/formats/TAP.TXT

static HEADER_SIG: &'static str = "C64-TAPE-RAW";

struct Header {
    signature: [u8; 12],
    version: u8,
    #[allow(dead_code)]
    reserved: [u8; 3],
    size: u32,
}

struct TapImage {
    tape: Option<Box<dyn Tape>>,
}

impl Image for TapImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting TAP image");
        c64.attach_tape(self.tape.take().unwrap());
    }

    fn unmount(&mut self, c64: &mut C64) {
        c64.detach_tape();
    }
}

pub struct TapLoader {}

impl TapLoader {
    pub fn new() -> Self {
        Self {}
    }

    fn read_header(&self, rdr: &mut dyn Read) -> io::Result<Header> {
        let mut signature = [0u8; 12];
        let mut reserved = [0u8; 3];
        let header = Header {
            signature: {
                rdr.read_exact(&mut signature)?;
                signature
            },
            version: rdr.read_u8()?,
            reserved: {
                rdr.read_exact(&mut reserved)?;
                reserved
            },
            size: rdr.read_u32::<LittleEndian>()?,
        };
        Ok(header)
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

impl Loader for TapLoader {
    fn autostart(&self, path: &Path) -> Result<AutostartMethod, io::Error> {
        let image = self.load(path)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, path: &Path) -> Result<Box<dyn Image>, io::Error> {
        info!(target: "loader", "Loading TAP {}", path.to_str().unwrap());
        let file = File::open(path)?;
        let mut rdr = BufReader::new(file);
        let header = self
            .read_header(&mut rdr)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid tape header"))?;
        info!(target: "loader", "Found tape, version {}, size {}", header.version, header.size);
        self.validate_header(&header)?;
        let mut data = vec![0; header.size as usize];
        rdr.read_exact(&mut data)?;
        let tape = TapTape {
            version: header.version,
            data,
            pos: 0,
        };
        Ok(Box::new(TapImage {
            tape: Some(Box::new(tape)),
        }))
    }
}

struct TapTape {
    version: u8,
    data: Vec<u8>,
    pos: usize,
}

impl Tape for TapTape {
    fn read_pulse(&mut self) -> Option<u32> {
        if self.pos < self.data.len() {
            let value = self.data[self.pos] as u32;
            self.pos += 1;
            if value != 0 {
                Some(value << 3)
            } else {
                let pulse = match self.version {
                    0 => 256 << 3,
                    1 => {
                        let byte1 = self.data[self.pos] as u32;
                        let byte2 = self.data[self.pos + 1] as u32;
                        let byte3 = self.data[self.pos + 2] as u32;
                        self.pos += 3;
                        (byte3 << 16) | (byte2 << 8) | byte1
                    }
                    _ => panic!("invalid version {}", self.version),
                };
                Some(pulse)
            }
        } else {
            None
        }
    }

    fn seek(&mut self, pos: usize) -> bool {
        if pos < self.data.len() {
            self.pos = pos;
            true
        } else {
            false
        }
    }
}
