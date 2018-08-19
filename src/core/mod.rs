// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::rc::Rc;

mod chip_factory;
mod clock;
mod io_port;
mod irq_control;
mod irq_line;
mod pin;
mod ram;
mod rom;
mod system_model;

pub use self::chip_factory::ChipFactory;
pub use self::clock::Clock;
pub use self::io_port::IoPort;
pub use self::irq_control::IrqControl;
pub use self::irq_line::IrqLine;
pub use self::pin::Pin;
pub use self::ram::Ram;
pub use self::rom::Rom;
pub use self::system_model::{SidModel, SystemModel, VicModel};

/// Addressable represents a bank of memory.
pub trait Addressable {
    /// Read byte from the specified address.
    fn read(&self, address: u16) -> u8;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

/// A chip represents a system component that is driven by clock signal.
pub trait Chip {
    /// The core method of the chip, emulates one clock cycle of the chip.
    fn clock(&mut self);
    /// Process delta cycles at once.
    fn clock_delta(&mut self, delta: u32);
    /// Handle vsync event.
    fn process_vsync(&mut self);
    /// Handle reset signal.
    fn reset(&mut self);
    // I/O
    /// Read value from the specified register.
    fn read(&mut self, reg: u8) -> u8;
    /// Write value to the specified register.
    fn write(&mut self, reg: u8, value: u8);
}

/// A tick represents a callback invoked by the cpu for each clock cycle
/// during instruction execution.
pub type TickFn = Rc<Fn()>;

/// CPU is responsible for decoding and executing instructions.
pub trait Cpu {
    fn get_a(&self) -> u8;
    fn get_p(&self) -> u8;
    fn get_pc(&self) -> u16;
    fn get_sp(&self) -> u8;
    fn get_x(&self) -> u8;
    fn get_y(&self) -> u8;
    fn set_a(&mut self, value: u8);
    fn set_p(&mut self, value: u8);
    fn set_pc(&mut self, value: u16);
    fn set_sp(&mut self, value: u8);
    fn set_x(&mut self, value: u8);
    fn set_y(&mut self, value: u8);
    fn reset(&mut self);
    /// The core method of the cpu, decodes and executes one instruction. Tick callback is invoked
    /// for each elapsed clock cycle.
    fn step(&mut self, tick_fn: &TickFn);
    // I/O
    /// Read byte from the specified address.
    fn read(&self, address: u16) -> u8;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

/// Represents memory management unit which controls visible memory banks
/// and is used by CPU to read from and write to memory locations.
pub trait Mmu {
    /// Change bank configuration based on the specified mode.
    fn switch_banks(&mut self, mode: u8);
    // I/O
    /// Read byte from the specified address.
    fn read(&self, address: u16) -> u8;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

/// Sound output used by SID chip.
pub trait SoundOutput {
    /// Write generated sample to the output buffer.
    fn write(&mut self, value: i16);
}

/// Video output used by VIC chip.
pub trait VideoOutput {
    /// Get frame buffer width and height..
    fn get_dimension(&self) -> (usize, usize);
    fn set_sync(&mut self, value: bool);
    /// Write pixel color to the specified location. Index is computed from raster x, y coordinates:
    /// index = y * pitch + x.
    fn write(&mut self, index: usize, color: u8);
}
