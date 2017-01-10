/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use c64::C64;
use loader::Image;

pub enum Mode {
    Run,
}

pub struct Autostart {
    mode: Mode,
    image: Box<Image>,
}

impl Autostart {
    pub fn new(mode: Mode, image: Box<Image>) -> Autostart {
        Autostart {
            mode: mode,
            image: image,
        }
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

pub enum Method {
    WithImage(Box<Image>),
    WithBinImage(Box<Image>),
    WithAutostart(Option<Autostart>),
}

impl Method {
    pub fn execute(&mut self, c64: &mut C64) {
        match *self {
            Method::WithImage(ref mut image) => {
                image.mount(c64);
                c64.reset();
            },
            Method::WithBinImage(ref mut image) => {
                image.mount(c64);
            },
            Method::WithAutostart(ref mut autostart) => {
                c64.set_autostart(autostart.take());
                c64.reset();
            },
        }
    }
}
