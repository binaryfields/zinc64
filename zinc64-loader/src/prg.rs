// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use byteorder::LittleEndian;
use zinc64_system::autostart;
use zinc64_system::{Autostart, AutostartMethod, Image, C64};

use super::Loader;
use crate::io::{self, ReadBytesExt, Reader};

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

pub struct PrgLoader;

impl PrgLoader {
    pub fn new() -> impl Loader {
        Self {}
    }
}

impl Loader for PrgLoader {
    fn autostart(&self, reader: &mut dyn Reader) -> io::Result<AutostartMethod> {
        let image = self.load(reader)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, reader: &mut dyn Reader) -> io::Result<Box<dyn Image>> {
        info!(target: "loader", "Loading PRG");
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Box::new(PrgImage { data, offset }))
    }
}
