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

#[derive(Debug, PartialEq)]
pub enum Mode {
    TimerA,
    TimerB,
}

#[derive(Debug, PartialEq)]
enum InputMode {
    SystemClock = 0,
    External = 1,
    TimerA = 2,
    TimerAWithCNT = 3,
}

#[derive(Debug, PartialEq)]
enum OutputMode {
    Pulse,
    Toggle,
}

#[derive(Debug, PartialEq)]
enum RunMode {
    Continuous,
    OneShot,
}

#[derive(Debug)]
pub struct Timer {
    // Configuration
    mode: Mode,
    enabled: bool,
    input_mode: InputMode,
    output_mode: OutputMode,
    pb_on: bool,
    run_mode: RunMode,
    // Runtime State
    count_delay: ShiftDelay,
    counter: u16,
    latch: u16,
    load_delay: ShiftDelay,
    pb_low_delay: ShiftDelay,
    pb_output: bool,
    pb_toggle: bool,
}

impl Timer {
    pub fn new(mode: Mode) -> Timer {
        Timer {
            mode,
            enabled: false,
            input_mode: InputMode::SystemClock,
            output_mode: OutputMode::Pulse,
            pb_on: false,
            run_mode: RunMode::OneShot,
            count_delay: ShiftDelay::new(3),
            counter: 0,
            latch: 0,
            load_delay: ShiftDelay::new(1),
            pb_low_delay: ShiftDelay::new(1),
            pb_output: false,
            pb_toggle: false,
        }
    }

    pub fn get_config(&self) -> u8 {
        let mut config = 0;
        config
            .set_bit(0, self.enabled)
            .set_bit(1, self.pb_on)
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

    pub fn is_pb_on(&self) -> bool {
        self.pb_on
    }

    pub fn get_pb_output(&self) -> bool {
        self.pb_output
    }

    pub fn set_config(&mut self, value: u8) {
        let prev_enabled = self.enabled;
        self.pb_on = value.get_bit(1);
        self.output_mode = if value.get_bit(2) {
            OutputMode::Toggle
        } else {
            OutputMode::Pulse
        };
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
        // Update PB output
        if self.enabled && !prev_enabled {
            self.pb_toggle = true;
        }
        if self.pb_on {
            self.pb_output = match self.output_mode {
                OutputMode::Pulse => self.pb_low_delay.is_done(),
                OutputMode::Toggle => self.pb_toggle,
            };
        }
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
            // Update PB output
            self.pb_toggle = !self.pb_toggle;
            if self.pb_on {
                self.pb_output = match self.output_mode {
                    OutputMode::Toggle => !self.pb_output,
                    OutputMode::Pulse => {
                        self.pb_low_delay.start();
                        true
                    }
                };
            }
            /*
            A control bit selects either timer mode. In one-shot
            mode, the timer will count down from the latched value
            to zero, generate an interrupt, reload the latched value,
            then stop. In continuous mode, the timer will count from
            the latched value to zero, generate an interrupt, reload
            the latched value and repeatthe procedure continuously
            */
            self.load_delay.feed(1);
            if self.run_mode == RunMode::OneShot {
                self.enable(false);
                self.count_delay.remove(2);
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
        // Reset PB output
        if self.pb_low_delay.is_done() {
            self.pb_output = false;
        }
        // Shift delay counters
        self.count_delay.clock();
        self.load_delay.clock();
        self.pb_low_delay.clock();
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
        self.pb_on = false;
        self.run_mode = RunMode::OneShot;
        self.count_delay.reset();
        self.counter = 0;
        self.latch = 0xffff;
        self.load_delay.reset();
        self.pb_low_delay.reset();
        self.pb_output = false;
        self.pb_toggle = false;
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

    #[test]
    fn count_delay_2c() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock();
        timer.clock();
        timer.set_config(0x01);
        timer.clock(); // Count0|Count1
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
    }

    #[test]
    fn load_delay_1c() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock(); // Load0
        assert_eq!(timer.get_counter(), 0x00);
        timer.clock(); // Load1
        assert_eq!(timer.get_counter(), 0x02);
   }

    #[test]
    fn pb_output_pulse() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock();
        timer.clock();
        timer.set_config(0x03);
        timer.clock(); // Count0|Count1
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
        assert_eq!(timer.get_pb_output(), false);
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
        assert_eq!(timer.get_pb_output(), false);
        timer.clock(); // Count3|Underflow|Load1
        assert_eq!(timer.get_counter(), 0x02);
        assert_eq!(timer.get_pb_output(), true);
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
        assert_eq!(timer.get_pb_output(), false);
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
        assert_eq!(timer.get_pb_output(), false);
        timer.clock(); // Count3|Underflow|Load1
        assert_eq!(timer.get_counter(), 0x02);
        assert_eq!(timer.get_pb_output(), true);
    }

    #[test]
    fn reload_delay_0c() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock();
        timer.clock();
        timer.set_config(0x01);
        timer.clock(); // Count0|Count1
        timer.clock(); // Count2
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
        let output = timer.clock(); // Count3|Underflow|Load1
        assert_eq!(output, true);
        assert_eq!(timer.get_counter(), 0x02);
    }

    #[test]
    fn reload_count_delay_1c() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock();
        timer.clock();
        timer.set_config(0x01);
        timer.clock(); // Count0|Count1
        timer.clock(); // Count2
        timer.clock(); // Count3
        timer.clock(); // Count3|Underflow|Load1
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
    }

    #[test]
    fn reload_scenario() {
        let mut timer = Timer::new(Mode::TimerA);
        timer.set_config(0x00);
        timer.set_latch_lo(0x02);
        timer.set_latch_hi(0x00);
        timer.clock();
        timer.clock();
        timer.set_config(0x01);
        timer.clock(); // Count0|Count1
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
        timer.clock(); // Count3|Underflow|Load1
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count2
        assert_eq!(timer.get_counter(), 0x02);
        timer.clock(); // Count3
        assert_eq!(timer.get_counter(), 0x01);
        timer.clock(); // Count3|Underflow|Load1
        assert_eq!(timer.get_counter(), 0x02);
    }
}