// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::too_many_arguments)]

use std::rc::Rc;
use std::sync::Arc;

use crate::board::{AddressableFaded, Memory, Mmio, Mmu};
use crate::chip::cia::{self, Cia};
use crate::chip::sid::{SamplingMethod, Sid, SidModel};
use crate::chip::vic::{Vic, VicMemory, VicModel};
use crate::chip::{Addressable, Chip, Cpu, Cpu6510, Ram, Rom, SoundOutput, VideoOutput};
use crate::config::Config;
use crate::shared::{Shared, SharedCell, new_shared};
use crate::signal::{Clock, IoPort, IrqLine, Pin};

/// ChipFactory serves as the foundation of an extensible emulator architecture and
/// provides an interface to construct each chip/component within the system.
/// It allows for each component to be swapped out and replaced by different implementation.
/// To accomplish this, special considerations were made to model interactions between chips
/// without coupling them together. All interactions are managed through separate I/O state
/// provided as input to each of the chip constructors (`IrqLine`, `Pin`).
///
/// Four core traits used to model system operation are `Chip`, `Cpu`, `Mmu` and `Addressable`.
/// The consumer of chip factory (usually an emulator) will use these four traits to interact
/// with each component of the system.
///
pub trait ChipFactory {
    /// Constructs CPU.
    ///
    /// The three least significant bits in the port register correspond to the three
    /// control lines used for bank switching.
    ///
    /// # Dependencies
    /// `mem` - memory management unit
    /// # I/O
    /// `io_port` - cpu I/O port
    /// # Signals
    /// `ba_line` - ba input
    /// `irq_line` - interrupt request input
    /// `nmi_line` - non-maskable interrupt request input
    fn new_cpu(
        &self,
        mem: Shared<dyn Addressable>,
        io_port: Shared<IoPort>,
        ba_line: Shared<Pin>,
        irq_line: Shared<IrqLine>,
        nmi_line: Shared<IrqLine>,
    ) -> Box<dyn Cpu>;

    // -- Chipset

    /// Constructs CIA 1 chip.
    ///
    /// CIA 1 is connected to the two control ports used to connect joysticks.
    /// Keyboard matrix is also connected to CIA 1 port B.
    ///
    /// # Dependencies
    /// `joystick_1` - joystick 1 state
    /// `joystick_2` - joystick 2 state
    /// `keyboard_matrix` - keyboard state
    /// # I/O
    /// `port_a` - I/O port A
    /// `port_b` - I/O port B
    /// # Signals
    /// `flag_pin` - flag input pin
    /// `irq_line` - interrupt request output
    fn new_cia_1(
        &self,
        joystick_1: SharedCell<u8>,
        joystick_2: SharedCell<u8>,
        keyboard_matrix: Shared<[u8; 16]>,
        port_a: Shared<IoPort>,
        port_b: Shared<IoPort>,
        flag_pin: Shared<Pin>,
        irq_line: Shared<IrqLine>,
    ) -> Shared<dyn Chip>;

    /// Constructs CIA 2 chip.
    ///
    /// # I/O
    /// `port_a` - I/O port A
    /// `port_b` - I/O port B
    /// # Signals
    /// `flag_pin` - flag input pin
    /// `nmi_line` - interrupt request output
    fn new_cia_2(
        &self,
        port_a: Shared<IoPort>,
        port_b: Shared<IoPort>,
        flag_pin: Shared<Pin>,
        nmi_line: Shared<IrqLine>,
    ) -> Shared<dyn Chip>;

    /// Constructs SID chip.
    ///
    /// Since SID processing may be invoked only during v-sync, system clock is provided
    /// to allow SID to sync up sound generation to the current cycle when a register
    /// read or write is performed.
    ///
    /// SID output is written to the provided sound buffer.
    ///
    /// # Dependencies
    /// `chip_model` - choose either 6581 or 8580
    /// `system_clock` - system clock
    /// # I/O
    /// `sound_buffer` - output for generated 16-bit sound samples
    fn new_sid(
        &self,
        chip_model: SidModel,
        system_clock: Rc<Clock>,
        sound_buffer: Arc<dyn SoundOutput>,
    ) -> Shared<dyn Chip>;

    /// Constructs VIC chip.
    ///
    /// Since VIC relies on CIA 2 port A for its memory address generation,
    /// the memory base address is provided through `vic_base_address`. This is an optimization
    /// as `vic_base_address` will be updated only when CIA 2 port A changes.
    ///
    /// VIC output is written to provided frame buffer. VIC should also set vsync flag
    /// when v-sync condition exists.
    ///
    /// # Dependencies
    /// `chip_model` - choose either 6567 or 6569
    /// `color_ram` - 1K*4 bit color ram
    /// `ram` - 64KB main memory
    /// `rom_charset` - 4KB character generator ROM
    /// `vic_base_address` - memory base address as defined by CIA 2 port A bits 0 and 1
    /// # I/O
    /// `frame_buffer` - pixel color information is written here
    /// `vsync_flag` - set when vsync condition is reached
    /// # Signals
    /// `ba_line` - ba output
    /// `irq_line` - interrupt request output
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
    ) -> Shared<dyn Chip>;

    // -- Memory

    /// Constructs memory controller.
    ///
    /// Memory is used by the CPU to access banks of memory accessible at
    /// any given time. Bank switching is controlled through 5 latch bits that control
    /// the memory management unit (LORAM, HIRAM, CHAREN, GAME, EXROM) that does address
    /// translation.
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
    ) -> Shared<dyn Addressable>;

    /// Constructs RAM with the specified `capacity`.
    fn new_ram(&self, capacity: usize) -> Shared<Ram>;

    /// Constructs ROM based on the specified image file.
    fn new_rom(&self, data: &[u8], offset: u16) -> Shared<Rom>;
}

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
