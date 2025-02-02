// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use bit_field::BitField;
// use log::LogLevel;

pub struct IrqLine {
    #[allow(unused)]
    kind: &'static str,
    signal: u8,
}

impl IrqLine {
    pub fn new(kind: &'static str) -> Self {
        Self { kind, signal: 0 }
    }

    pub fn is_low(&self) -> bool {
        self.signal != 0
    }

    pub fn reset(&mut self) {
        self.signal = 0;
    }

    pub fn set_low(&mut self, source: usize, value: bool) {
        /* if log_enabled!(LogLevel::Trace) {
            trace!(
                target: "cpu::int", "{}.{:?} {}",
                self.kind,
                source,
                if value { "set " } else { "cleared " }
            );
        } */
        self.signal.set_bit(source, value);
    }
}
