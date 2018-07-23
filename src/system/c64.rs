// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;
use std::sync::{Arc, Mutex};

use core::{Chip, ChipFactory, Clock, Cpu, IoPort, IrqLine, Pin, Ram, TickFn};
use device::joystick;
use device::{Cartridge, Datassette, ExpansionPort, Joystick, Keyboard, Tape};

use super::breakpoint::BreakpointManager;
use super::{Autostart, CircularBuffer, Config, FrameBuffer, Palette};

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
    pub fn addr(&self) -> u16 {
        *self as u16
    }
}

pub struct C64 {
    // Dependencies
    config: Rc<Config>,
    // Chipset
    cpu: Box<dyn Cpu>,
    cia_1: Rc<RefCell<dyn Chip>>,
    cia_2: Rc<RefCell<dyn Chip>>,
    sid: Rc<RefCell<dyn Chip>>,
    vic: Rc<RefCell<dyn Chip>>,
    // Memory
    color_ram: Rc<RefCell<Ram>>,
    ram: Rc<RefCell<Ram>>,
    // Peripherals
    datassette: Rc<RefCell<Datassette>>,
    expansion_port: Rc<RefCell<ExpansionPort>>,
    joystick_1: Option<Rc<RefCell<Joystick>>>,
    joystick_2: Option<Rc<RefCell<Joystick>>>,
    keyboard: Rc<RefCell<Keyboard>>,
    // Buffers
    frame_buffer: Rc<RefCell<FrameBuffer>>,
    sound_buffer: Arc<Mutex<CircularBuffer>>,
    // Configuration
    autostart: Option<Autostart>,
    breakpoints: BreakpointManager,
    // Runtime State
    clock: Rc<Clock>,
    frame_count: u32,
    last_pc: u16,
}

impl C64 {
    pub fn new(config: Rc<Config>, factory: Box<dyn ChipFactory>) -> Result<C64, io::Error> {
        info!(target: "c64", "Initializing system");
        // Buffers
        let clock = Rc::new(Clock::new());
        let frame_buffer = Rc::new(RefCell::new(FrameBuffer::new(
            config.model.frame_buffer_size.0,
            config.model.frame_buffer_size.1,
            Palette::default(),
        )));
        let joystick_1_state = Rc::new(Cell::new(0u8));
        let joystick_2_state = Rc::new(Cell::new(0u8));
        let keyboard_matrix = Rc::new(RefCell::new([0; 8]));
        let sound_buffer = Arc::new(Mutex::new(CircularBuffer::new(
            config.sound.buffer_size << 2,
        )));

        // I/O Lines
        let ba_line = Rc::new(RefCell::new(Pin::new_high()));
        let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_1_flag_pin = Rc::new(RefCell::new(Pin::new_low()));
        let cia_1_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_1_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_2_flag_pin = Rc::new(RefCell::new(Pin::new_low()));
        let cia_2_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_2_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let exp_io_line = Rc::new(RefCell::new(IoPort::new(0xff, 0xff)));
        let irq_line = Rc::new(RefCell::new(IrqLine::new("irq")));
        let nmi_line = Rc::new(RefCell::new(IrqLine::new("nmi")));

        // Memory
        let color_ram = factory.new_ram(config.model.color_ram);
        let ram = factory.new_ram(config.model.memory_size);
        let rom_basic = factory.new_rom(Path::new("res/rom/basic.rom"), BaseAddr::Basic.addr())?;
        let rom_charset = factory.new_rom(Path::new("res/rom/characters.rom"), 0)?;
        let rom_kernal = factory.new_rom(Path::new("res/rom/kernal.rom"), BaseAddr::Kernal.addr())?;

        // Chipset
        let cia_1 = factory.new_cia_1(
            cia_1_flag_pin.clone(),
            cia_1_port_a.clone(),
            cia_1_port_b.clone(),
            irq_line.clone(),
            joystick_1_state.clone(),
            joystick_2_state.clone(),
            keyboard_matrix.clone(),
        );
        let cia_2 = factory.new_cia_2(
            cia_2_flag_pin.clone(),
            cia_2_port_a.clone(),
            cia_2_port_b.clone(),
            nmi_line.clone(),
            keyboard_matrix.clone(),
        );
        let sid = factory.new_sid(&config.model, clock.clone(), sound_buffer.clone());
        let vic = factory.new_vic(
            config.model.vic_model,
            ba_line.clone(),
            cia_2_port_a.clone(),
            color_ram.clone(),
            frame_buffer.clone(),
            irq_line.clone(),
            ram.clone(),
            rom_charset.clone(),
        );

        // Memory Controller and Processor
        let expansion_port = Rc::new(RefCell::new(ExpansionPort::new(exp_io_line.clone())));
        let mem = factory.new_memory(
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
            ba_line.clone(),
            cpu_io_port.clone(),
            irq_line.clone(),
            nmi_line.clone(),
            mem.clone(),
        );

        // Peripherals
        let datassette = Rc::new(RefCell::new(Datassette::new(
            cia_1_flag_pin.clone(),
            cpu_io_port.clone(),
        )));
        let joystick1 = if config.joystick.joystick_1 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(Joystick::new(
                config.joystick.joystick_1,
                config.joystick.axis_motion_threshold,
                joystick_1_state.clone(),
            ))))
        } else {
            None
        };
        let joystick2 = if config.joystick.joystick_2 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(Joystick::new(
                config.joystick.joystick_2,
                config.joystick.axis_motion_threshold,
                joystick_2_state.clone(),
            ))))
        } else {
            None
        };
        let keyboard = Rc::new(RefCell::new(Keyboard::new(keyboard_matrix.clone())));

        // Observers
        let exp_io_line_clone_1 = exp_io_line.clone();
        let mem_clone_1 = mem.clone();
        cpu_io_port
            .borrow_mut()
            .set_observer(Box::new(move |cpu_port| {
                let expansion_port_io = exp_io_line_clone_1.borrow().get_value();
                let mode = cpu_port & 0x07 | expansion_port_io & 0x18;
                mem_clone_1.borrow_mut().switch_banks(mode);
            }));

        let cpu_io_port_clone_2 = cpu_io_port.clone();
        let mem_clone_2 = mem.clone();
        exp_io_line
            .borrow_mut()
            .set_observer(Box::new(move |expansion_port_io| {
                let cpu_port_io = cpu_io_port_clone_2.borrow().get_value();
                let mode = cpu_port_io & 0x07 | expansion_port_io & 0x18;
                mem_clone_2.borrow_mut().switch_banks(mode);
            }));

        Ok(C64 {
            config,
            cpu,
            sid: sid.clone(),
            vic: vic.clone(),
            cia_1: cia_1.clone(),
            cia_2: cia_2.clone(),
            color_ram: color_ram.clone(),
            expansion_port: expansion_port.clone(),
            ram: ram.clone(),
            datassette,
            joystick_1: joystick1,
            joystick_2: joystick2,
            keyboard: keyboard.clone(),
            frame_buffer: frame_buffer.clone(),
            sound_buffer: sound_buffer.clone(),
            autostart: None,
            breakpoints: BreakpointManager::new(),
            clock,
            frame_count: 0,
            last_pc: 0,
        })
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

    pub fn get_cpu(&self) -> &Box<Cpu> {
        &self.cpu
    }

    pub fn get_cpu_mut(&mut self) -> &mut Box<Cpu> {
        &mut self.cpu
    }

    pub fn get_cycles(&self) -> u64 {
        self.clock.get()
    }

    pub fn get_cia_1(&self) -> Rc<RefCell<dyn Chip>> {
        self.cia_1.clone()
    }

    pub fn get_cia_2(&self) -> Rc<RefCell<dyn Chip>> {
        self.cia_2.clone()
    }

    pub fn get_datasette(&self) -> Rc<RefCell<Datassette>> {
        self.datassette.clone()
    }

    pub fn get_frame_buffer(&self) -> Rc<RefCell<FrameBuffer>> {
        self.frame_buffer.clone()
    }

    pub fn get_frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn get_joystick(&self, index: u8) -> Option<Rc<RefCell<Joystick>>> {
        if let Some(ref joystick) = self.joystick_1 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        if let Some(ref joystick) = self.joystick_2 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        None
    }

    pub fn get_joystick1(&self) -> Option<Rc<RefCell<Joystick>>> {
        self.joystick_1.clone()
    }

    pub fn get_joystick2(&self) -> Option<Rc<RefCell<Joystick>>> {
        self.joystick_2.clone()
    }

    pub fn get_keyboard(&self) -> Rc<RefCell<Keyboard>> {
        self.keyboard.clone()
    }

    pub fn get_sid(&self) -> Rc<RefCell<dyn Chip>> {
        self.sid.clone()
    }

    pub fn get_sound_buffer(&self) -> Arc<Mutex<CircularBuffer>> {
        self.sound_buffer.clone()
    }

    pub fn get_vic(&self) -> Rc<RefCell<dyn Chip>> {
        self.vic.clone()
    }

    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.get_pc()
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    pub fn check_breakpoints(&mut self) -> bool {
        // FIXME self.breakpoints.check(&self.cpu).is_some()
        false
    }

    pub fn load(&mut self, data: &Vec<u8>, offset: u16) {
        let mut mem = self.ram.borrow_mut();
        let mut address = offset;
        for byte in data {
            mem.write(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    pub fn reset(&mut self, hard: bool) {
        info!(target: "c64", "Resetting system");
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
        if let Some(ref joystick) = self.joystick_1 {
            joystick.borrow_mut().reset();
        }
        if let Some(ref joystick) = self.joystick_2 {
            joystick.borrow_mut().reset();
        }
        self.keyboard.borrow_mut().reset();
        self.frame_buffer.borrow_mut().reset();
        self.sound_buffer.lock().unwrap().reset();
        // Runtime State
        // self.clock.reset();
        self.frame_count = 0;
        self.last_pc = 0;
    }

    pub fn run_frame(&mut self) -> bool {
        let cia_1_clone = self.cia_1.clone();
        let cia_2_clone = self.cia_2.clone();
        let clock_clone = self.clock.clone();
        let datassette_clone = self.datassette.clone();
        let vic_clone = self.vic.clone();
        let tick_fn: TickFn = Box::new(move || {
            vic_clone.borrow_mut().clock();
            cia_1_clone.borrow_mut().clock();
            cia_2_clone.borrow_mut().clock();
            datassette_clone.borrow_mut().clock();
            clock_clone.tick();
        });
        let bp_present = self.breakpoints.is_bp_present();
        let mut vsync = false;
        while !vsync {
            self.step_internal(&tick_fn);
            if bp_present && self.check_breakpoints() {
                break;
            }
            vsync = self.frame_buffer.borrow().get_sync();
        }
        if vsync {
            self.sid.borrow_mut().process_vsync();
            self.cia_1.borrow_mut().process_vsync();
            self.cia_2.borrow_mut().process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        vsync
    }

    pub fn step(&mut self) {
        let cia_1_clone = self.cia_1.clone();
        let cia_2_clone = self.cia_2.clone();
        let clock_clone = self.clock.clone();
        let datassette_clone = self.datassette.clone();
        let vic_clone = self.vic.clone();
        let tick_fn: TickFn = Box::new(move || {
            vic_clone.borrow_mut().clock();
            cia_1_clone.borrow_mut().clock();
            cia_2_clone.borrow_mut().clock();
            datassette_clone.borrow_mut().clock();
            clock_clone.tick();
        });
        self.step_internal(&tick_fn);
        if self.frame_buffer.borrow().get_sync() {
            self.sid.borrow_mut().process_vsync();
            self.cia_1.borrow_mut().process_vsync();
            self.cia_2.borrow_mut().process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
    }

    pub fn step_internal(&mut self, tick_fn: &TickFn) {
        self.last_pc = self.cpu.get_pc();
        self.cpu.step(&tick_fn);
        if self.autostart.is_some() {
            if self.cpu.get_pc() == BaseAddr::BootComplete.addr() {
                if let Some(mut autostart) = self.autostart.take() {
                    autostart.execute(self);
                }
            }
        }
    }

    // -- Peripherals Ops

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.expansion_port.borrow_mut().attach(cartridge);
    }

    pub fn attach_tape(&mut self, tape: Box<Tape>) {
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
    use core::SystemModel;

    #[test]
    fn verify_mem_layout() {
        let config = Rc::new(Config::new(SystemModel::from("pal")));
        let factory = Box::new(C64Factory::new(config.clone()));
        let mut c64 = C64::new(config.clone(), factory).unwrap();
        c64.reset(false);
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.read_debug(0xa000));
    }
}
