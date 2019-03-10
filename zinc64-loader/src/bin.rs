// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use zinc64_emu::system::{AutostartMethod, Image, C64};

use super::Loader;
use crate::io::{self, Reader};

struct BinImage {
    data: Vec<u8>,
    offset: u16,
}

impl Image for BinImage {
    fn mount(&mut self, c64: &mut C64) {
        info!(target: "loader", "Mounting BIN image");
        c64.get_cpu_mut().write(0x0001, 0);
        c64.load(&self.data, self.offset);
        c64.get_cpu_mut().set_pc(self.offset);
    }

    fn unmount(&mut self, _c64: &mut C64) {}
}

pub struct BinLoader {
    offset: u16,
}

impl BinLoader {
    pub fn new(offset: u16) -> Self {
        Self { offset }
    }
}

impl Loader for BinLoader {
    fn autostart(&self, reader: &mut dyn Reader) -> io::Result<AutostartMethod> {
        let image = self.load(reader)?;
        Ok(AutostartMethod::WithBinImage(image))
    }

    fn load(&self, reader: &mut dyn Reader) -> io::Result<Box<dyn Image>> {
        info!(target: "loader", "Loading BIN");
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Box::new(BinImage {
            data,
            offset: self.offset,
        }))
    }
}
