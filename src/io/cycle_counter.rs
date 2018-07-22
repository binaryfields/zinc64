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
