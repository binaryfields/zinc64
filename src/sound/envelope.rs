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

use util::bit;

#[derive(Clone, Copy)]
pub enum State {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy)]
pub struct Envelope {
    pub attack: u8,
    pub decay: u8,
    pub sustain: u8,
    pub release: u8,
    // Control
    pub gate: bool,
    // Runtime State
    pub state: State,
    pub counter: u16,
}

impl Envelope {
    pub fn new() -> Envelope {
        Envelope {
            attack: 0,
            decay: 0,
            sustain: 0,
            release: 0,
            gate: false,
            state: State::Release,
            counter: 0,
        }
    }

    pub fn set_control(&mut self, value: u8) {
        self.gate = bit::bit_test(value, 0);
    }

    pub fn reset(&mut self) {
        self.attack = 0;
        self.decay = 0;
        self.sustain = 0;
        self.release = 0;
        self.gate = false;
        self.state = State::Release;
        self.counter = 0;
    }
}
