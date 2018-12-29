// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::core::{
    Addressable, Chip, Clock, Cpu, IoPort, IrqLine, Mmu, Pin, Ram, Rom, SidModel, SoundOutput,
    VicModel, VideoOutput,
};

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
        mem: Rc<RefCell<dyn Mmu>>,
        io_port: Rc<RefCell<IoPort>>,
        ba_line: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
        nmi_line: Rc<RefCell<IrqLine>>,
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
        joystick_1: Rc<Cell<u8>>,
        joystick_2: Rc<Cell<u8>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
        port_a: Rc<RefCell<IoPort>>,
        port_b: Rc<RefCell<IoPort>>,
        flag_pin: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>>;

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
        port_a: Rc<RefCell<IoPort>>,
        port_b: Rc<RefCell<IoPort>>,
        flag_pin: Rc<RefCell<Pin>>,
        nmi_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>>;

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
        sound_buffer: Arc<Mutex<dyn SoundOutput>>,
    ) -> Rc<RefCell<dyn Chip>>;

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
        color_ram: Rc<RefCell<Ram>>,
        ram: Rc<RefCell<Ram>>,
        rom_charset: Rc<RefCell<Rom>>,
        vic_base_address: Rc<Cell<u16>>,
        frame_buffer: Rc<RefCell<dyn VideoOutput>>,
        vsync_flag: Rc<Cell<bool>>,
        ba_line: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
    ) -> Rc<RefCell<dyn Chip>>;

    // -- Memory

    /// Constructs expansion port that is used to connect cartridges.
    ///
    /// GAME and EXROM lines are use for memory bank switching.
    ///
    /// # I/O
    /// `exp_io_line` - exposes cartridge GAME and EXROM lines (bits 3 and 4)
    fn new_expansion_port(&self, exp_io_line: Rc<RefCell<IoPort>>) -> Rc<RefCell<dyn Addressable>>;

    /// Constructs memory controller.
    ///
    /// Memory controller is used by the CPU to access banks of memory accessible at
    /// any given time. Bank switching is controlled through 5 latch bits that control
    /// the memory configurations (LORAM, HIRAM, CHAREN, GAME, EXROM).
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

    /// Constructs RAM with the specified `capacity`.
    fn new_ram(&self, capacity: usize) -> Rc<RefCell<Ram>>;

    /// Constructs ROM based on the specified image file.
    fn new_rom(&self, path: &Path, offset: u16) -> Result<Rc<RefCell<Rom>>, io::Error>;
}
