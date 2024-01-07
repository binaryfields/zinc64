// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

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

use super::breakpoint::BreakpointManager;
use super::{Autostart, Config};
use zinc64_core::device::joystick;
use zinc64_core::device::{Cartridge, Datassette, Joystick, Keyboard};
use zinc64_core::factory::Tape;
use zinc64_core::mem::{ExpansionPort, Pla};

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum BaseAddr {
    Basic = 0xa000,
    BootComplete = 0xa65c,
    Charset = 0xd000,
    Kernal = 0xe000,
}

impl BaseAddr {
    pub fn addr(self) -> u16 {
        self as u16
    }
}

pub struct C64 {
    // Dependencies
    config: Rc<Config>,
    // Chipset
    cpu: Box<dyn Cpu>,
    cia_1: Shared<dyn Chip>,
    cia_2: Shared<dyn Chip>,
    sid: Shared<dyn Chip>,
    vic: Shared<dyn Chip>,
    // Memory
    color_ram: Shared<Ram>,
    expansion_port: Shared<ExpansionPort>,
    ram: Shared<Ram>,
    // Peripherals
    datassette: Shared<Datassette>,
    joystick_1: Option<Joystick>,
    joystick_2: Option<Joystick>,
    keyboard: Keyboard,
    // Buffers
    frame_buffer: Shared<dyn VideoOutput>,
    sound_buffer: Arc<dyn SoundOutput>,
    // Runtime State
    autostart: Option<Autostart>,
    breakpoints: BreakpointManager,
    clock: Rc<Clock>,
    frame_count: u32,
    last_pc: u16,
    tick_fn: TickFn,
    vsync_flag: SharedCell<bool>,
}

impl C64 {
    pub fn build(
        config: Rc<Config>,
        factory: &dyn ChipFactory,
        frame_buffer: Shared<dyn VideoOutput>,
        sound_buffer: Arc<dyn SoundOutput>,
    ) -> C64 {
        info!(target: "c64", "Initializing system");
        // Buffers
        let clock = Rc::new(Clock::default());
        let joystick_1_state = new_shared_cell(0u8);
        let joystick_2_state = new_shared_cell(0u8);
        let keyboard_matrix = new_shared([0; 16]);
        let vsync_flag = new_shared_cell(false);
        let vic_base_address = new_shared_cell(0u16);

        // I/O Lines
        let ba_line = new_shared(Pin::new_high());
        let cpu_io_port = new_shared(IoPort::new(0x00, 0xff));
        let cia_1_flag_pin = new_shared(Pin::new_low());
        let cia_1_port_a = new_shared(IoPort::new(0x00, 0xff));
        let cia_1_port_b = new_shared(IoPort::new(0x00, 0xff));
        let cia_2_flag_pin = new_shared(Pin::new_low());
        let cia_2_port_a = new_shared(IoPort::new(0x00, 0xff));
        let cia_2_port_b = new_shared(IoPort::new(0x00, 0xff));
        let exp_io_line = new_shared(IoPort::new(0xff, 0xff));
        let irq_line = new_shared(IrqLine::new("irq"));
        let nmi_line = new_shared(IrqLine::new("nmi"));

        // Memory
        let color_ram = factory.new_ram(config.model.color_ram);
        let ram = factory.new_ram(config.model.memory_size);
        let rom_basic = factory.new_rom(config.roms.basic.as_slice(), BaseAddr::Basic.addr());
        let rom_charset = factory.new_rom(config.roms.charset.as_slice(), 0);
        let rom_kernal = factory.new_rom(config.roms.kernal.as_slice(), BaseAddr::Kernal.addr());

        // Chipset
        let cia_1 = factory.new_cia_1(
            joystick_1_state.clone(),
            joystick_2_state.clone(),
            keyboard_matrix.clone(),
            cia_1_port_a.clone(),
            cia_1_port_b.clone(),
            cia_1_flag_pin.clone(),
            irq_line.clone(),
        );
        let cia_2 = factory.new_cia_2(
            cia_2_port_a.clone(),
            cia_2_port_b.clone(),
            cia_2_flag_pin.clone(),
            nmi_line.clone(),
        );
        let sid = factory.new_sid(config.model.sid_model, clock.clone(), sound_buffer.clone());
        let vic = factory.new_vic(
            config.model.vic_model,
            color_ram.clone(),
            ram.clone(),
            rom_charset.clone(),
            vic_base_address.clone(),
            frame_buffer.clone(),
            vsync_flag.clone(),
            ba_line.clone(),
            irq_line.clone(),
        );

        // Memory Controller and Processor
        let expansion_port = new_shared(ExpansionPort::new(exp_io_line.clone()));
        let mmu = new_shared(Pla::new());
        let mem = factory.new_memory(
            mmu.clone(),
            cia_1.clone(),
            cia_2.clone(),
            color_ram.clone(),
            expansion_port.clone(),
            ram.clone(),
            rom_basic.clone(),
            rom_charset.clone(),
            rom_kernal.clone(),
            sid.clone(),
            vic.clone(),
        );
        let cpu = factory.new_cpu(
            mem.clone(),
            cpu_io_port.clone(),
            ba_line.clone(),
            irq_line.clone(),
            nmi_line.clone(),
        );

        // Peripherals
        let datassette = new_shared(Datassette::new(cia_1_flag_pin.clone(), cpu_io_port.clone()));
        let joystick1 = if config.joystick.joystick_1 != joystick::Mode::None {
            Some(Joystick::new(
                config.joystick.joystick_1,
                config.joystick.axis_motion_threshold,
                joystick_1_state.clone(),
            ))
        } else {
            None
        };
        let joystick2 = if config.joystick.joystick_2 != joystick::Mode::None {
            Some(Joystick::new(
                config.joystick.joystick_2,
                config.joystick.axis_motion_threshold,
                joystick_2_state.clone(),
            ))
        } else {
            None
        };
        let keyboard = Keyboard::new(keyboard_matrix.clone());

        // Observers
        let exp_io_line_clone_1 = exp_io_line.clone();
        let mmu_clone_1 = mmu.clone();
        cpu_io_port
            .borrow_mut()
            .set_observer(Box::new(move |cpu_port| {
                let expansion_port_io = exp_io_line_clone_1.borrow().get_value();
                let mode = cpu_port & 0x07 | expansion_port_io & 0x18;
                mmu_clone_1.borrow_mut().switch_banks(mode);
            }));

        let cpu_io_port_clone_2 = cpu_io_port.clone();
        let mmu_clone_2 = mmu.clone();
        exp_io_line
            .borrow_mut()
            .set_observer(Box::new(move |expansion_port_io| {
                let cpu_port_io = cpu_io_port_clone_2.borrow().get_value();
                let mode = cpu_port_io & 0x07 | expansion_port_io & 0x18;
                mmu_clone_2.borrow_mut().switch_banks(mode);
            }));
        let vic_base_address_clone = vic_base_address.clone();
        cia_2_port_a
            .borrow_mut()
            .set_observer(Box::new(move |value| {
                let base_address = ((!value & 0x03) as u16) << 14;
                vic_base_address_clone.set(base_address);
            }));
        let tick_fn: TickFn = {
            let cia_1_clone = cia_1.clone();
            let cia_2_clone = cia_2.clone();
            let clock_clone = clock.clone();
            let datassette_clone = datassette.clone();
            let vic_clone = vic.clone();
            Rc::new(move || {
                vic_clone.borrow_mut().clock();
                cia_1_clone.borrow_mut().clock();
                cia_2_clone.borrow_mut().clock();
                datassette_clone.borrow_mut().clock();
                clock_clone.tick();
            })
        };
        C64 {
            config,
            cpu,
            cia_1: cia_1.clone(),
            cia_2: cia_2.clone(),
            sid: sid.clone(),
            vic: vic.clone(),
            color_ram: color_ram.clone(),
            expansion_port: expansion_port.clone(),
            ram: ram.clone(),
            datassette,
            joystick_1: joystick1,
            joystick_2: joystick2,
            keyboard,
            frame_buffer: frame_buffer.clone(),
            sound_buffer: sound_buffer.clone(),
            autostart: None,
            breakpoints: BreakpointManager::default(),
            clock,
            frame_count: 0,
            last_pc: 0,
            tick_fn,
            vsync_flag,
        }
    }

    pub fn get_bpm(&self) -> &BreakpointManager {
        &self.breakpoints
    }

    pub fn get_bpm_mut(&mut self) -> &mut BreakpointManager {
        &mut self.breakpoints
    }

    pub fn get_clock(&self) -> Rc<Clock> {
        self.clock.clone()
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_cpu(&self) -> &dyn Cpu {
        &*self.cpu
    }

    pub fn get_cpu_mut(&mut self) -> &mut dyn Cpu {
        &mut *self.cpu
    }

    pub fn get_cycles(&self) -> u64 {
        self.clock.get()
    }

    pub fn get_cia_1(&self) -> Shared<dyn Chip> {
        self.cia_1.clone()
    }

    pub fn get_cia_2(&self) -> Shared<dyn Chip> {
        self.cia_2.clone()
    }

    pub fn get_datasette(&self) -> Shared<Datassette> {
        self.datassette.clone()
    }

    pub fn get_frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn get_joystick1(&self) -> &Option<Joystick> {
        &self.joystick_1
    }

    pub fn get_joystick1_mut(&mut self) -> &mut Option<Joystick> {
        &mut self.joystick_1
    }

    pub fn get_joystick2(&self) -> &Option<Joystick> {
        &self.joystick_2
    }

    pub fn get_joystick2_mut(&mut self) -> &mut Option<Joystick> {
        &mut self.joystick_2
    }

    pub fn get_keyboard(&mut self) -> &mut Keyboard {
        &mut self.keyboard
    }

    pub fn get_sid(&self) -> Shared<dyn Chip> {
        self.sid.clone()
    }

    pub fn get_vic(&self) -> Shared<dyn Chip> {
        self.vic.clone()
    }

    pub fn get_vsync(&self) -> bool {
        self.vsync_flag.get()
    }

    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.get_pc()
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    pub fn reset_vsync(&self) {
        self.vsync_flag.set(false)
    }

    pub fn check_breakpoints(&mut self) -> bool {
        self.breakpoints.check(&*self.cpu).is_some()
    }

    pub fn load(&mut self, data: &[u8], offset: u16) {
        let mut mem = self.ram.borrow_mut();
        let mut address = offset;
        for byte in data {
            mem.write(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    pub fn reset(&mut self, hard: bool) {
        info!(target: "c64", "Resetting system");
        self.clock.reset();
        // Memory
        if hard {
            for i in 0..self.config.model.memory_size as u16 {
                self.ram.borrow_mut().write(i, 0x00);
            }
            for i in 0..self.config.model.color_ram as u16 {
                self.color_ram.borrow_mut().write(i, 0x00);
            }
        }
        // Chipset
        self.cpu.reset();
        self.cia_1.borrow_mut().reset();
        self.cia_2.borrow_mut().reset();
        self.sid.borrow_mut().reset();
        self.vic.borrow_mut().reset();
        // I/O
        self.expansion_port.borrow_mut().reset();
        // Peripherals
        self.datassette.borrow_mut().reset();
        if let Some(ref mut joystick) = self.joystick_1 {
            joystick.reset();
        }
        if let Some(ref mut joystick) = self.joystick_2 {
            joystick.reset();
        }
        self.keyboard.reset();
        self.frame_buffer.borrow_mut().reset();
        self.sound_buffer.reset();
        // Runtime State
        self.frame_count = 0;
        self.last_pc = 0;
        self.vsync_flag.set(false);
    }

    pub fn run_frame(&mut self) -> bool {
        let tick_fn = self.tick_fn.clone();
        let bp_present = self.breakpoints.is_bp_present();
        while !self.vsync_flag.get() {
            self.step_internal(&tick_fn);
            if bp_present && self.check_breakpoints() {
                break;
            }
        }
        if self.vsync_flag.get() {
            self.sid.borrow_mut().process_vsync();
            self.cia_1.borrow_mut().process_vsync();
            self.cia_2.borrow_mut().process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        self.vsync_flag.get()
    }

    pub fn step(&mut self) {
        let tick_fn = self.tick_fn.clone();
        self.step_internal(&tick_fn);
        if self.vsync_flag.get() {
            self.sid.borrow_mut().process_vsync();
            self.cia_1.borrow_mut().process_vsync();
            self.cia_2.borrow_mut().process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
    }

    #[inline]
    pub fn step_internal(&mut self, tick_fn: &TickFn) {
        self.last_pc = self.cpu.get_pc();
        self.cpu.step(&tick_fn);
        if self.autostart.is_some() && self.cpu.get_pc() == BaseAddr::BootComplete.addr() {
            if let Some(mut autostart) = self.autostart.take() {
                autostart.execute(self);
            }
        }
    }

    // -- Peripherals Ops

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.expansion_port.borrow_mut().attach(cartridge);
    }

    pub fn attach_tape(&mut self, tape: Box<dyn Tape>) {
        self.datassette.borrow_mut().attach(tape);
    }

    pub fn detach_cartridge(&mut self) {
        self.expansion_port.borrow_mut().detach();
        self.reset(false);
    }

    pub fn detach_tape(&mut self) {
        self.datassette.borrow_mut().detach();
    }
}

#[cfg(test)]
mod tests {
    use super::super::C64Factory;
    use super::*;
    use zinc64_core::factory::SystemModel;

    static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
    static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
    static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");

    #[test]
    fn verify_mem_layout() {
        let config = Rc::new(Config::new_with_roms(
            SystemModel::from("pal"),
            RES_BASIC_ROM,
            RES_CHARSET_ROM,
            RES_KERNAL_ROM,
        ));
        let factory = Box::new(C64Factory::new(config.clone()));
        let video_output = new_shared(NullVideo {});
        let sound_output = Arc::new(NullSound {});
        let mut c64 = C64::build(config.clone(), &*factory, video_output, sound_output);
        c64.reset(false);
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.read(0xa000));
    }

    struct NullSound;
    impl SoundOutput for NullSound {
        fn reset(&self) {}
        fn write(&self, _samples: &[i16]) {}
    }

    struct NullVideo;
    impl VideoOutput for NullVideo {
        fn get_dimension(&self) -> (usize, usize) {
            (0, 0)
        }
        fn reset(&mut self) {}
        fn write(&mut self, _index: usize, _color: u8) {}
    }
}
