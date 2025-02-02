// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::{self, Read};
use std::str;

use byteorder::{LittleEndian, ReadBytesExt};
use zinc64_engine::media::Mode;
use zinc64_engine::{Autostart, AutostartMethod, Media};

use crate::{Loader, invalid_data};

static HEADER_SIG: &str = "C64File";

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

#[derive(Default)]
pub struct P00Loader;

impl P00Loader {
    pub fn new() -> Self {
        Self {}
    }

    fn read_header(&self, rdr: &mut dyn Read) -> io::Result<Header> {
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
            str::from_utf8(&header.signature).map_err(|_| invalid_data("invalid P00 signature"))?;
        if sig == HEADER_SIG {
            Ok(())
        } else {
            Err(invalid_data("invalid P00 signature"))
        }
    }
}

impl Loader for P00Loader {
    fn autostart(&self, reader: &mut dyn Read) -> io::Result<AutostartMethod> {
        let media = self.load(reader)?;
        Ok(AutostartMethod::Run(Autostart::new(Mode::Run, media)))
    }

    fn load(&self, reader: &mut dyn Read) -> io::Result<Media> {
        info!(target: "loader", "Loading P00");
        let header = self
            .read_header(reader)
            .map_err(|_| invalid_data("invalid P00 header"))?;
        self.validate_header(&header)?;
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Media::Program { data, offset })
    }
}
