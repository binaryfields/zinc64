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

use util::bit;

const ACC_MASK: u32 = 0x00ffffff;

#[derive(Clone, Copy)]
pub struct Wave {
    pub frequency: u16,
    pub pulse_width: u16,
    // Control
    pub form: u8,
    pub ring: bool,
    pub sync: bool,
    pub test: bool,
    // Runtime State
    pub acc: u32,
    pub shift: u32,
}

impl Wave {
    pub fn new() -> Wave {
        Wave {
            frequency: 0,
            pulse_width: 0,
            form: 0,
            ring: false,
            sync: false,
            test: false,
            acc: 0,
            shift: 0,
        }
    }

    pub fn set_control(&mut self, value: u8) {
        self.sync = bit::bit_test(value, 1);
        self.ring = bit::bit_test(value, 2);
        self.test = bit::bit_test(value, 3);
        self.form = (value >> 4) & 0x0f;
        if self.test {
            self.acc = 0;
            self.shift = 0x7ffff8;
        }
    }

    pub fn reset(&mut self) {
        self.frequency = 0;
        self.pulse_width = 0;
        self.form = 0;
        self.ring = false;
        self.sync = false;
        self.test = false;
        self.acc = 0;
        self.shift = 0x7ffff8;
    }
}
