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

use log::LogLevel;

pub struct InterruptLine {
    kind: Type,
    line: u8,
}

impl InterruptLine {
    pub fn new(kind: Type) -> InterruptLine {
        InterruptLine {
            kind,
            line: 0,
        }
    }

    pub fn clear(&mut self, source: Source) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Clear {:?}, source {:?}", self.kind, source);
        }
        self.line &= !(source as u8);
    }

    pub fn is_low(&self) -> bool {
        self.line != 0
    }

    pub fn reset(&mut self) {
        self.line = 0;
    }

    pub fn set(&mut self, source: Source) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Set {:?}, source {:?}", self.kind, source);
        }
        self.line |= source as u8;
    }
}

#[derive(Debug)]
pub enum Source {
    Cia = 1 << 0,
    Vic = 1 << 1,
}

enum Vector {
    Nmi = 0xfffa,
    Reset = 0xfffc,
    Irq = 0xfffe,
}

#[derive(Debug)]
pub enum Type {
    Break = 1 << 0,
    Irq = 1 << 1,
    Nmi = 1 << 2,
    Reset = 1 << 3,
}

impl Type {
    pub fn vector(&self) -> u16 {
        match *self {
            Type::Break => Vector::Irq as u16,
            Type::Irq => Vector::Irq as u16,
            Type::Nmi => Vector::Nmi as u16,
            Type::Reset => Vector::Reset as u16,
        }
    }
}
