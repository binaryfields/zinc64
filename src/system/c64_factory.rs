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

use core::{
    Addressable, Chip, ChipFactory, Clock, Cpu, IoPort, IrqLine, Mmu, Pin, Ram, Rom, SoundOutput,
    SystemModel, VicModel, VideoOutput,
};
use cpu::Cpu6510;
use device::ExpansionPort;
use io::cia;
use io::Cia;
use mem::{Memory, Mmio};
use sound::sid::SamplingMethod;
use sound::Sid;
use video::{Vic, VicMemory};

use super::Config;

pub struct C64Factory {
    config: Rc<Config>,
}

impl C64Factory {
    pub fn new(config: Rc<Config>) -> C64Factory {
        C64Factory { config }
    }
}

impl ChipFactory for C64Factory {
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
    ) -> Rc<RefCell<dyn Chip>> {
        Rc::new(RefCell::new(Cia::new(
            cia::Mode::Cia1,
            cia_flag_pin,
            cia_port_a,
            cia_port_b,
            irq_line,
            Some(joystick_1),
            Some(joystick_2),
            keyboard_matrix,
        )))
    }

    fn new_cia_2(
        &self,
        cia_flag_pin: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Rc<RefCell<dyn Chip>> {
        Rc::new(RefCell::new(Cia::new(
            cia::Mode::Cia2,
            cia_flag_pin,
            cia_port_a,
            cia_port_b,
            irq_line,
            None,
            None,
            keyboard_matrix,
        )))
    }

    fn new_sid(
        &self,
        system_model: &SystemModel,
        clock: Rc<Clock>,
        sound_buffer: Arc<Mutex<dyn SoundOutput>>,
    ) -> Rc<RefCell<dyn Chip>> {
        let mut sid = Sid::new(system_model.sid_model, clock, sound_buffer);
        sid.set_sampling_parameters(
            SamplingMethod::ResampleFast,
            self.config.model.cpu_freq,
            self.config.sound.sample_rate,
        );
        sid.enable_filter(self.config.sound.sid_filters);
        Rc::new(RefCell::new(sid))
    }

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
    ) -> Rc<RefCell<dyn Chip>> {
        let vic_mem = Rc::new(RefCell::new(VicMemory::new(rom_charset, cia_2_port_a, ram)));
        Rc::new(RefCell::new(Vic::new(
            chip_model,
            ba_line,
            color_ram,
            irq_line,
            frame_buffer,
            vic_mem,
        )))
    }

    // -- Memory

    fn new_expansion_port(&self, exp_io_line: Rc<RefCell<IoPort>>) -> Rc<RefCell<dyn Addressable>> {
        Rc::new(RefCell::new(ExpansionPort::new(exp_io_line)))
    }

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
    ) -> Rc<RefCell<dyn Mmu>> {
        let io = Box::new(Mmio::new(
            cia_1,
            cia_2,
            color_ram,
            expansion_port.clone(),
            sid,
            vic,
        ));
        Rc::new(RefCell::new(Memory::new(
            expansion_port.clone(),
            io,
            ram,
            rom_basic,
            rom_charset,
            rom_kernal,
        )))
    }

    fn new_ram(&self, capacity: usize) -> Rc<RefCell<Ram>> {
        Rc::new(RefCell::new(Ram::new(capacity)))
    }

    fn new_rom(&self, path: &Path, offset: u16) -> Result<Rc<RefCell<Rom>>, io::Error> {
        let rom = Rom::load(path, offset)?;
        Ok(Rc::new(RefCell::new(rom)))
    }

    // -- Processor

    fn new_cpu(
        &self,
        ba_line: Rc<RefCell<Pin>>,
        io_port: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        nmi_line: Rc<RefCell<IrqLine>>,
        mem: Rc<RefCell<dyn Mmu>>,
    ) -> Box<dyn Cpu> {
        Box::new(Cpu6510::new(ba_line, io_port, irq_line, nmi_line, mem))
    }
}
