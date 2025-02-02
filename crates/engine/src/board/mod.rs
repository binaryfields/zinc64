// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub use self::expansion_port::ExpansionPort;
pub use self::memory::Memory;
pub use self::mmio::Mmio;
pub use self::pla::Pla;

mod expansion_port;
mod memory;
mod mmio;
mod pla;

/// Represents memory management unit which controls visible memory banks.
pub trait Mmu {
    /// Map address to currently mapped in memory bank.
    fn map(&self, address: u16) -> Bank;
    /// Change bank configuration based on the specified mode.
    fn switch_banks(&mut self, mode: u8);
}

/// Memory bank type used with MMU to determine how to map a memory address
#[derive(Clone, Copy)]
pub enum Bank {
    Basic,
    Charset,
    Kernal,
    Io,
    Ram,
    RomH,
    RomL,
    Disabled,
}

/// Addressable represents a bank of memory that may be faded by RAM.
pub trait AddressableFaded {
    /// Read byte from the specified address.
    fn read(&mut self, address: u16) -> Option<u8>;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

#[derive(Copy, Clone)]
pub(crate) enum BaseAddr {
    Basic = 0xa000,
    BootComplete = 0xa65c,
    Charset = 0xd000,
    Kernal = 0xe000,
}

impl BaseAddr {
    pub fn addr(self) -> u16 {
        self as u16
    }
}
