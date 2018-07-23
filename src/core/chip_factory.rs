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

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use core::{Addressable, Chip, Clock, Cpu, IoPort, IrqLine,
           Mmu, Pin, Ram, Rom, SoundOutput, SystemModel, VicModel, VideoOutput};

pub trait ChipFactory {
    // -- Chipset

    fn new_cia_1(
        &self,
        cia_flag_pin: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        joystick_1: Rc<Cell<u8>>,
        joystick_2: Rc<Cell<u8>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Rc<RefCell<dyn Chip>>;

    fn new_cia_2(
        &self,
        cia_flag_pin: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Rc<RefCell<dyn Chip>>;

    fn new_sid(
        &self,
        system_model: &SystemModel,
        clock: Rc<Clock>,
        sound_buffer: Arc<Mutex<dyn SoundOutput>>,
    ) -> Rc<RefCell<dyn Chip>>;

    fn new_vic(
        &self,
        chip_model: VicModel,
        ba_line: Rc<RefCell<Pin>>,
        cia_2_port_a: Rc<RefCell<IoPort>>,
        color_ram: Rc<RefCell<Ram>>,
        frame_buffer: Rc<RefCell<dyn VideoOutput>>,
        irq_line: Rc<RefCell<IrqLine>>,
        ram: Rc<RefCell<Ram>>,
        rom_charset: Rc<RefCell<Rom>>,
    ) -> Rc<RefCell<dyn Chip>>;

    // -- Memory

    fn new_expansion_port(&self, exp_io_line: Rc<RefCell<IoPort>>) -> Rc<RefCell<dyn Addressable>>;

    fn new_memory(
        &self,
        cia_1: Rc<RefCell<dyn Chip>>,
        cia_2: Rc<RefCell<dyn Chip>>,
        color_ram: Rc<RefCell<Ram>>,
        expansion_port: Rc<RefCell<dyn Addressable>>,
        ram: Rc<RefCell<Ram>>,
        rom_basic: Rc<RefCell<Rom>>,
        rom_charset: Rc<RefCell<Rom>>,
        rom_kernal: Rc<RefCell<Rom>>,
        sid: Rc<RefCell<dyn Chip>>,
        vic: Rc<RefCell<dyn Chip>>,
    ) -> Rc<RefCell<dyn Mmu>>;

    fn new_ram(&self, capacity: usize) -> Rc<RefCell<Ram>>;

    fn new_rom(&self, path: &Path, offset: u16) -> Result<Rc<RefCell<Rom>>, io::Error>;

    // -- Processor

    fn new_cpu(
        &self,
        ba_line: Rc<RefCell<Pin>>,
        io_port: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        nmi_line: Rc<RefCell<IrqLine>>,
        mem: Rc<RefCell<dyn Mmu>>,
    ) -> Box<dyn Cpu>;
}
