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

pub struct Icr {
    data: u8,
    mask: u8,
}

impl Icr {
    pub fn new() -> Icr {
        Icr { data: 0, mask: 0 }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data = 0;
    }

    #[allow(dead_code)]
    #[inline]
    pub fn clear_events(&mut self, events: u8) {
        self.data &= !events;
    }

    #[inline]
    pub fn get_data(&self) -> u8 {
        let mut data = self.data;
        data.set_bit(7, self.get_interrupt_request());
        data
    }

    #[inline]
    pub fn get_interrupt_request(&self) -> bool {
        (self.mask & self.data) != 0
    }

    #[allow(dead_code)]
    #[inline]
    pub fn get_mask(&self) -> u8 {
        self.mask
    }

    #[inline]
    pub fn set_event(&mut self, bit: u8) {
        self.data.set_bit(bit as usize, true);
    }

    #[inline]
    pub fn reset(&mut self) {
        self.data = 0;
        self.mask = 0;
    }

    #[inline]
    pub fn update_mask(&mut self, mask: u8) {
        /*
        The MASK register provides convenient control of
        individual mask bits. When writing to the MASK register,
        if bit 7 (SET/CLEAR) of the data written is a ZERO,
        any mask bit written with a one will be cleared, while
        those mask bits written with a zero will be unaffected. If
        bit 7 of the data written is a ONE, any mask bit written
        with a one will be set, while those mask bits written with
        a zero will be unaffected. In order for an interrupt flag to
        set IR and generate an Interrupt Request, the corresponding
        MASK bit must be set.
        */
        if mask.get_bit(7) {
            self.mask |= mask & 0x1f;
        } else {
            self.mask &= !(mask & 0x1f);
        }
    }
}
