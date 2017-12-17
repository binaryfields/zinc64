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

pub struct Pulse {
    low_cycles: u32,
    remaining_cycles: u32,
}

impl Pulse {
    pub fn new(length: u32, duty: u32) -> Pulse {
        Pulse {
            low_cycles: length * (100 - duty) / 100,
            remaining_cycles: length,
        }
    }

    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.remaining_cycles == 0
    }

    #[inline(always)]
    pub fn advance(&mut self) -> bool {
        self.remaining_cycles -= 1;
        if self.low_cycles == 0 {
            true
        } else {
            self.low_cycles -= 1;
            false
        }
    }
}
