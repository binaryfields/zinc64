/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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

use bit_field::BitField;

pub type Observer = Box<Fn(u8)>;

// direction - (where 1 is an output, and 0 is an input).

pub struct IoPort {
    direction: u8,
    input: u8,
    output: u8,
    observer: Option<Observer>,
}

impl IoPort {
    pub fn new(direction: u8, input: u8) -> IoPort {
        IoPort {
            direction,
            input,
            output: 0,
            observer: None,
        }
    }

    #[inline]
    pub fn get_direction(&self) -> u8 {
        self.direction
    }

    #[inline]
    pub fn get_value(&self) -> u8 {
        (self.output & self.direction) | (self.input & !self.direction)
    }

    #[inline]
    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction;
        self.notify_observer();
    }

    #[inline]
    pub fn set_input(&mut self, value: u8) {
        self.input = value;
        self.notify_observer();
    }

    #[inline]
    pub fn set_input_bit(&mut self, bit: usize, value: bool) {
        self.input.set_bit(bit, value);
        self.notify_observer();
    }

    #[inline]
    pub fn set_observer(&mut self, observer: Observer) {
        self.observer = Some(observer);
    }

    #[inline]
    pub fn set_value(&mut self, value: u8) {
        self.output = value;
        self.notify_observer();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.direction = 0x00;
        self.input = 0xff;
        self.output = 0x00;
        self.notify_observer();
    }

    #[inline]
    fn notify_observer(&self) {
        if let Some(ref observer) = self.observer {
            observer(self.get_value());
        }
    }
}
