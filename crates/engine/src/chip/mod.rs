// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub use self::cpu::{Bus, Cpu6510, Register};
pub use self::ram::Ram;
pub use self::rom::Rom;

pub mod cia;
pub mod cpu;
mod ram;
mod rom;
pub mod sid;
pub mod vic;

/// Sound output used by SID chip.
pub trait SoundOutput {
    /// Reset output.
    fn reset(&self);
    /// Write generated sample to the output buffer.
    fn write(&self, samples: &[i16]);
}
