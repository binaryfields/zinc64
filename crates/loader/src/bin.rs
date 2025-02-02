// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::{self, Read};

use zinc64_engine::{AutostartMethod, Media};

use crate::Loader;

pub struct BinLoader {
    offset: u16,
}

impl BinLoader {
    pub fn new(offset: u16) -> Self {
        Self { offset }
    }
}

impl Loader for BinLoader {
    fn autostart(&self, reader: &mut dyn Read) -> io::Result<AutostartMethod> {
        let media = self.load(reader)?;
        Ok(AutostartMethod::Jump(media))
    }

    fn load(&self, reader: &mut dyn Read) -> io::Result<Media> {
        info!(target: "loader", "Loading BIN");
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Media::Binary {
            data,
            offset: self.offset,
        })
    }
}
