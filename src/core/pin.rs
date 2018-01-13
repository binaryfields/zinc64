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

#[derive(Clone, Copy, PartialEq)]
enum State {
    High,
    Low,
}

pub struct Pin {
    state: State,
    last: State,
}

impl Pin {
    pub fn new_high() -> Pin {
        Pin {
            state: State::High,
            last: State::High,
        }
    }

    pub fn new_low() -> Pin {
        Pin {
            state: State::Low,
            last: State::Low,
        }
    }

    #[inline]
    pub fn is_falling(&self) -> bool {
        self.last == State::High && self.state == State::Low
    }

    #[inline]
    pub fn is_high(&self) -> bool {
        self.state == State::High
    }

    #[inline]
    pub fn is_low(&self) -> bool {
        self.state == State::Low
    }

    #[inline]
    pub fn is_rising(&self) -> bool {
        self.last == State::Low && self.state == State::High
    }

    #[inline]
    pub fn set_active(&mut self, active: bool) {
        if active {
            self.set(State::High);
        } else {
            self.set(State::Low);
        }
    }

    #[inline]
    fn set(&mut self, state: State) {
        self.last = self.state;
        self.state = state;
    }
}
