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

mod circular_buffer;
mod clock;
mod cycle_counter;
mod factory;
mod frame_buffer;
pub mod geo;
mod io_port;
mod irq_control;
mod irq_line;
mod system_model;
mod pin;
mod ram;
mod rom;

pub use self::circular_buffer::CircularBuffer;
pub use self::clock::Clock;
pub use self::cycle_counter::CycleCounter;
pub use self::factory::Factory;
pub use self::frame_buffer::FrameBuffer;
pub use self::io_port::IoPort;
pub use self::irq_control::IrqControl;
pub use self::irq_line::IrqLine;
pub use self::system_model::{SidModel, SystemModel, VicModel};
pub use self::pin::Pin;
pub use self::ram::Ram;
pub use self::rom::Rom;

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

pub trait MemoryController {
    fn switch_banks(&mut self, mode: u8);
    // I/O
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
