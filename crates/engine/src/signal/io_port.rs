// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

// direction - (where 1 is an output, and 0 is an input).

pub struct IoPort {
    direction: u8,
    input: u8,
    output: u8,
}

impl IoPort {
    pub fn new(direction: u8, input: u8) -> Self {
        Self {
            direction,
            input,
            output: 0,
        }
    }

    pub fn get_direction(&self) -> u8 {
        self.direction
    }

    pub fn reset(&mut self) {
        self.direction = 0x00;
        self.input = 0xff;
        self.output = 0x00;
    }

    pub fn get_value(&self) -> u8 {
        (self.output & self.direction) | (self.input & !self.direction)
    }

    pub fn get_value_2(&self, input: u8) -> u8 {
        (self.output & self.direction) | (input & !self.direction)
    }

    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction;
    }

    pub fn set_input(&mut self, value: u8) {
        self.input = value;
    }

    pub fn set_input_bit(&mut self, bit: usize, value: bool) {
        self.input.set_bit(bit, value);
    }

    pub fn set_value(&mut self, value: u8) {
        self.output = value;
    }
}
