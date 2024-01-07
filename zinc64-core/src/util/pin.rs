// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

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
    pub fn new_high() -> Self {
        Self {
            state: State::High,
            last: State::High,
        }
    }

    pub fn new_low() -> Self {
        Self {
            state: State::Low,
            last: State::Low,
        }
    }

    pub fn is_falling(&self) -> bool {
        self.last == State::High && self.state == State::Low
    }

    pub fn is_high(&self) -> bool {
        self.state == State::High
    }

    pub fn is_low(&self) -> bool {
        self.state == State::Low
    }

    pub fn is_rising(&self) -> bool {
        self.last == State::Low && self.state == State::High
    }

    pub fn set_active(&mut self, active: bool) {
        if active {
            self.set(State::High);
        } else {
            self.set(State::Low);
        }
    }

    fn set(&mut self, state: State) {
        self.last = self.state;
        self.state = state;
    }
}
