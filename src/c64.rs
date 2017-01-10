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

use cpu::{Cpu, CpuIo};
use config::Config;
use device::{Cartridge, Joystick, Keyboard};
use device::joystick;
use mem::{Addressable, BaseAddr, ColorRam, DeviceIo, Memory};
use io::{Cia, CiaIo, ExpansionPort, ExpansionPortIo};
use io::cia;
use loader::Autostart;
use video::{RenderTarget, Vic};

// Design:
//   C64 represents the machine itself and all of its components. Connections between different
//   components are managed as component dependencies.

// TODO c64: move ioport configuration to reset
// TODO c64: update test cases to use loader

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
    vic: Rc<RefCell<Vic>>,
    // sid: Rc<RefCell<Sid>>,
    // I/O
    expansion_port: Rc<RefCell<ExpansionPort>>,
    // Peripherals
    joystick1: Option<Rc<RefCell<Joystick>>>,
    joystick2: Option<Rc<RefCell<Joystick>>>,
    keyboard: Rc<RefCell<Keyboard>>,
    rt: Rc<RefCell<RenderTarget>>,
    // TBD
    autostart: Option<Autostart>,
    breakpoints: Vec<u16>,
    // Runtime State
    last_pc: u16,
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
        // Peripherals
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
        let rt = Rc::new(RefCell::new(
            RenderTarget::new(config.visible_size)
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
        let vic = Rc::new(RefCell::new(
            Vic::new(config.clone(),
                     cpu.clone(),
                     mem.clone(),
                     color_ram.clone(),
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
                          vic.clone())
        ));
        mem.borrow_mut().set_cia2(cia2.clone());
        mem.borrow_mut().set_device_io(device_io.clone());
        mem.borrow_mut().set_expansion_port(expansion_port.clone());
        // Initialization
        cpu.borrow_mut().write(BaseAddr::IoPortDdr.addr(), 0x2f);
        cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 31);
        Ok(
            C64 {
                config: config,
                mem: mem.clone(),
                color_ram: color_ram.clone(),
                cpu: cpu.clone(),
                vic: vic.clone(),
                cia1: cia1.clone(),
                cia2: cia2.clone(),
                expansion_port: expansion_port.clone(),
                joystick1: joystick1,
                joystick2: joystick2,
                keyboard: keyboard.clone(),
                rt: rt.clone(),
                autostart: None,
                breakpoints: Vec::new(),
                last_pc: 0,
            }
        )
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }
    pub fn get_cpu(&self) -> Rc<RefCell<Cpu>> {
        self.cpu.clone()
    }
    pub fn get_cycles(&self) -> u32 {
        self.cpu.borrow().get_cycles()
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
    pub fn get_memory(&self) -> Rc<RefCell<Memory>> {
        self.mem.clone()
    }
    pub fn get_render_target(&self) -> Rc<RefCell<RenderTarget>> {
        self.rt.clone()
    }
    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.borrow().get_pc()
    }
    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    pub fn load(&mut self, data: &Vec<u8>, offset: u16) {
        let mut mem = self.mem.borrow_mut();
        let mut address = offset;
        for byte in data {
            mem.write_ram(address, *byte);
            address = address.wrapping_add(1);
        }
    }

    pub fn reset(&mut self) {
        info!(target: "c64", "Resetting system");
        self.cpu.borrow_mut().reset();
        self.last_pc = 0;
        //self.expansion_port.borrow_mut().reset();
    }

    pub fn run_frame(&mut self) -> bool {
        let frame_cycles = (self.config.cpu_frequency as f64
            / self.config.refresh_rate) as u64;
        let mut last_pc = 0x0000;
        for i in 0..frame_cycles {
            self.step();
            let pc = self.cpu.borrow().get_pc();
            if self.check_breakpoints() {
                println!("trap at 0x{:x}", pc);
                return false;
            }
            if pc == last_pc {
                println!("infinite loop at 0x{:x}", pc);
                return false;
            }
            last_pc = pc;
        }
        true
    }

    pub fn step(&mut self) -> u32 {
        self.last_pc = self.cpu.borrow().get_pc();
        let prev_cycles = self.cpu.borrow().get_cycles();
        self.cpu.borrow_mut().execute();
        let cycles = self.cpu.borrow().get_cycles() - prev_cycles;
        if self.autostart.is_some() {
            if self.cpu.borrow().get_pc() == 0xa65c {
                if let Some(mut autostart) = self.autostart.take() {
                    autostart.execute(self);
                }
            }
        }
        for i in 0..(cycles + 1) {
            self.cia1.borrow_mut().step();
            self.cia2.borrow_mut().step();
            self.vic.borrow_mut().step();
        }
        cycles
    }

    // -- Cartridge Ops

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.expansion_port.borrow_mut().attach(cartridge);
    }

    pub fn detach_cartridge(&mut self) {
        self.expansion_port.borrow_mut().detach();
        self.reset();
    }

    // -- Debug Ops

    pub fn add_breakpoint(&mut self, breakpoint: u16) {
        self.breakpoints.push(breakpoint);
    }

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
