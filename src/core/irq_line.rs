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
use log::LogLevel;

pub struct IrqLine {
    kind: &'static str,
    signal: u8,
}

impl IrqLine {
    pub fn new(kind: &'static str) -> Self {
        Self { kind, signal: 0 }
    }

    pub fn is_low(&self) -> bool {
        self.signal != 0
    }

    pub fn reset(&mut self) {
        self.signal = 0;
    }

    pub fn set_low(&mut self, source: usize, value: bool) {
        if log_enabled!(LogLevel::Trace) {
            trace!(
                target: "cpu::int", "{}.{:?} {}",
                self.kind,
                source,
                if value { "set " } else { "cleared " }
            );
        }
        self.signal.set_bit(source, value);
    }
}
