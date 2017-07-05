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

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpu::{Cpu, CpuIo};
use config::Config;
use device::{Cartridge, Datassette, Joystick, Keyboard, Tape};
use device::joystick;
use mem::{ColorRam, DeviceIo, Memory};
use io::{Cia, CiaIo, ExpansionPort, ExpansionPortIo};
use io::cia;
use loader::Autostart;
use sound::{Sid, SoundBuffer};
use time;
use video::{RenderTarget, Vic};

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

#[allow(dead_code)]
pub struct C64 {
    // Deps
    config: Config,
    // Chipset
    cpu: Rc<RefCell<Cpu>>,
    mem: Rc<RefCell<Memory>>,
    color_ram: Rc<RefCell<ColorRam>>,
    cia1: Rc<RefCell<Cia>>,
    cia2: Rc<RefCell<Cia>>,
    sid: Rc<RefCell<Sid>>,
    vic: Rc<RefCell<Vic>>,
    // I/O
    expansion_port: Rc<RefCell<ExpansionPort>>,
    // Peripherals
    datassette: Rc<RefCell<Datassette>>,
    joystick1: Option<Rc<RefCell<Joystick>>>,
    joystick2: Option<Rc<RefCell<Joystick>>>,
    keyboard: Rc<RefCell<Keyboard>>,
    rt: Rc<RefCell<RenderTarget>>,
    sound_buffer: Arc<Mutex<SoundBuffer>>,
    // Configuration
    autostart: Option<Autostart>,
    breakpoints: Vec<u16>,
    speed: u8,
    warp_mode: bool,
    // Runtime State
    cycles: u64,
    frames: u32,
    last_pc: u16,
    next_frame_ns: u64,
}

impl C64 {
    pub fn new(config: Config) -> Result<C64, io::Error> {
        info!(target: "c64", "Initializing system");
        // I/O Lines
        let cia1_io = Rc::new(RefCell::new(
            CiaIo::new()
        ));
        let cia2_io = Rc::new(RefCell::new(
            CiaIo::new()
        ));
        let cpu_io = Rc::new(RefCell::new(
            CpuIo::new()
        ));
        let expansion_port_io = Rc::new(RefCell::new(
            ExpansionPortIo::new()
        ));
        let rt = Rc::new(RefCell::new(
            RenderTarget::new(config.screen_size)
        ));
        let sound_buffer = Arc::new(Mutex::new(
            SoundBuffer::new(4096) // FIXME magic value
        ));
        // Peripherals
        let datassette = Rc::new(RefCell::new(
            Datassette::new(cia1_io.clone(), cpu_io.clone())
        ));
        let joystick1 = if config.joystick1 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(
                Joystick::new(config.joystick1, 3200)))
            )
        } else {
            None
        };
        let joystick2 = if config.joystick2 != joystick::Mode::None {
            Some(Rc::new(RefCell::new(
                Joystick::new(config.joystick2, 3200)))
            )
        } else {
            None
        };
        let keyboard = Rc::new(RefCell::new(
            Keyboard::new()
        ));
        // Chipset
        let mem = Rc::new(RefCell::new(
            Memory::new(0x10000,
                        cpu_io.clone(),
                        expansion_port_io.clone())?
        ));
        let color_ram = Rc::new(RefCell::new(
            ColorRam::new(1024)
        ));
        let cpu = Rc::new(RefCell::new(
            Cpu::new(cpu_io.clone(), mem.clone())
        ));
        let cia1 = Rc::new(RefCell::new(
            Cia::new(cia::Mode::Cia1,
                     cia1_io.clone(),
                     cpu_io.clone(),
                     joystick1.clone(),
                     joystick2.clone(),
                     keyboard.clone())
        ));
        let cia2 = Rc::new(RefCell::new(
            Cia::new(cia::Mode::Cia2,
                     cia2_io.clone(),
                     cpu_io.clone(),
                     joystick1.clone(),
                     joystick2.clone(),
                     keyboard.clone())
        ));
        let sid = Rc::new(RefCell::new(
            Sid::new(sound_buffer.clone())
        ));
        let vic = Rc::new(RefCell::new(
            Vic::new(config.clone(),
                     color_ram.clone(),
                     cpu_io.clone(),
                     mem.clone(),
                     rt.clone())
        ));
        // I/O
        let expansion_port = Rc::new(RefCell::new(
            ExpansionPort::new(expansion_port_io.clone(), mem.clone())
        ));
        let device_io = Rc::new(RefCell::new(
            DeviceIo::new(cia1.clone(),
                          cia2.clone(),
                          color_ram.clone(),
                          expansion_port.clone(),
                          sid.clone(),
                          vic.clone())
        ));
        mem.borrow_mut().set_cia2(cia2.clone());
        mem.borrow_mut().set_device_io(device_io.clone());
        mem.borrow_mut().set_expansion_port(expansion_port.clone());
        Ok(
            C64 {
                config: config,
                mem: mem.clone(),
                color_ram: color_ram.clone(),
                cpu: cpu.clone(),
                sid: sid.clone(),
                vic: vic.clone(),
                cia1: cia1.clone(),
                cia2: cia2.clone(),
                expansion_port: expansion_port.clone(),
                datassette: datassette,
                joystick1: joystick1,
                joystick2: joystick2,
                keyboard: keyboard.clone(),
                rt: rt.clone(),
                sound_buffer: sound_buffer.clone(),
                autostart: None,
                breakpoints: Vec::new(),
                speed: 100,
                warp_mode: false,
                cycles: 0,
                frames: 0,
                last_pc: 0,
                next_frame_ns: 0,
            }
        )
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> {
        self.cpu.clone()
    }

    pub fn get_cycles(&self) -> u64 {
        self.cycles
    }

    pub fn get_datasette(&self) -> Rc<RefCell<Datassette>> {
        self.datassette.clone()
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

    pub fn get_render_target(&self) -> Rc<RefCell<RenderTarget>> {
        self.rt.clone()
    }

    pub fn get_sound_buffer(&self) -> Arc<Mutex<SoundBuffer>> {
        self.sound_buffer.clone()
    }

    pub fn get_warp_mode(&self) -> bool {
        self.warp_mode
    }

    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.borrow().get_pc()
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    pub fn set_speed(&mut self, value: u8) {
        self.speed = value;
    }

    pub fn set_warp_mode(&mut self, enabled: bool) {
        self.warp_mode = enabled;
    }

    pub fn load(&mut self, data: &Vec<u8>, offset: u16) {
        let mut mem = self.mem.borrow_mut();
        let mut address = offset;
        for byte in data {
            mem.write_ram(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    pub fn reset(&mut self, hard: bool) {
        info!(target: "c64", "Resetting system");
        if hard {
            self.mem.borrow_mut().reset();
            self.color_ram.borrow_mut().reset();
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
        self.rt.borrow_mut().reset();
        self.sound_buffer.lock().unwrap().clear();
        // Runtime State
        self.cycles = 0;
        self.frames = 0;
        self.last_pc = 0;
        self.next_frame_ns = 0;
    }

    pub fn run_frame(&mut self, overflow_cycles: u16) -> u16 {
        let mut cycles = (self.config.frame_cycles - overflow_cycles) as i16;
        let mut elapsed = 0;
        while cycles > 0 {
            let completed = self.step();
            cycles -= completed as i16;
            elapsed += completed as u32;
        }
        self.frames += 1;
        if self.frames % (self.config.refresh_rate as u32 / 10) == 0 {
            self.cia1.borrow_mut().tod_tick();
            self.cia2.borrow_mut().tod_tick();
        }
        self.sid.borrow_mut().step_delta(elapsed);
        if !self.warp_mode {
            self.sync_frame();
        }
        (-cycles) as u16
    }

    #[allow(unused_variables)]
    pub fn step(&mut self) -> u8 {
        self.last_pc = self.cpu.borrow().get_pc();
        let cycles = self.cpu.borrow_mut().execute();
        for i in 0..cycles {
            self.vic.borrow_mut().step();
            self.cia1.borrow_mut().step();
            self.cia2.borrow_mut().step();
            self.datassette.borrow_mut().step();
        }
        if self.autostart.is_some() {
            if self.cpu.borrow().get_pc() == 0xa65c {
                if let Some(mut autostart) = self.autostart.take() {
                    autostart.execute(self);
                }
            }
        }
        self.cycles += cycles as u64;
        cycles
    }

    fn sync_frame(&mut self) {
        let frame_duration_scaled_ns = self.config.frame_duration_ns * 100 / self.speed as u32;
        let time_ns = time::precise_time_ns();
        let wait_ns = if self.next_frame_ns > time_ns {
            (self.next_frame_ns - time_ns) as u32
        } else {
            0
        };
        if wait_ns > 0 && wait_ns <= frame_duration_scaled_ns {
            thread::sleep(Duration::new(0, wait_ns));
        }
        self.next_frame_ns = time::precise_time_ns() + frame_duration_scaled_ns as u64;
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

    // -- Debug Ops

    pub fn add_breakpoint(&mut self, breakpoint: u16) {
        self.breakpoints.push(breakpoint);
    }

    #[allow(dead_code)]
    pub fn check_breakpoints(&self) -> bool {
        let pc = self.cpu.borrow().get_pc();
        !self.breakpoints.is_empty() && self.breakpoints.contains(&pc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use config::Config;
    use mem::BaseAddr;

    //#[test]
    fn cpu_test() {
        let mut c64 = C64::new(Config::pal()).unwrap();
        let cpu = c64.get_cpu();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x00);
        c64.load(Path::new("rom/6502_functional_test.bin"), 0x0400).unwrap();
        cpu.borrow_mut().set_pc(0x0400);
        let mut last_pc = 0x0000;
        loop {
            c64.step();
            if cpu.borrow_mut().get_pc() == 0x3463 {
                break;
            }
            if cpu.borrow_mut().get_pc() == last_pc {
                panic!("trap at 0x{:x}", cpu.borrow_mut().get_pc());
            }
            last_pc = cpu.borrow_mut().get_pc();
        }
    }

    #[test]
    fn mem_layout() {
        let c64 = C64::new(Config::pal()).unwrap();
        let cpu = c64.get_cpu();
        assert_eq!(0x94, cpu.borrow().read(BaseAddr::Basic.addr()));
    }

    #[test]
    fn keyboard_read() {
        /*
        .c000  78         sei
        .c001  a9 ff      lda #$ff
        .c003  8d 02 dc   sta $dc02
        .c006  a9 00      lda #$00
        .c008  8d 03 dc   sta $dc03
        .c00b  a9 fd      lda #$fd
        .c00d  8d 00 dc   sta $dc00
        .c010  ad 01 dc   lda $dc01
        .c013  29 20      and #$20
        .c015  d0 f9      bne $c010
        .c017  58         cli
        */
        let code = [
            0x78u8,
            0xa9, 0xff,
            0x8d, 0x02, 0xdc,
            0xa9, 0x00,
            0x8d, 0x03, 0xdc,
            0xa9, 0xfd,
            0x8d, 0x00, 0xdc,
            0xad, 0x01, 0xdc,
            0x29, 0x20,
            0xd0, 0xf9,
            0x58
        ];
        let mut c64 = C64::new(Config::pal()).unwrap();
        let cpu = c64.get_cpu();
        let keyboard = c64.get_keyboard();
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x00);
        c64.load(&code.to_vec(), 0xc000);
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0x06);
        keyboard.borrow_mut().set_row(1, !(1 << 5));
        cpu.borrow_mut().set_pc(0xc000);
        let mut last_pc = 0x0000;
        let mut branch_count = 0;
        loop {
            c64.step();
            if cpu.borrow().get_pc() == 0xc018 {
                break;
            }
            if cpu.borrow().get_pc() == 0xc015 {
                branch_count += 1;
                if branch_count > 1 {
                    panic!("trap at 0x{:x}", cpu.borrow_mut().get_pc());
                }
            }
            last_pc = cpu.borrow_mut().get_pc();
        }
    }
}
