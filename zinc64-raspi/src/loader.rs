// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use core::result::Result;

use byteorder::{ByteOrder, LittleEndian};
use zinc64_emu::system::autostart;
use zinc64_emu::system::{Autostart, AutostartMethod, Image, C64};

struct PrgImage {
    data: Vec<u8>,
    offset: u16,
}

impl Image for PrgImage {
    fn mount(&mut self, c64: &mut C64) {
        info!("Mounting PRG image");
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

impl PrgLoader {
    pub fn autostart(&self, buf: &[u8]) -> Result<AutostartMethod, &'static str> {
        let image = self.load(buf)?;
        let autostart = Autostart::new(autostart::Mode::Run, image);
        Ok(AutostartMethod::WithAutostart(Some(autostart)))
    }

    fn load(&self, buf: &[u8]) -> Result<Box<dyn Image>, &'static str> {
        info!("Loading PRG image");
        let offset = LittleEndian::read_u16(buf);
        let data = Vec::from(&buf[2..]);
        info!("Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Box::new(PrgImage { data, offset }))
    }
}
