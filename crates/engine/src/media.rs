// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::c64::C64;
use crate::peripheral::{Cartridge, Tape};

pub enum Media {
    Program { data: Vec<u8>, offset: u16 },
    Binary { data: Vec<u8>, offset: u16 },
    Cartridge(Box<Cartridge>),
    Tape(Box<dyn Tape>),
}

pub struct Autostart {
    mode: Mode,
    media: Media,
}

impl Autostart {
    pub fn new(mode: Mode, media: Media) -> Autostart {
        Autostart { mode, media }
    }

    pub fn execute(self, c64: &mut C64) {
        let command = self.command().to_string() + "\n";
        c64.mount(self.media);
        c64.get_keyboard().enqueue(&command);
    }

    fn command(&self) -> &str {
        match self.mode {
            Mode::Run => "RUN",
        }
    }
}

pub enum Mode {
    Run,
}

pub enum AutostartMethod {
    Boot(Media),
    Jump(Media),
    Run(Autostart),
}

impl AutostartMethod {
    pub fn execute(self, c64: &mut C64) {
        match self {
            AutostartMethod::Boot(media) => {
                c64.mount(media);
                c64.reset(false);
            }
            AutostartMethod::Jump(media) => c64.mount(media),
            AutostartMethod::Run(autostart) => {
                c64.set_autostart(Some(autostart));
                c64.reset(false);
            }
        }
    }
}
