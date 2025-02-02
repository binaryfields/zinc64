// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::{self, Read};

use byteorder::{LittleEndian, ReadBytesExt};
use zinc64_engine::media::Mode;
use zinc64_engine::{Autostart, AutostartMethod, Media};

use crate::Loader;

#[derive(Default)]
pub struct PrgLoader;

impl PrgLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl Loader for PrgLoader {
    fn autostart(&self, reader: &mut dyn Read) -> io::Result<AutostartMethod> {
        let media = self.load(reader)?;
        Ok(AutostartMethod::Run(Autostart::new(Mode::Run, media)))
    }

    fn load(&self, reader: &mut dyn Read) -> io::Result<Media> {
        info!(target: "loader", "Loading PRG");
        let offset = reader.read_u16::<LittleEndian>()?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        info!(target: "loader", "Program offset 0x{:x}, size {}", offset, data.len());
        Ok(Media::Program { data, offset })
    }
}
