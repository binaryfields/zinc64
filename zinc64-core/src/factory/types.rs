// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
#[cfg(feature = "std")]
use std::rc::Rc;

/// A tick represents a callback invoked by the cpu for each clock cycle
/// during instruction execution.
pub type TickFn = Rc<dyn Fn()>;

pub fn make_noop() -> TickFn {
    Rc::new(|| {})
}

/// Addressable represents a bank of memory.
pub trait Addressable {
    /// Read byte from the specified address.
    fn read(&self, address: u16) -> u8;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

/// Addressable represents a bank of memory that may be faded by RAM.
pub trait AddressableFaded {
    /// Read byte from the specified address.
    fn read(&mut self, address: u16) -> Option<u8>;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
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

#[derive(Copy, Clone)]
pub enum Register {
    A,
    X,
    Y,
    SP,
    PCL,
    PCH,
    P
}

/// CPU is responsible for decoding and executing instructions.
pub trait Cpu {
    // -- Getters/Setters
    fn get_register(&self, reg: Register) -> u8;
    fn set_register(&mut self, reg: Register, value: u8);
    fn get_pc(&self) -> u16;
    fn set_pc(&mut self, value: u16);
    fn is_cpu_jam(&self) -> bool;
    /// The core method of the cpu, decodes and executes one instruction. Tick callback is invoked
    /// for each elapsed clock cycle.
    fn step(&mut self, tick_fn: &TickFn);
    /// Reset chip.
    fn reset(&mut self);
    // I/O
    /// Read byte from the specified address.
    fn read(&self, address: u16) -> u8;
    /// Write byte to the specified address.
    fn write(&mut self, address: u16, value: u8);
}

/// Represents memory management unit which controls visible memory banks.
pub trait Mmu {
    /// Map address to currently mapped in memory bank.
    fn map(&self, address: u16) -> Bank;
    /// Change bank configuration based on the specified mode.
    fn switch_banks(&mut self, mode: u8);
}

/// Sound output used by SID chip.
pub trait SoundOutput {
    /// Reset output.
    fn reset(&self);
    /// Write generated sample to the output buffer.
    fn write(&self, samples: &[i16]);
}

/// Video output used by VIC chip.
pub trait VideoOutput {
    /// Get frame buffer width and height.
    fn get_dimension(&self) -> (usize, usize);
    /// Reset output.
    fn reset(&mut self);
    /// Write pixel color to the specified location. Index is computed from raster x, y coordinates:
    /// index = y * pitch + x.
    fn write(&mut self, index: usize, color: u8);
}

pub trait Tape {
    fn read_pulse(&mut self) -> Option<u32>;
    fn seek(&mut self, pos: usize) -> bool;
}
