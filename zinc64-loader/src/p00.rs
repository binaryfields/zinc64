// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use byteorder::LittleEndian;
use core::str;
use zinc64_emu::system::autostart;
use zinc64_emu::system::{Autostart, AutostartMethod, Image, C64};

use super::Loader;
use crate::io::{self, ReadBytesExt, Reader};

static HEADER_SIG: &'static str = "C64File";

struct Header {
    signature: [u8; 7],
    #[allow(dead_code)]
    reserved_1: u8,
    #[allow(dead_code)]
    filename: [u8; 16],
    #[allow(dead_code)]
    reserved_2: u8,
    #[allow(dead_code)]
    reserved_3: u8,
}

struct P00Image {
    data: Vec<u8>,
    offset: u16,
}

impl Image for P00Image {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting P00 image");
        c64.load(&self.data, self.offset);
    }

    fn unmount(&mut self, _c64: &mut C64) {}
}

pub struct P00Loader;

impl P00Loader {
    pub fn new() -> impl Loader {
        Self {}
    }

    fn read_header(&self, rdr: &mut dyn Reader) -> io::Result<Header> {
        let mut signature = [0u8; 7];
        let mut filename = [0u8; 16];
        let header = Header {
            signature: {
                rdr.read_exact(&mut signature)?;
                signature
            },
            reserved_1: rdr.read_u8()?,
            filename: {
                rdr.read_exact(&mut filename)?;
                filename
            },
            reserved_2: rdr.read_u8()?,
            reserved_3: rdr.read_u8()?,
        };
        Ok(header)
    }

    fn validate_header(&self, header: &Header) -> io::Result<()> {
        let sig =
            str::from_utf8(&header.signature).map_err(|_| "invalid P00 signature".to_owned())?;
        if sig == HEADER_SIG {
            Ok(())
        } else {
            Err("invalid P00 signature".to_owned())
        }
    }
}

impl Loader for P00Loader {
    fn autostart(&self, reader: &mut dyn Reader) -> io::Result<AutostartMethod> {
        let image = self.load(reader)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, reader: &mut dyn Reader) -> io::Result<Box<dyn Image>> {
        info!(target: "loader", "Loading P00");
        let header = self
            .read_header(reader)
            .map_err(|_| "invalid P00 header".to_owned())?;
        self.validate_header(&header)?;
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Box::new(P00Image { data, offset }))
    }
}
