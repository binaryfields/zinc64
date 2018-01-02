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

use std::cell::Cell;
use std::rc::Rc;

use bit_field::BitField;

// TODO device: joystick test cases

#[derive(Clone, Copy, PartialEq)]
pub enum Button {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    Fire = 4,
}

impl Button {
    pub fn bit(&self) -> usize {
        *self as usize
    }
}

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

pub struct Joystick {
    // Configuration
    mode: Mode,
    threshold: i16,
    // State
    state: Rc<Cell<u8>>,
}

impl Joystick {
    pub fn new(mode: Mode, threshold: i16, state: Rc<Cell<u8>>) -> Joystick {
        Joystick {
            mode,
            threshold,
            state,
        }
    }

    pub fn get_index(&self) -> u8 {
        self.mode.index()
    }

    pub fn is_virtual(&self) -> bool {
        self.mode == Mode::Numpad
    }

    pub fn reset(&mut self) {
        self.state.set(0);
    }

    fn set_state(&mut self, bit: usize, value: bool) {
        let mut new_state = self.state.get();
        new_state.set_bit(bit, value);
        self.state.set(new_state);
    }

    // -- Event Handlers

    pub fn on_axis_motion(&mut self, axis_idx: u8, value: i16) {
        match axis_idx {
            0 if value < -self.threshold => {
                self.set_state(Button::Left.bit(), true);
                self.set_state(Button::Right.bit(), false);
            },
            0 if value > self.threshold => {
                self.set_state(Button::Left.bit(), false);
                self.set_state(Button::Right.bit(), true);
            },
            0 => {
                self.set_state(Button::Left.bit(), false);
                self.set_state(Button::Right.bit(), false);
            },
            1 if value < -self.threshold => {
                self.set_state(Button::Up.bit(), false);
                self.set_state(Button::Down.bit(), true);
            },
            1 if value > self.threshold => {
                self.set_state(Button::Up.bit(), true);
                self.set_state(Button::Down.bit(), false);
            },
            1 => {
                self.set_state(Button::Up.bit(), false);
                self.set_state(Button::Down.bit(), false);
            },
            _ => panic!("invalid axis {}", axis_idx),
        }
    }

    pub fn on_button_down(&mut self, _button_idx: u8) {
        self.set_state(Button::Fire.bit(), true);
    }

    pub fn on_button_up(&mut self, _button_idx: u8) {
        self.set_state(Button::Fire.bit(), false);
    }

    pub fn on_key_down(&mut self, keycode: Button) {
        self.set_state(keycode.bit(), true);
    }

    pub fn on_key_up(&mut self, keycode: Button) {
        self.set_state(keycode.bit(), false);
    }
}
