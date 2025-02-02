// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::result::Result;

use zinc64_engine::C64;
use zinc64_loader::Loaders;

use super::Handler;

pub struct LoadCommand<'a> {
    c64: &'a mut C64,
    path: String,
}

impl<'a> LoadCommand<'a> {
    pub fn new(c64: &'a mut C64, path: String) -> Self {
        Self { c64, path }
    }

    pub fn help() -> &'static str {
        "load <image_path>"
    }
}

impl<'a> Handler for LoadCommand<'a> {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        let path = Path::new(&self.path);
        let ext = path.extension().map(|s| s.to_str().unwrap());
        let loader = Loaders::from_ext(ext).map_err(|err| err.to_string())?;
        let file = File::open(path).map_err(|err| err.to_string())?;
        let mut reader = BufReader::new(file);
        let autostart = loader
            .autostart(&mut reader)
            .map_err(|err| err.to_string())?;
        autostart.execute(self.c64);
        out.write("Loaded image.\n".as_bytes())
            .map_err(|err| format!("{}", err))?;
        Ok(())
    }
}
