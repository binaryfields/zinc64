// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use zinc64_emu::system::C64;
use zinc64_loader::Loaders;

use crate::util::FileReader;

pub struct ConsoleApp {
    c64: C64,
}

impl ConsoleApp {
    pub fn new(c64: C64) -> Self {
        Self { c64 }
    }

    pub fn load_image(&mut self, path: &Path) -> Result<(), String> {
        let ext = path.extension().map(|s| s.to_str().unwrap());
        let loader = Loaders::from_ext(ext)?;
        let file = File::open(path).map_err(|err| format!("{}", err))?;
        let mut reader = FileReader(BufReader::new(file));
        let mut autostart = loader.autostart(&mut reader)?;
        autostart.execute(&mut self.c64);
        Ok(())
    }

    pub fn run(&mut self) {
        loop {
            self.c64.run_frame();
            self.c64.reset_vsync();
            if self.c64.is_cpu_jam() {
                warn!(target: "main", "CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
                break;
            }
        }
    }
}
