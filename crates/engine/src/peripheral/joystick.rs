// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Button {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    Fire = 4,
}

impl Button {
    pub fn bit(self) -> usize {
        self as usize
    }
}

// DEFERRED device: joystick test cases

pub struct Joystick {
    state: u8,
}

impl Joystick {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }

    pub fn press(&mut self, button: Button) {
        self.state.set_bit(button.bit(), true);
    }

    pub fn release(&mut self, button: Button) {
        self.state.set_bit(button.bit(), false);
    }

    pub fn port_lines(&self) -> u8 {
        !self.state
    }
}

impl Default for Joystick {
    fn default() -> Self {
        Self::new()
    }
}
