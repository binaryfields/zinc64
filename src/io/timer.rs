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

#[derive(Debug, PartialEq)]
pub enum Mode {
    OneShot,
    Continuous,
}

#[derive(Debug, PartialEq)]
pub enum Input {
    SystemClock = 0,
    External = 1,
    TimerA = 2,
    TimerAWithCNT = 3,
}

#[derive(Debug, PartialEq)]
pub enum Output {
    Toggle,
    Pulse,
}

#[derive(Debug)]
pub struct Timer {
    pub enabled: bool,
    pub mode: Mode,
    pub input: Input,
    pub output: Output,
    pub output_enabled: bool,
    pub latch: u16,
    pub value: u16,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            enabled: false,
            mode: Mode::OneShot,
            input: Input::SystemClock,
            output: Output::Pulse,
            output_enabled: false,
            latch: 0,
            value: 0,
        }
    }

    pub fn reset(&mut self) {
        self.enabled = false;
        self.mode = Mode::OneShot;
        self.input = Input::SystemClock;
        self.output = Output::Pulse;
        self.output_enabled = false;
        self.latch = 0xffff;
        self.value = 0x0000;
    }

    pub fn update(&mut self, pulse: u16) -> bool {
        if self.enabled {
            if self.value == 0 {
                self.reload();
                true
            } else {
                self.value -= pulse;
                false
            }
        } else {
            false
        }
    }

    fn reload(&mut self) {
        /*
        A control bit selects either timer mode. In one-shot
        mode, the timer will count down from the latched value
        to zero, generate an interrupt, reload the latched value,
        then stop. In continuous mode, the timer will count from
        the latched value to zero, generate an interrupt, reload
        the latched value and repeatthe procedure continuously
        */
        match self.mode {
            Mode::Continuous => {
                self.value = self.latch;
            }
            Mode::OneShot => {
                self.value = self.latch;
                self.enabled = false;
            }
        }
    }
}
