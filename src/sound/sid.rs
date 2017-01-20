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

// SPEC: http://www.oxyron.de/html/registers_sid.html

pub struct Sid {}

impl Sid {
    pub fn new() -> Sid {
        Sid {}
    }

    pub fn reset(&mut self) {}

    // -- Device I/O

    pub fn read(&mut self, reg: u8) -> u8 {
        0
    }

    pub fn write(&mut self, reg: u8, value: u8) {}
}