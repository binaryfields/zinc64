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

use bit_field::BitField;
use log::LogLevel;

pub struct IrqLine {
    kind: &'static str,
    line: u8,
}

impl IrqLine {
    pub fn new(kind: &'static str) -> IrqLine {
        IrqLine {
            kind,
            line: 0,
        }
    }

    #[inline]
    pub fn clear(&mut self, source: usize) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Clear {}, source {:?}", self.kind, source);
        }
        self.line.set_bit(source, false);
    }

    #[inline]
    pub fn is_low(&self) -> bool {
        self.line != 0
    }

    #[inline]
    pub fn reset(&mut self) {
        self.line = 0;
    }

    #[inline]
    pub fn set(&mut self, source: usize) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Set {}, source {:?}", self.kind, source);
        }
        self.line.set_bit(source, true);
    }
}
