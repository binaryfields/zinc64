/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
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
