// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

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

pub trait Addressable {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub trait Chip {
    fn clock(&mut self);
    fn clock_delta(&mut self, delta: u32);
    fn process_vsync(&mut self);
    fn reset(&mut self);
    // I/O
    fn read(&mut self, reg: u8) -> u8;
    fn write(&mut self, reg: u8, value: u8);
}

pub type TickFn = Box<Fn()>;

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
    fn step(&mut self, tick_fn: &TickFn);
    // I/O
    fn read_debug(&self, address: u16) -> u8;
    fn write_debug(&mut self, address: u16, value: u8);
}

pub trait Mmu {
    fn switch_banks(&mut self, mode: u8);
    // I/O
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub trait SoundOutput {
    fn write(&mut self, value: i16);
}

pub trait VideoOutput {
    fn set_sync(&mut self, value: bool);
    fn write(&mut self, x: u16, y: u16, color: u8);
}
