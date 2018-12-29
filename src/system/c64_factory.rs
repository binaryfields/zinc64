// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::core::{
    Addressable, Chip, ChipFactory, Clock, Cpu, IoPort, IrqLine, Mmu, Pin, Ram, Rom, SidModel,
    SoundOutput, VicModel, VideoOutput,
};
use crate::cpu::Cpu6510;
use crate::device::ExpansionPort;
use crate::io::cia;
use crate::io::Cia;
use crate::mem::{Memory, Mmio};
use crate::sound::sid::SamplingMethod;
use crate::sound::Sid;
use crate::video::{Vic, VicMemory};

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
    fn new_cpu(
        &self,
        mem: Rc<RefCell<dyn Mmu>>,
        io_port: Rc<RefCell<IoPort>>,
        ba_line: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
        nmi_line: Rc<RefCell<IrqLine>>,
    ) -> Box<dyn Cpu> {
        Box::new(Cpu6510::new(mem, io_port, ba_line, irq_line, nmi_line))
    }

    // -- Chipset

    fn new_cia_1(
        &self,
        joystick_1: Rc<Cell<u8>>,
        joystick_2: Rc<Cell<u8>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
        port_a: Rc<RefCell<IoPort>>,
        port_b: Rc<RefCell<IoPort>>,
        flag_pin: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>> {
        Rc::new(RefCell::new(Cia::new(
            cia::Mode::Cia1,
            Some(joystick_1),
            Some(joystick_2),
            Some(keyboard_matrix),
            port_a,
            port_b,
            flag_pin,
            irq_line,
        )))
    }

    fn new_cia_2(
        &self,
        port_a: Rc<RefCell<IoPort>>,
        port_b: Rc<RefCell<IoPort>>,
        flag_pin: Rc<RefCell<Pin>>,
        nmi_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>> {
        Rc::new(RefCell::new(Cia::new(
            cia::Mode::Cia2,
            None,
            None,
            None,
            port_a,
            port_b,
            flag_pin,
            nmi_line,
        )))
    }

    fn new_sid(
        &self,
        chip_model: SidModel,
        system_clock: Rc<Clock>,
        sound_buffer: Arc<Mutex<dyn SoundOutput>>,
    ) -> Rc<RefCell<dyn Chip>> {
        let mut sid = Sid::new(chip_model, system_clock, sound_buffer);
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
        color_ram: Rc<RefCell<Ram>>,
        ram: Rc<RefCell<Ram>>,
        rom_charset: Rc<RefCell<Rom>>,
        vic_base_address: Rc<Cell<u16>>,
        frame_buffer: Rc<RefCell<dyn VideoOutput>>,
        vsync_flag: Rc<Cell<bool>>,
        ba_line: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>> {
        let vic_mem = VicMemory::new(vic_base_address, rom_charset, ram);
        Rc::new(RefCell::new(Vic::new(
            chip_model,
            color_ram,
            vic_mem,
            frame_buffer,
            vsync_flag,
            ba_line,
            irq_line,
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
}
