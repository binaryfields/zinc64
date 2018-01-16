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

use bit_field::BitField;

#[derive(Debug)]
pub struct ShiftDelay {
    // Configuration
    delay: usize,
    mask: u16,
    // State
    counter: u16,
    feed: u16,
}

impl ShiftDelay {
    pub fn new(delay: usize) -> ShiftDelay {
        let mut mask = 0;
        for i in 0..(delay + 1) {
            mask.set_bit(i, true);
        }
        ShiftDelay {
            delay,
            mask,
            counter: 0,
            feed: 0,
        }
    }

    #[inline]
    pub fn has_cycle(&self, cycle: usize) -> bool {
        self.counter.get_bit(cycle)
    }

    #[inline]
    pub fn is_done(&self) -> bool {
        self.counter.get_bit(self.delay)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn set_feed(&mut self, cycle: usize, value: bool) {
        self.feed.set_bit(cycle, value);
    }

    #[inline]
    pub fn clock(&mut self) {
        self.counter = ((self.counter << 1) & self.mask) | self.feed;
    }

    #[inline]
    pub fn feed(&mut self, cycle: usize) {
        self.counter.set_bit(cycle, true);
    }

    #[inline]
    pub fn remove(&mut self, cycle: usize) {
        self.counter.set_bit(cycle, false);
    }

    #[inline]
    pub fn reset(&mut self) {
        self.counter = 0;
        self.feed = 0;
    }

    #[inline]
    pub fn start(&mut self) {
        self.feed(0);
    }
}