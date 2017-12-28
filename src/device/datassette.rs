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

use std::cell::RefCell;
use std::rc::Rc;

use bit_field::BitField;

use cpu::CpuIo;
use io::CiaIo;
use util::Pulse;

use super::Tape;

// TODO device: datassette test cases

const DUTY_CYCLE: u32 = 50;

pub struct Datassette {
    // Dependencies
    cia_io: Rc<RefCell<CiaIo>>,
    cpu_io: Rc<RefCell<CpuIo>>,
    // Runtime State
    playing: bool,
    tape: Option<Box<Tape>>,
    current_pulse: Pulse,
}

impl Datassette {
    pub fn new(cia_io: Rc<RefCell<CiaIo>>, cpu_io: Rc<RefCell<CpuIo>>) -> Datassette {
        Datassette {
            cia_io,
            cpu_io,
            playing: false,
            tape: None,
            current_pulse: Pulse::new(0, DUTY_CYCLE),
        }
    }

    pub fn attach(&mut self, tape: Box<Tape>) {
        self.tape = Some(tape);
    }

    #[inline(always)]
    pub fn clock(&mut self) {
        if self.is_playing() && self.tape.is_some() {
            if self.current_pulse.is_done() {
                let pulse_maybe = if let Some(ref mut tape) = self.tape {
                    tape.read_pulse()
                } else {
                    None
                };
                if let Some(pulse) = pulse_maybe {
                    self.current_pulse = Pulse::new(pulse, DUTY_CYCLE);
                } else {
                    self.stop();
                }
            }
            if !self.current_pulse.is_done() {
                self.cia_io
                    .borrow_mut()
                    .flag
                    .set_active(self.current_pulse.advance());
            }
        }
    }

    pub fn detach(&mut self) {
        self.stop();
        self.tape = None;
    }

    pub fn is_playing(&self) -> bool {
        self.playing & !self.cpu_io.borrow().port_1.get_value().get_bit(5)
    }

    pub fn play(&mut self) {
        info!(target: "device", "Starting datassette");
        if self.tape.is_some() {
            self.cpu_io.borrow_mut().cassette_switch = false;
            self.playing = true;
        }
    }

    pub fn reset(&mut self) {
        self.cpu_io.borrow_mut().cassette_switch = true;
        self.playing = false;
        self.current_pulse = Pulse::new(0, DUTY_CYCLE);
        if let Some(ref mut tape) = self.tape {
            tape.seek(0);
        }
    }

    pub fn stop(&mut self) {
        info!(target: "device", "Stopping datassette");
        self.cpu_io.borrow_mut().cassette_switch = true;
        self.playing = false;
    }
}
