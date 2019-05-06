// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

mod expansion_port;
mod memory;
mod mmio;
mod pla;

pub use self::expansion_port::ExpansionPort;
pub use self::memory::Memory;
pub use self::mmio::Mmio;
pub use self::pla::Pla;

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum BaseAddr {
    Basic = 0xa000,
    Charset = 0xd000,
    Kernal = 0xe000,
}

impl BaseAddr {
    pub fn addr(self) -> u16 {
        self as u16
    }
}
