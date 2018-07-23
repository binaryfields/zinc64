// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;

pub struct IrqControl {
    data: u8,
    mask: u8,
}

impl IrqControl {
    pub fn new() -> Self {
        Self { data: 0, mask: 0 }
    }

    pub fn clear(&mut self) {
        self.data = 0;
    }

    pub fn clear_events(&mut self, events: u8) {
        self.data = self.data & (!events);
    }

    pub fn get_data(&self) -> u8 {
        let mut data = self.data;
        data.set_bit(7, self.is_triggered());
        data
    }

    pub fn get_mask(&self) -> u8 {
        self.mask
    }

    #[allow(dead_code)]
    pub fn get_raw_data(&self) -> u8 {
        self.data
    }

    pub fn is_triggered(&self) -> bool {
        (self.mask & self.data) != 0
    }

    pub fn reset(&mut self) {
        self.data = 0;
        self.mask = 0;
    }

    pub fn set_event(&mut self, bit: usize) {
        self.data.set_bit(bit, true);
    }

    pub fn set_mask(&mut self, mask: u8) {
        self.mask = mask;
    }

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
