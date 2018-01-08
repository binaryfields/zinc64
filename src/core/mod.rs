/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

mod factory;
mod frame_buffer;
pub mod geo;
mod ioport;
mod irqline;
mod system_model;
mod pin;
mod ram;
mod rom;
mod sound_buffer;

pub use self::factory::Factory;
pub use self::frame_buffer::FrameBuffer;
pub use self::ioport::IoPort;
pub use self::irqline::IrqLine;
pub use self::system_model::{SystemModel, SidModel, VicModel};
pub use self::pin::Pin;
pub use self::ram::Ram;
pub use self::rom::Rom;
pub use self::sound_buffer::SoundBuffer;

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
    fn get_pc(&self) -> u16;
    fn set_pc(&mut self, value: u16);
    fn reset(&mut self);
    fn step(&mut self, tick_fn: &TickFn);
    fn read_debug(&self, address: u16) -> u8;
    fn write_debug(&mut self, address: u16, value: u8);
}

pub trait MemoryController {
    fn switch_banks(&mut self, mode: u8);
    // I/O
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
