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

    #[allow(unused_variables)]
    pub fn on_button_down(&mut self, button_idx: u8) {
        self.button = true;
    }

    #[allow(unused_variables)]
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