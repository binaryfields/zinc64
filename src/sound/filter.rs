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

pub struct Filter {
    pub mode: u8,
    pub fc: u16,
    pub filt: u8,
    pub resonance: u8,
    pub volume: u8,
}

impl Filter {
    pub fn new() -> Filter {
        Filter {
            mode: 0,
            fc: 0,
            filt: 0,
            resonance: 0,
            volume: 0,
        }
    }

    pub fn reset(&mut self) {
        self.mode = 0;
        self.fc = 0;
        self.filt = 0;
        self.resonance = 0;
        self.volume = 0;
    }
}
