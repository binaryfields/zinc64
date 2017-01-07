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

use sdl2::keyboard::Keycode;

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    None = 0xff,
    Numpad = 0xfe,
    Joy0 = 0,
    Joy1 = 1,
}

impl Mode {
    pub fn from(mode: &str) -> Mode {
        match mode {
            "none" => Mode::None,
            "numpad" => Mode::Numpad,
            "joy0" => Mode::Joy0,
            "joy1" => Mode::Joy1,
            _ => panic!("invalid mode {}", mode),
        }
    }

    pub fn index(&self) -> u8 {
        *self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Negative,
    Off,
    Positive,
}

pub struct Joystick {
    mode: Mode,
    threshold: i16,
    x_axis: Motion,
    y_axis: Motion,
    button: bool,
}

impl Joystick {
    pub fn new(mode: Mode, threshold: i16) -> Joystick {
        Joystick {
            mode: mode,
            threshold: threshold,
            x_axis: Motion::Off,
            y_axis: Motion::Off,
            button: false,
        }
    }

    pub fn get_button(&self) -> bool {
        self.button
    }

    pub fn get_index(&self) -> u8 {
        self.mode.index()
    }

    pub fn get_x_axis(&self) -> Motion {
        self.x_axis
    }

    pub fn get_y_axis(&self) -> Motion {
        self.y_axis
    }

    pub fn is_virtual(&self) -> bool {
        self.mode == Mode::Numpad
    }

    // -- Events

    pub fn on_axis_motion(&mut self, axis_idx: u8, value: i16) {
        match axis_idx {
            0 if value < -self.threshold => self.x_axis = Motion::Negative,
            0 if value > self.threshold => self.x_axis = Motion::Positive,
            0 => self.x_axis = Motion::Off,
            1 if value < -self.threshold => self.y_axis = Motion::Negative,
            1 if value > self.threshold => self.y_axis = Motion::Positive,
            1 => self.y_axis = Motion::Off,
            _ => panic!("invalid axis {}", axis_idx),
        }
    }

    pub fn on_button_down(&mut self, button_idx: u8) {
        self.button = true;
    }

    pub fn on_button_up(&mut self, button_idx: u8) {
        self.button = false;
    }

    pub fn on_key_down(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::Kp4 => self.x_axis = Motion::Negative,
            Keycode::Kp6 => self.x_axis = Motion::Positive,
            Keycode::Kp2 => self.y_axis = Motion::Negative,
            Keycode::Kp8 => self.y_axis = Motion::Positive,
            Keycode::KpEnter => self.button = true,
            _ => {},
        }
    }

    pub fn on_key_up(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::Kp4 => self.x_axis = Motion::Off,
            Keycode::Kp6 => self.x_axis = Motion::Off,
            Keycode::Kp2 => self.y_axis = Motion::Off,
            Keycode::Kp8 => self.y_axis = Motion::Off,
            Keycode::KpEnter => self.button = false,
            _ => {},
        }
    }
}