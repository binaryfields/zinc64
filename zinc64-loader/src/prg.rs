// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use std::result::Result;

use byteorder::{LittleEndian, ReadBytesExt};
use zinc64::system::autostart;
use zinc64::system::{Autostart, AutostartMethod, Image, C64};

use super::Loader;

struct PrgImage {
    data: Vec<u8>,
    offset: u16,
}

impl Image for PrgImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting PRG image");
        c64.load(&self.data, self.offset);
    }

    fn unmount(&mut self, _c64: &mut C64) {}
}

pub struct PrgLoader {}

impl PrgLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl Loader for PrgLoader {
    fn autostart(&self, path: &Path) -> Result<AutostartMethod, io::Error> {
        let image = self.load(path)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, path: &Path) -> Result<Box<Image>, io::Error> {
        info!(target: "loader", "Loading PRG {}", path.to_str().unwrap());
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Box::new(PrgImage { data, offset }))
    }
}
