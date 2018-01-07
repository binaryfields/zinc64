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

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use zinc64::core::{
    Addressable,
    Chip,
    Cpu,
    Factory,
    FrameBuffer,
    IrqLine,
    IoLine,
    IoPort,
    MemoryController,
    SystemModel,
    Pin,
    Ram,
    Rom,
    SoundBuffer,
    VicModel,
};
use zinc64::cpu::Cpu6510;
use zinc64::device::ExpansionPort;
use zinc64::mem::{Mmio, Memory};
use zinc64::io::Cia;
use zinc64::io::cia;
use zinc64::sound::Sid;
use zinc64::sound::sid;
use zinc64::video::{Vic, VicMemory};

pub struct ChipFactory;

impl Factory for ChipFactory {

    // -- Chipset

    fn new_cia1(
        &self,
        cia_flag: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        cpu_irq: Rc<RefCell<IrqLine>>,
        cpu_nmi: Rc<RefCell<IrqLine>>,
        joystick_1: Rc<Cell<u8>>,
        joystick_2: Rc<Cell<u8>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Rc<RefCell<Chip>> {
        Rc::new(RefCell::new(
            Cia::new(
                cia::Mode::Cia1,
                cia_flag,
                cia_port_a,
                cia_port_b,
                cpu_irq,
                cpu_nmi,
                Some(joystick_1),
                Some(joystick_2),
                keyboard_matrix,
            )
        ))
    }

    fn new_cia2(
        &self,
        cia_flag: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        cpu_irq: Rc<RefCell<IrqLine>>,
        cpu_nmi: Rc<RefCell<IrqLine>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Rc<RefCell<Chip>> {
        Rc::new(RefCell::new(
            Cia::new(
                cia::Mode::Cia2,
                cia_flag,
                cia_port_a,
                cia_port_b,
                cpu_irq,
                cpu_nmi,
                None,
                None,
                keyboard_matrix,
            )
        ))
    }

    fn new_sid(
        &self,
        system_model: &SystemModel,
        sound_buffer: Arc<Mutex<SoundBuffer>>,
    ) -> Rc<RefCell<Chip>> {
        let mut sid = Sid::new(system_model.sid_model, sound_buffer);
        sid.set_sampling_parameters(
            sid::SamplingMethod::ResampleFast,
            system_model.cpu_freq,
            44100,
        );
        Rc::new(RefCell::new(sid))
    }

    fn new_vic(
        &self,
        chip_model: VicModel,
        charset: Rc<RefCell<Rom>>,
        cia_2_port_a: Rc<RefCell<IoPort>>,
        color_ram: Rc<RefCell<Ram>>,
        cpu_irq: Rc<RefCell<IrqLine>>,
        frame_buffer: Rc<RefCell<FrameBuffer>>,
        ram: Rc<RefCell<Ram>>,
    ) -> Rc<RefCell<Chip>> {
        let vic_mem = Rc::new(RefCell::new(
            VicMemory::new(
                charset,
                cia_2_port_a,
                ram,
            )
        ));
        Rc::new(RefCell::new(
            Vic::new(
                chip_model,
                color_ram,
                cpu_irq,
                frame_buffer,
                vic_mem,
            )
        ))
    }

    // -- Memory

    fn new_expansion_port(
        &self,
        exp_io_line: Rc<RefCell<IoLine>>,
    ) -> Rc<RefCell<Addressable>> {
        Rc::new(RefCell::new(
            ExpansionPort::new(exp_io_line)
        ))
    }

    fn new_memory(
        &self,
        cia1: Rc<RefCell<Chip>>,
        cia2: Rc<RefCell<Chip>>,
        color_ram: Rc<RefCell<Ram>>,
        expansion_port: Rc<RefCell<Addressable>>,
        ram: Rc<RefCell<Ram>>,
        rom_basic: Rc<RefCell<Rom>>,
        rom_charset: Rc<RefCell<Rom>>,
        rom_kernal: Rc<RefCell<Rom>>,
        sid: Rc<RefCell<Chip>>,
        vic: Rc<RefCell<Chip>>,
    ) -> Rc<RefCell<MemoryController>> {
        let io = Box::new(
            Mmio::new(
                cia1,
                cia2,
                color_ram,
                expansion_port.clone(),
                sid,
                vic,
            )
        );
        Rc::new(RefCell::new(
            Memory::new(
                expansion_port.clone(),
                io,
                ram,
                rom_basic,
                rom_charset,
                rom_kernal,
            )
        ))
    }

    fn new_ram(
        &self,
        capacity: usize,
    ) -> Rc<RefCell<Ram>> {
        Rc::new(RefCell::new(
            Ram::new(capacity)
        ))
    }

    fn new_rom(&self,
               path: &Path,
               offset: u16,
    ) -> Result<Rc<RefCell<Rom>>, io::Error> {
        let rom = Rom::load(path, offset)?;
        Ok(Rc::new(RefCell::new(rom)))
    }

    // -- Processor

    fn new_cpu(
        &self,
        io_port: Rc<RefCell<IoPort>>,
        irq: Rc<RefCell<IrqLine>>,
        nmi: Rc<RefCell<IrqLine>>,
        mem: Rc<RefCell<MemoryController>>,
    ) -> Rc<RefCell<Cpu>> {
        Rc::new(RefCell::new(
            Cpu6510::new(
                io_port,
                irq,
                nmi,
                mem,
            )
        ))
    }
}
