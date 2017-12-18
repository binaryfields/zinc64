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

pub type Observer = Box<Fn(u8)>;

pub struct IoPort {
    direction: u8,
    latch: u8,
    value: u8,
    observer: Option<Observer>,
}

impl IoPort {
    pub fn new(direction: u8) -> IoPort {
        IoPort {
            direction,
            latch: 0,
            value: 0,
            observer: None,
        }
    }

    #[inline(always)]
    pub fn get_direction(&self) -> u8 {
        self.direction
    }

    #[inline(always)]
    pub fn get_value(&self) -> u8 {
        self.value
    }

    #[inline(always)]
    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction;
        // set input pins to 1
        self.value = self.latch | !self.direction;
        if let Some(ref observer) = self.observer {
            observer(self.value);
        }
    }

    pub fn set_observer(&mut self, observer: Observer) {
        self.observer = Some(observer);
    }

    #[inline(always)]
    pub fn set_value(&mut self, value: u8) {
        self.latch = value;
        // set input pins to 1
        self.value = self.latch | !self.direction;
        if let Some(ref observer) = self.observer {
            observer(self.value);
        }
    }

    pub fn reset(&mut self) {
        self.direction = 0x00;
        self.latch = 0x00;
        self.set_value(0x00);
    }
}
