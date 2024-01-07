// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(feature = "std")]
use std::rc::Rc;
#[cfg(feature = "std")]
use std::sync::Arc;
use zinc64_core::factory::*;
use zinc64_core::util::*;

use super::Config;
use zinc64_core::cpu::Cpu6510;
use zinc64_core::io::cia;
use zinc64_core::io::Cia;
use zinc64_core::mem::{Memory, Mmio};
use zinc64_core::sound::sid::SamplingMethod;
use zinc64_core::sound::Sid;
use zinc64_core::video::{Vic, VicMemory};

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
        mem: Shared<dyn Addressable>,
        io_port: Shared<IoPort>,
        ba_line: Shared<Pin>,
        irq_line: Shared<IrqLine>,
        nmi_line: Shared<IrqLine>,
    ) -> Box<dyn Cpu> {
        Box::new(Cpu6510::new(mem, io_port, ba_line, irq_line, nmi_line))
    }

    // -- Chipset

    fn new_cia_1(
        &self,
        joystick_1: SharedCell<u8>,
        joystick_2: SharedCell<u8>,
        keyboard_matrix: Shared<[u8; 16]>,
        port_a: Shared<IoPort>,
        port_b: Shared<IoPort>,
        flag_pin: Shared<Pin>,
        irq_line: Shared<IrqLine>,
    ) -> Shared<dyn Chip> {
        new_shared(Cia::new(
            cia::Mode::Cia1,
            Some(joystick_1),
            Some(joystick_2),
            Some(keyboard_matrix),
            port_a,
            port_b,
            flag_pin,
            irq_line,
        ))
    }

    fn new_cia_2(
        &self,
        port_a: Shared<IoPort>,
        port_b: Shared<IoPort>,
        flag_pin: Shared<Pin>,
        nmi_line: Shared<IrqLine>,
    ) -> Shared<dyn Chip> {
        new_shared(Cia::new(
            cia::Mode::Cia2,
            None,
            None,
            None,
            port_a,
            port_b,
            flag_pin,
            nmi_line,
        ))
    }

    fn new_sid(
        &self,
        chip_model: SidModel,
        system_clock: Rc<Clock>,
        sound_buffer: Arc<dyn SoundOutput>,
    ) -> Shared<dyn Chip> {
        let mut sid = Sid::new(chip_model, system_clock, sound_buffer);
        sid.set_sampling_parameters(
            SamplingMethod::Fast,
            self.config.model.cpu_freq,
            self.config.sound.sample_rate,
        );
        sid.enable_filter(self.config.sound.sid_filters);
        new_shared(sid)
    }

    fn new_vic(
        &self,
        chip_model: VicModel,
        color_ram: Shared<Ram>,
        ram: Shared<Ram>,
        rom_charset: Shared<Rom>,
        vic_base_address: SharedCell<u16>,
        frame_buffer: Shared<dyn VideoOutput>,
        vsync_flag: SharedCell<bool>,
        ba_line: Shared<Pin>,
        irq_line: Shared<IrqLine>,
    ) -> Shared<dyn Chip> {
        let vic_mem = VicMemory::new(vic_base_address, rom_charset, ram);
        new_shared(Vic::new(
            chip_model,
            color_ram,
            vic_mem,
            frame_buffer,
            vsync_flag,
            ba_line,
            irq_line,
        ))
    }

    // -- Memory

    fn new_memory(
        &self,
        mmu: Shared<dyn Mmu>,
        cia_1: Shared<dyn Chip>,
        cia_2: Shared<dyn Chip>,
        color_ram: Shared<Ram>,
        expansion_port: Shared<dyn AddressableFaded>,
        ram: Shared<Ram>,
        rom_basic: Shared<Rom>,
        rom_charset: Shared<Rom>,
        rom_kernal: Shared<Rom>,
        sid: Shared<dyn Chip>,
        vic: Shared<dyn Chip>,
    ) -> Shared<dyn Addressable> {
        let io = Mmio::new(cia_1, cia_2, color_ram, expansion_port.clone(), sid, vic);
        new_shared(Memory::new(
            mmu,
            expansion_port.clone(),
            io,
            ram,
            rom_basic,
            rom_charset,
            rom_kernal,
        ))
    }

    fn new_ram(&self, capacity: usize) -> Shared<Ram> {
        new_shared(Ram::new(capacity))
    }

    fn new_rom(&self, data: &[u8], offset: u16) -> Shared<Rom> {
        new_shared(Rom::new_with_data(data, offset))
    }
}
