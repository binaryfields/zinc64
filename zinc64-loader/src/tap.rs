// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec;
use byteorder::LittleEndian;
use core::str;
use zinc64_emu::device::Tape;
use zinc64_emu::system::autostart;
use zinc64_emu::system::{Autostart, AutostartMethod, Image, C64};

use super::Loader;
use crate::io::{self, ReadBytesExt, Reader};

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

pub struct TapLoader;

impl TapLoader {
    pub fn new() -> impl Loader {
        Self {}
    }

    fn read_header(&self, rdr: &mut dyn Reader) -> io::Result<Header> {
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
            .map_err(|_| "invalid cartridge signature".to_owned())?;
        if sig == HEADER_SIG {
            Ok(())
        } else {
            Err("invalid cartridge signature".to_owned())
        }
    }
}

impl Loader for TapLoader {
    fn autostart(&self, reader: &mut dyn Reader) -> io::Result<AutostartMethod> {
        let image = self.load(reader)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, reader: &mut dyn Reader) -> io::Result<Box<dyn Image>> {
        info!(target: "loader", "Loading TAP");
        let header = self
            .read_header(reader)
            .map_err(|_| "invalid tape header".to_owned())?;
        info!(target: "loader", "Found tape, version {}, size {}", header.version, header.size);
        self.validate_header(&header)?;
        let mut data = vec![0; header.size as usize];
        reader.read_exact(&mut data)?;
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
