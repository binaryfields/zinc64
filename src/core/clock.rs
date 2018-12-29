// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::Cell;

#[derive(Default)]
pub struct Clock {
    counter: Cell<u64>,
}

impl Clock {
    pub fn elapsed(&self, prev: u64) -> u64 {
        self.counter.get() - prev
    }

    pub fn get(&self) -> u64 {
        self.counter.get()
    }

    pub fn reset(&self) {
        self.counter.set(0);
    }

    pub fn tick(&self) {
        let result = self.counter.get().wrapping_add(1);
        self.counter.set(result);
    }

    pub fn tick_delta(&self, delta: u64) {
        let result = self.counter.get().wrapping_add(delta);
        self.counter.set(result);
    }
}
