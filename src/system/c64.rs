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
use std::path::Path;
use std::io;
use std::rc::Rc;
use std::result::Result;
use std::sync::{Arc, Mutex};

use core::{
    Chip,
    Cpu,
    Factory,
    FrameBuffer,
    IoPort,
    IrqLine,
    Pin,
    Ram,
    SoundBuffer,
    TickFn,
};
use device::{
    Cartridge,
    Datassette,
    ExpansionPort,
    Joystick,
    Keyboard,
    Tape,
};
use device::joystick;

use super::{Autostart, Clock, Config, Palette};

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
    cpu: Rc<RefCell<Cpu>>,
    cia1: Rc<RefCell<Chip>>,
    cia2: Rc<RefCell<Chip>>,
    sid: Rc<RefCell<Chip>>,
    vic: Rc<RefCell<Chip>>,
    // Memory
    color_ram: Rc<RefCell<Ram>>,
    ram: Rc<RefCell<Ram>>,
    // Peripherals
    datassette: Rc<RefCell<Datassette>>,
    expansion_port: Rc<RefCell<ExpansionPort>>,
    joystick1: Option<Rc<RefCell<Joystick>>>,
    joystick2: Option<Rc<RefCell<Joystick>>>,
    keyboard: Rc<RefCell<Keyboard>>,
    // Buffers
    frame_buffer: Rc<RefCell<FrameBuffer>>,
    sound_buffer: Arc<Mutex<SoundBuffer>>,
    // Configuration
    autostart: Option<Autostart>,
    breakpoints: Vec<u16>,
    // Runtime State
    clock: Rc<Clock>,
    frames: u32,
    last_pc: u16,
}

impl C64 {
    pub fn new(config: Rc<Config>, factory: Box<Factory>) -> Result<C64, io::Error> {
        info!(target: "c64", "Initializing system");
        // Buffers
        let frame_buffer = Rc::new(RefCell::new(
            FrameBuffer::new(
                config.model.frame_buffer_size.0,
                config.model.frame_buffer_size.1,
                Palette::default(),
            )
        ));
        let joystick_1_state = Rc::new(Cell::new(0u8));
        let joystick_2_state = Rc::new(Cell::new(0u8));
        let keyboard_matrix = Rc::new(RefCell::new([0; 8]));
        let sound_buffer = Arc::new(Mutex::new(
            SoundBuffer::new(config.sound.buffer_size),
        ));

        // I/O Lines
        let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
        let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
        let exp_io_line = Rc::new(RefCell::new(IoPort::new(0xff, 0xff)));
        let cia_1_flag = Rc::new(RefCell::new(Pin::new_low()));
        let cia_1_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_1_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_2_flag = Rc::new(RefCell::new(Pin::new_low()));
        let cia_2_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_2_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));

        // Memory
        let color_ram = factory.new_ram(config.model.color_ram);
        let ram = factory.new_ram(config.model.memory_size);
        let basic = factory.new_rom(
            Path::new("res/rom/basic.rom"),
            BaseAddr::Basic.addr(),
        )?;
        let charset = factory.new_rom(
            Path::new("res/rom/characters.rom"),
            0,
        )?;
        let kernal = factory.new_rom(
            Path::new("res/rom/kernal.rom"),
            BaseAddr::Kernal.addr(),
        )?;

        // Chipset
        let cia1 = factory.new_cia1(
            cia_1_flag.clone(),
            cia_1_port_a.clone(),
            cia_1_port_b.clone(),
            cpu_irq.clone(),
            cpu_nmi.clone(),
            joystick_1_state.clone(),
            joystick_2_state.clone(),
            keyboard_matrix.clone(),
        );
        let cia2 = factory.new_cia2(
            cia_2_flag.clone(),
            cia_2_port_a.clone(),
            cia_2_port_b.clone(),
            cpu_irq.clone(),
            cpu_nmi.clone(),
            keyboard_matrix.clone(),
        );
        let sid = factory.new_sid(
            &config.model,
            sound_buffer.clone(),
        );
        let vic = factory.new_vic(
            config.model.vic_model,
            charset.clone(),
            cia_2_port_a.clone(),
            color_ram.clone(),
            cpu_irq.clone(),
            frame_buffer.clone(),
            ram.clone(),
        );

        // Memory Controller and Processor
        let expansion_port = Rc::new(RefCell::new(
            ExpansionPort::new(
                exp_io_line.clone()
            )
        ));
        let mem = factory.new_memory(
            cia1.clone(),
            cia2.clone(),
            color_ram.clone(),
            expansion_port.clone(),
            ram.clone(),
            basic.clone(),
            charset.clone(),
            kernal.clone(),
            sid.clone(),
            vic.clone(),
        );
        let cpu = factory.new_cpu(
            cpu_io_port.clone(),
            cpu_irq.clone(),
            cpu_nmi.clone(),
            mem.clone(),
        );

        // Peripherals
        let datassette = Rc::new(RefCell::new(
            Datassette::new(
                cia_1_flag.clone(),
                cpu_io_port.clone(),
            )
        ));
        let joystick1 = if config.joystick.joystick_1 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(
                Joystick::new(
                    config.joystick.joystick_1,
                    config.joystick.axis_motion_threshold,
                    joystick_1_state.clone(),
                )
            )))
        } else {
            None
        };
        let joystick2 = if config.joystick.joystick_2 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(
                Joystick::new(
                    config.joystick.joystick_2,
                    config.joystick.axis_motion_threshold,
                    joystick_2_state.clone(),
                )
            )))
        } else {
            None
        };
        let keyboard = Rc::new(RefCell::new(
            Keyboard::new(keyboard_matrix.clone())
        ));

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
            cpu: cpu.clone(),
            sid: sid.clone(),
            vic: vic.clone(),
            cia1: cia1.clone(),
            cia2: cia2.clone(),
            color_ram: color_ram.clone(),
            expansion_port: expansion_port.clone(),
            ram: ram.clone(),
            datassette,
            joystick1,
            joystick2,
            keyboard: keyboard.clone(),
            frame_buffer: frame_buffer.clone(),
            sound_buffer: sound_buffer.clone(),
            autostart: None,
            breakpoints: Vec::new(),
            clock: Rc::new(Clock::new()),
            frames: 0,
            last_pc: 0,
        })
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> {
        self.cpu.clone()
    }

    pub fn get_cycles(&self) -> u64 {
        self.clock.get()
    }

    pub fn get_datasette(&self) -> Rc<RefCell<Datassette>> {
        self.datassette.clone()
    }

    pub fn get_frame_buffer(&self) -> Rc<RefCell<FrameBuffer>> {
        self.frame_buffer.clone()
    }

    pub fn get_joystick(&self, index: u8) -> Option<Rc<RefCell<Joystick>>> {
        if let Some(ref joystick) = self.joystick1 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        if let Some(ref joystick) = self.joystick2 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        None
    }

    pub fn get_joystick1(&self) -> Option<Rc<RefCell<Joystick>>> {
        self.joystick1.clone()
    }

    pub fn get_joystick2(&self) -> Option<Rc<RefCell<Joystick>>> {
        self.joystick2.clone()
    }

    pub fn get_keyboard(&self) -> Rc<RefCell<Keyboard>> {
        self.keyboard.clone()
    }

    pub fn get_sound_buffer(&self) -> Arc<Mutex<SoundBuffer>> {
        self.sound_buffer.clone()
    }

    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.borrow().get_pc()
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
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
        self.cpu.borrow_mut().reset();
        self.cia1.borrow_mut().reset();
        self.cia2.borrow_mut().reset();
        self.sid.borrow_mut().reset();
        self.vic.borrow_mut().reset();
        // I/O
        self.expansion_port.borrow_mut().reset();
        // Peripherals
        self.datassette.borrow_mut().reset();
        if let Some(ref joystick) = self.joystick1 {
            joystick.borrow_mut().reset();
        }
        if let Some(ref joystick) = self.joystick2 {
            joystick.borrow_mut().reset();
        }
        self.keyboard.borrow_mut().reset();
        self.frame_buffer.borrow_mut().reset();
        self.sound_buffer.lock().unwrap().clear();
        // Runtime State
        // self.clock.set(0);
        self.frames = 0;
        self.last_pc = 0;
    }

    pub fn run_frame(&mut self, overflow_cycles: i32) -> i32 {
        let mut elapsed = 0u32;
        let mut delta = self.config.model.cycles_per_frame as i32 - overflow_cycles;
        let vic_clone = self.vic.clone();
        let cia1_clone = self.cia1.clone();
        let cia2_clone = self.cia2.clone();
        let datassette_clone = self.datassette.clone();
        let clock_clone = self.clock.clone();
        let tick_fn: TickFn = Box::new(move || {
            vic_clone.borrow_mut().clock();
            cia1_clone.borrow_mut().clock();
            cia2_clone.borrow_mut().clock();
            datassette_clone.borrow_mut().clock();
            clock_clone.tick();
        });
        while delta > 0 {
            let prev_clk = self.clock.get();
            self.step(&tick_fn);
            let cycles = (self.clock.get() - prev_clk) as u32;
            elapsed += cycles;
            delta -= cycles as i32;
        }
        self.sid.borrow_mut().clock_delta(elapsed);
        self.cia1.borrow_mut().process_vsync();
        self.cia2.borrow_mut().process_vsync();
        self.frames = self.frames.wrapping_add(1);
        delta
    }

    #[inline]
    pub fn step(&mut self, tick_fn: &TickFn) {
        self.last_pc = self.cpu.borrow().get_pc();
        self.cpu.borrow_mut().step(&tick_fn);
        if self.autostart.is_some() {
            if self.cpu.borrow().get_pc() == BaseAddr::BootComplete.addr() {
                if let Some(mut autostart) = self.autostart.take() {
                    autostart.execute(self);
                }
            }
        }
    }

    // -- Debug Ops

    pub fn add_breakpoint(&mut self, breakpoint: u16) {
        self.breakpoints.push(breakpoint);
    }

    #[allow(dead_code)]
    pub fn check_breakpoints(&self) -> bool {
        let pc = self.cpu.borrow().get_pc();
        !self.breakpoints.is_empty() && self.breakpoints.contains(&pc)
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
    use super::*;
    use super::super::ChipFactory;
    use core::SystemModel;

    #[test]
    fn verify_mem_layout() {
        let config = Rc::new(Config::new(SystemModel::from("pal")));
        let factory = Box::new(ChipFactory::new(config.clone()));
        let mut c64 = C64::new(config.clone(), factory).unwrap();
        c64.reset(false);
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.borrow().read_debug(0xa000));
    }
}