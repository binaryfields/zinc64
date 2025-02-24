// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::{Bank, BaseAddr, ExpansionPort, Pla};
use crate::chip::cia::{self, Cia, CiaPins, PortInput};
use crate::chip::sid::{SamplingMethod, Sid};
use crate::chip::vic::{Vic, VicBus};
use crate::chip::{Bus, Ram, Rom};
use crate::config::Config;
use crate::peripheral::{Cartridge, Datassette, Joystick, Keyboard, Tape};
use crate::signal::{IoPort, IrqLine, Pin};

const CIA_2_INPUT: PortInput = PortInput { a: 0xff, b: 0xff };

// Spec: COMMODORE 64 MEMORY MAPS p. 263
// Design:
//   Inspired by UAE memory address64k/bank concepts. The memory configuration is based on
//   zones that can be mapped to different banks. The CPU port at 0x0000/0x0001 reconfigures
//   the memory layout.

struct Lines {
    ba: Pin,
    cia_1_flag: Pin,
    cia_2_flag: Pin,
    irq: IrqLine,
    nmi: IrqLine,
}

pub struct Board {
    // Memory
    basic: Rom,
    charset: Rom,
    color_ram: Ram,
    kernal: Rom,
    ram: Ram,
    cpu_port: IoPort,
    expansion_port: ExpansionPort,
    pla: Pla,
    // Chipset
    cia_1: Cia,
    cia_2: Cia,
    sid: Sid,
    vic: Vic,
    // Peripherals
    datassette: Datassette,
    joystick_1: Joystick,
    joystick_2: Joystick,
    keyboard: Keyboard,
    // I/O
    lines: Lines,
    // Runtime State
    cycles: u64,
    vsync: bool,
}

impl Board {
    pub fn new(config: &Config) -> Self {
        // I/O Lines
        let cpu_port = IoPort::new(0x00, 0xff);
        let lines = Lines {
            ba: Pin::new_high(),
            cia_1_flag: Pin::new_low(),
            cia_2_flag: Pin::new_low(),
            irq: IrqLine::new("irq"),
            nmi: IrqLine::new("nmi"),
        };

        // Memory
        let color_ram = Ram::new(config.model.color_ram);
        let ram = Ram::new(config.model.memory_size);
        let basic = Rom::new_with_data(&config.roms.basic, BaseAddr::Basic.addr());
        let charset = Rom::new_with_data(&config.roms.charset, 0);
        let kernal = Rom::new_with_data(&config.roms.kernal, BaseAddr::Kernal.addr());
        let expansion_port = ExpansionPort::new();
        let pla = Pla::new();

        // Chipset
        let cia_1 = Cia::new(cia::Mode::Cia1);
        let cia_2 = Cia::new(cia::Mode::Cia2);
        let mut sid = Sid::new(config.model.sid_model);
        sid.set_sampling_parameters(
            SamplingMethod::Fast,
            config.model.cpu_freq,
            config.sound.sample_rate,
        );
        sid.enable_filter(config.sound.sid_filters);
        let vic = Vic::new(config.model.vic_model, config.model.frame_buffer_size);

        // Peripherals
        let datassette = Datassette::new();
        let keyboard = Keyboard::new();

        Self {
            basic,
            charset,
            color_ram,
            kernal,
            ram,
            cpu_port,
            expansion_port,
            pla,
            cia_1,
            cia_2,
            sid,
            vic,
            datassette,
            joystick_1: Joystick::new(),
            joystick_2: Joystick::new(),
            keyboard,
            lines,
            cycles: 0,
            vsync: false,
        }
    }

    pub fn cia_1(&self) -> &Cia {
        &self.cia_1
    }

    pub fn cia_2(&self) -> &Cia {
        &self.cia_2
    }

    pub fn sid(&self) -> &Sid {
        &self.sid
    }

    pub fn vic(&self) -> &Vic {
        &self.vic
    }

    pub fn joystick_1(&self) -> &Joystick {
        &self.joystick_1
    }

    pub fn joystick_1_mut(&mut self) -> &mut Joystick {
        &mut self.joystick_1
    }

    pub fn joystick_2(&self) -> &Joystick {
        &self.joystick_2
    }

    pub fn joystick_2_mut(&mut self) -> &mut Joystick {
        &mut self.joystick_2
    }

    pub fn keyboard_mut(&mut self) -> &mut Keyboard {
        &mut self.keyboard
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    pub fn vsync(&self) -> bool {
        self.vsync
    }

    // ---- reset ----
    pub fn reset(&mut self, hard: bool) {
        self.cycles = 0;
        // Memory
        if hard {
            self.color_ram.fill(0x00);
        }
        self.cpu_port.set_value(0xff);
        self.switch_banks();
        self.lines.irq.reset();
        self.lines.nmi.reset();
        self.write(0x0000, 0b_0010_1111);
        self.write(0x0001, 0b_0001_1111);
        // Chipset
        self.cia_1.reset(&mut CiaPins {
            flag: &mut self.lines.cia_1_flag,
            irq: &mut self.lines.irq,
        });
        self.cia_2.reset(&mut CiaPins {
            flag: &mut self.lines.cia_2_flag,
            irq: &mut self.lines.nmi,
        });
        self.sid.reset(self.cycles);
        self.vic.reset();
        // I/O
        self.expansion_port.reset();
        self.switch_banks();
        // Peripherals
        self.datassette.reset(&mut self.cpu_port);
        self.joystick_1.reset();
        self.joystick_2.reset();
        self.keyboard.reset();
        // Runtime State
        self.vsync = false;
    }

    // ---- media ----
    pub fn load(&mut self, data: &[u8], offset: u16) {
        self.ram.load(data, offset);
    }

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.expansion_port.attach(cartridge);
    }

    pub fn detach_cartridge(&mut self) {
        self.expansion_port.detach();
    }

    pub fn attach_tape(&mut self, tape: Box<dyn Tape>) {
        self.datassette.attach(tape);
    }

    pub fn detach_tape(&mut self) {
        self.datassette.detach(&mut self.cpu_port);
    }

    pub fn is_tape_playing(&self) -> bool {
        self.datassette.is_playing(&self.cpu_port)
    }

    pub fn play_tape(&mut self) {
        self.datassette.play(&mut self.cpu_port);
    }

    pub fn stop_tape(&mut self) {
        self.datassette.stop(&mut self.cpu_port);
    }

    // ---- clock ----
    pub fn tick(&mut self) {
        let bank = self.vic_bank();
        let frame_done = self.vic.clock(&mut VicBus {
            ram: &self.ram,
            charset: &self.charset,
            color_ram: &self.color_ram,
            bank,
            ba: &mut self.lines.ba,
            irq: &mut self.lines.irq,
        });
        if frame_done {
            self.vsync = true;
        }
        self.cia_1.clock(&mut CiaPins {
            flag: &mut self.lines.cia_1_flag,
            irq: &mut self.lines.irq,
        });
        self.cia_2.clock(&mut CiaPins {
            flag: &mut self.lines.cia_2_flag,
            irq: &mut self.lines.nmi,
        });
        self.datassette
            .clock(&mut self.cpu_port, &mut self.lines.cia_1_flag);
        self.cycles = self.cycles.wrapping_add(1);
    }

    fn vic_bank(&self) -> u16 {
        let port_a = self.cia_2.port_a().get_value();
        ((!port_a & 0x03) as u16) << 14
    }

    // ---- vsync ----
    pub fn process_vsync(&mut self) {
        self.sid.process_vsync(self.cycles);
        self.cia_1.process_vsync();
        self.cia_2.process_vsync();
    }

    pub fn begin_run(&mut self) {
        self.vsync = false;
        self.sid.clear_samples();
    }

    // ---- i/o ----
    fn read_io(&mut self, address: u16) -> u8 {
        match address {
            0xd000..=0xd3ff => self.vic.read((address & 0x003f) as u8),
            0xd400..=0xd7ff => self.sid.read((address & 0x001f) as u8, self.cycles),
            0xd800..=0xdbff => self.color_ram.read(address - 0xd800),
            0xdc00..=0xdcff => self.read_cia_1((address & 0x000f) as u8),
            0xdd00..=0xddff => self.read_cia_2((address & 0x000f) as u8),
            0xde00..=0xdfff => self.expansion_port.read(address).unwrap_or(0),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }

    fn read_cia_1(&mut self, reg: u8) -> u8 {
        let input = self.cia_1_input();
        let value = self.cia_1.read(
            reg,
            input,
            &mut CiaPins {
                flag: &mut self.lines.cia_1_flag,
                irq: &mut self.lines.irq,
            },
        );
        value & self.joystick_lines(reg)
    }

    fn read_cia_2(&mut self, reg: u8) -> u8 {
        let input = CIA_2_INPUT;
        self.cia_2.read(
            reg,
            input,
            &mut CiaPins {
                flag: &mut self.lines.cia_2_flag,
                irq: &mut self.lines.nmi,
            },
        )
    }

    fn write_io(&mut self, address: u16, value: u8) {
        match address {
            0xd000..=0xd3ff => self
                .vic
                .write((address & 0x003f) as u8, value, &mut self.lines.irq),
            0xd400..=0xd7ff => self.sid.write((address & 0x001f) as u8, value, self.cycles),
            0xd800..=0xdbff => self.color_ram.write(address - 0xd800, value),
            0xdc00..=0xdcff => self.cia_1.write((address & 0x000f) as u8, value),
            0xdd00..=0xddff => self.cia_2.write((address & 0x000f) as u8, value),
            0xde00..=0xdfff => {
                if self.expansion_port.write(address, value) {
                    self.switch_banks();
                }
            }
            _ => panic!("invalid address 0x{:x}", address),
        }
    }

    // ---- inspection ----
    pub fn peek(&self, address: u16) -> u8 {
        match address {
            0x0000 => return self.cpu_port.get_direction(),
            0x0001 => return self.cpu_port.get_value() & 0x3f,
            _ => {}
        }
        match self.pla.map(address) {
            Bank::Ram => self.ram.read(address),
            Bank::Basic => self.basic.read(address),
            Bank::Charset => self.charset.read(address - BaseAddr::Charset.addr()),
            Bank::Kernal => self.kernal.read(address),
            Bank::RomL | Bank::RomH => self
                .expansion_port
                .peek(address)
                .unwrap_or(self.ram.read(address)),
            Bank::Io => self.peek_io(address),
            Bank::Disabled => 0,
        }
    }

    fn peek_io(&self, address: u16) -> u8 {
        match address {
            0xd000..=0xd3ff => self.vic.peek((address & 0x003f) as u8),
            0xd400..=0xd7ff => self.sid.peek((address & 0x001f) as u8),
            0xd800..=0xdbff => self.color_ram.read(address - 0xd800),
            0xdc00..=0xdcff => {
                let reg = (address & 0x000f) as u8;
                self.cia_1.peek(reg, self.cia_1_input()) & self.joystick_lines(reg)
            }
            0xdd00..=0xddff => self.cia_2.peek((address & 0x000f) as u8, CIA_2_INPUT),
            0xde00..=0xdfff => self.expansion_port.peek(address).unwrap_or(0),
            _ => panic!("invalid address 0x{:x}", address),
        }
    }

    // ---- shared ----
    fn switch_banks(&mut self) {
        let mode = self.cpu_port.get_value() & 0x07 | self.expansion_port.io_lines() & 0x18;
        self.pla.switch_banks(mode);
    }

    fn cia_1_input(&self) -> PortInput {
        PortInput {
            a: self.keyboard.scan_port_a(self.cia_1.port_b().get_value()),
            b: self.keyboard.scan_port_b(self.cia_1.port_a().get_value()),
        }
    }

    fn joystick_lines(&self, reg: u8) -> u8 {
        match reg {
            cia::reg::PRA => self.joystick_2.port_lines(),
            cia::reg::PRB => self.joystick_1.port_lines(),
            _ => 0xff,
        }
    }
}

impl Bus for Board {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            0x0000 => return self.cpu_port.get_direction(),
            0x0001 => return self.cpu_port.get_value() & 0x3f,
            _ => {}
        }
        let bank = self.pla.map(address);
        match bank {
            Bank::Ram => self.ram.read(address),
            Bank::Basic => self.basic.read(address),
            Bank::Charset => self.charset.read(address - BaseAddr::Charset.addr()),
            Bank::Kernal => self.kernal.read(address),
            Bank::RomL => self
                .expansion_port
                .read(address)
                .unwrap_or(self.ram.read(address)),
            Bank::RomH => self
                .expansion_port
                .read(address)
                .unwrap_or(self.ram.read(address)),
            Bank::Io => self.read_io(address),
            Bank::Disabled => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000 => {
                self.cpu_port.set_direction(value);
                self.switch_banks();
            }
            0x0001 => {
                self.cpu_port.set_value(value);
                self.switch_banks();
            }
            _ => {}
        }
        let bank = self.pla.map(address);
        match bank {
            Bank::Ram => self.ram.write(address, value),
            Bank::Basic => self.ram.write(address, value),
            Bank::Charset => self.ram.write(address, value),
            Bank::Kernal => self.ram.write(address, value),
            Bank::RomL => self.ram.write(address, value),
            Bank::RomH => self.ram.write(address, value),
            Bank::Io => self.write_io(address, value),
            Bank::Disabled => {}
        }
    }

    fn rdy(&self) -> bool {
        self.lines.ba.is_high()
    }

    fn irq(&self) -> bool {
        self.lines.irq.is_low()
    }

    fn nmi(&self) -> bool {
        self.lines.nmi.is_low()
    }

    fn ack_nmi(&mut self) {
        self.lines.nmi.reset();
    }
}
