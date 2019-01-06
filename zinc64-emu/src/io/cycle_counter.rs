// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub struct CycleCounter {
    // Configuration
    mask: u16,
    // State
    cycles: u16,
    feed: u16,
}

impl CycleCounter {
    pub fn new(mask: u16) -> Self {
        Self {
            mask,
            cycles: 0,
            feed: 0,
        }
    }

    #[inline(always)]
    pub fn has_cycle(&self, mask: u16) -> bool {
        self.cycles & mask != 0
    }

    #[inline(always)]
    pub fn autofeed(&mut self, mask: u16, enabled: bool) {
        if enabled {
            self.feed |= mask;
        } else {
            self.feed &= !mask;
        }
    }

    #[inline(always)]
    pub fn clock(&mut self) {
        self.cycles = ((self.cycles << 1) & self.mask) | self.feed;
    }

    #[inline(always)]
    pub fn feed(&mut self, mask: u16) {
        self.cycles |= mask;
    }

    #[inline(always)]
    pub fn remove(&mut self, mask: u16) {
        self.cycles &= !mask;
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.cycles = 0;
        self.feed = 0;
    }
}
