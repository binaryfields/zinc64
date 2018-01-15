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

use std::cell::RefCell;
use std::rc::Rc;

use bit_field::BitField;
use core::Pin;

use super::shift_delay::ShiftDelay;

// SPEC: A Software Model of the CIA6526 by Wolfgang Lorenz

#[derive(PartialEq)]
pub enum Mode {
    TimerA,
    TimerB,
}


#[derive(PartialEq)]
pub enum InputMode {
    SystemClock = 0,
    External = 1,
    TimerA = 2,
    TimerAWithCNT = 3,
}

#[derive(PartialEq)]
enum OutputMode {
    Toggle,
    Pulse,
}

#[derive(PartialEq)]
enum RunMode {
    OneShot,
    Continuous,
}

pub struct Timer {
    // Configuration
    mode: Mode,
    enabled: bool,
    input_mode: InputMode,
    output_mode: OutputMode,
    output_enabled: bool,
    run_mode: RunMode,
    // Runtime State
    count_delay: ShiftDelay,
    counter: u16,
    latch: u16,
    load_delay: ShiftDelay,
}

impl Timer {
    pub fn new(mode: Mode) -> Timer {
        Timer {
            mode,
            enabled: false,
            input_mode: InputMode::SystemClock,
            output_mode: OutputMode::Pulse,
            output_enabled: false,
            run_mode: RunMode::OneShot,
            count_delay: ShiftDelay::new(3),
            counter: 0,
            latch: 0,
            load_delay: ShiftDelay::new(1),
        }
    }

    pub fn get_config(&self) -> u8 {
        let mut config = 0;
        config
            .set_bit(0, self.enabled)
            .set_bit(1, self.output_enabled)
            .set_bit(2, self.output_mode == OutputMode::Toggle)
            .set_bit(3, self.run_mode == RunMode::OneShot);
        match self.input_mode {
            InputMode::SystemClock => config.set_bit(5, false),
            InputMode::External => config.set_bit(5, true),
            InputMode::TimerA => {
                config
                    .set_bit(5, false)
                    .set_bit(6, true)
            }
            InputMode::TimerAWithCNT => {
                config
                    .set_bit(5, true)
                    .set_bit(6, true)
            }
        };
        config
    }

    #[allow(dead_code)]
    pub fn get_counter(&self) -> u16 {
        self.counter
    }

    pub fn get_counter_hi(&self) -> u8 {
        (self.counter >> 8) as u8
    }

    pub fn get_counter_lo(&self) -> u8 {
        (self.counter & 0xff) as u8
    }

    #[allow(dead_code)]
    pub fn get_latch(&self) -> u16 {
        self.latch
    }

    pub fn set_config(&mut self, value: u8) {
        self.run_mode = if value.get_bit(3) {
            RunMode::OneShot
        } else {
            RunMode::Continuous
        };
        if value.get_bit(4) {
            self.load_delay.start();
        }
        let input_mode = match self.mode {
            Mode::TimerA => if value.get_bit(5) { 1 } else { 0 },
            Mode::TimerB => (value & 0x60) >> 5,
        };
        self.input_mode = match input_mode {
            0 => InputMode::SystemClock,
            1 => InputMode::External,
            2 => InputMode::TimerA,
            3 => InputMode::TimerAWithCNT,
            _ => panic!("invalid timer input"),
        };
        self.enable(value.get_bit(0));
    }

    pub fn set_latch_hi(&mut self, value: u8) {
        let result = ((value as u16) << 8) | (self.latch & 0x00ff);
        self.latch = result;
        if !self.enabled {
            self.load_delay.start();
        }
    }

    pub fn set_latch_lo(&mut self, value: u8) {
        let result = (self.latch & 0xff00) | (value as u16);
        self.latch = result;
    }

    #[inline]
    pub fn clock(&mut self) -> bool {
        // Decrement counter
        if self.count_delay.is_done() {
            self.counter -= 1;
        }
        // Underflow counter
        let underflow = if self.counter == 0 && self.count_delay.has_cycle(2) {
            /*
            A control bit selects either timer mode. In one-shot
            mode, the timer will count down from the latched value
            to zero, generate an interrupt, reload the latched value,
            then stop. In continuous mode, the timer will count from
            the latched value to zero, generate an interrupt, reload
            the latched value and repeatthe procedure continuously
            */
            match self.run_mode {
                RunMode::Continuous => {
                    self.load_delay.feed(1);
                }
                RunMode::OneShot => {
                    self.enable(false);
                    self.load_delay.feed(1);
                }
            }
            true
        } else {
            false
        };
        // Load counter
        if self.load_delay.is_done() {
            self.counter = self.latch;
            /*
            Whenever the counter is reloaded from the latch, either by
            underflow or by setting the force load bit of the CRA to 1,
            the next clock will be removed from the pipeline.
            */
            self.count_delay.remove(2);
        }
        // Shift delay counters
        self.count_delay.clock();
        self.load_delay.clock();
        underflow
    }

    #[inline]
    pub fn feed_source(&mut self, cnt: &Rc<RefCell<Pin>>, timer_a_output: bool) {
        match self.input_mode {
            InputMode::SystemClock => {
                // Already fed through by Count0 in self.feed
            }
            InputMode::External => {
                if cnt.borrow().is_rising() {
                    self.count_delay.feed(0);
                }
            }
            InputMode::TimerA => {
                if timer_a_output {
                    self.count_delay.feed(1);
                }
            }
            InputMode::TimerAWithCNT => {
                if timer_a_output && cnt.borrow().is_rising() {
                    self.count_delay.feed(0);
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.enabled = false;
        self.input_mode = InputMode::SystemClock;
        self.output_mode = OutputMode::Pulse;
        self.output_enabled = false;
        self.run_mode = RunMode::OneShot;
        self.count_delay.reset();
        self.counter = 0;
        self.latch = 0xffff;
        self.load_delay.reset();
    }

    fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled && self.input_mode == InputMode::SystemClock {
            self.count_delay.feed(0);
            self.count_delay.feed(1);
            self.count_delay.set_feed(0, true);
        } else {
            self.count_delay.remove(0);
            self.count_delay.remove(1);
            self.count_delay.set_feed(0, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_config() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0b_0010_1001);
        assert_eq!(true, timer.enabled);
        assert_eq!(RunMode::OneShot, timer.run_mode);
        assert_eq!(InputMode::External, timer.input_mode);
    }

    #[test]
    fn set_and_get_config() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0b_0010_1001);
        assert_eq!(0b_0010_1001, timer.get_config());
    }
}