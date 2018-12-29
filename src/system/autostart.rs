// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::C64;

pub trait Image {
    fn mount(&mut self, c64: &mut C64);
    fn unmount(&mut self, c64: &mut C64);
}

pub enum Mode {
    Run,
}

pub struct Autostart {
    mode: Mode,
    image: Box<dyn Image>,
}

impl Autostart {
    pub fn new(mode: Mode, image: Box<dyn Image>) -> Autostart {
        Autostart { mode, image }
    }

    pub fn execute(&mut self, c64: &mut C64) {
        self.image.mount(c64);
        let keyboard = c64.get_keyboard();
        let command = self.get_command().to_string() + "\n";
        keyboard.borrow_mut().enqueue(&command);
    }

    fn get_command(&self) -> &str {
        match self.mode {
            Mode::Run => "RUN",
        }
    }
}

pub enum AutostartMethod {
    WithImage(Box<dyn Image>),
    WithBinImage(Box<dyn Image>),
    WithAutostart(Option<Autostart>),
}

impl AutostartMethod {
    pub fn execute(&mut self, c64: &mut C64) {
        match *self {
            AutostartMethod::WithImage(ref mut image) => {
                image.mount(c64);
                c64.reset(false);
            }
            AutostartMethod::WithBinImage(ref mut image) => {
                image.mount(c64);
            }
            AutostartMethod::WithAutostart(ref mut autostart) => {
                c64.set_autostart(autostart.take());
                c64.reset(false);
            }
        }
    }
}
