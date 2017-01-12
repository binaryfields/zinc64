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
use std::rc::Rc;

use cpu::CpuIo;
use cpu::interrupt;
use device::{Joystick, Keyboard};
use device::joystick;
use log::LogLevel;
use util::bit;

// Spec: 6526 COMPLEX INTERFACE ADAPTER (CIA) Datasheet
// Spec: https://www.c64-wiki.com/index.php/CIA
// http://www.unusedino.de/ec64/technical/project64/mapping_c64.html

// TODO cia: add timer output logic
// TODO cia: add rtc
// TODO cia: test cia read regs 0d-of
// TODO cia: test timers

pub struct CiaIo {
    pub cnt: bool,
}

impl CiaIo {
    pub fn new() -> CiaIo {
        CiaIo {
            cnt: false,
        }
    }

    pub fn reset(&mut self) {
        self.cnt = true;
    }
}

#[derive(PartialEq)]
pub enum Mode {
    Cia1,
    Cia2,
}

struct Port {
    latch: u8,
    value: u8,
    direction: u8,
}

impl Port {
    pub fn new(direction: u8) -> Port {
        Port {
            latch: 0,
            value: 0,
            direction: direction,
        }
    }

    pub fn set_value(&mut self, value: u8) {
        self.latch = value;
        // set input pins to 1
        self.value = self.latch | !self.direction;
    }

    pub fn set_direction(&mut self, direction: u8) {
        self.direction = direction;
        // set input pins to 1
        self.value = self.latch | !self.direction;
    }

    pub fn reset(&mut self) {
        self.direction = 0x00;
        self.latch = 0x00;
        self.set_value(0x00);
    }
}

#[derive(Copy, Clone)]
enum Reg {
    PRA = 0x00,
    PRB = 0x01,
    DDRA = 0x02,
    DDRB = 0x03,
    TALO = 0x04,
    TAHI = 0x05,
    TBLO = 0x06,
    TBHI = 0x07,
    TODTS = 0x08,
    TODSEC = 0x09,
    TODMIN = 0x0a,
    TODHR = 0x0b,
    SDR = 0x0c,
    ICR = 0x0d,
    CRA = 0x0e,
    CRB = 0x0f,
}

impl Reg {
    pub fn from(reg: u8) -> Reg {
        match reg {
            0x00 => Reg::PRA,
            0x01 => Reg::PRB,
            0x02 => Reg::DDRA,
            0x03 => Reg::DDRB,
            0x04 => Reg::TALO,
            0x05 => Reg::TAHI,
            0x06 => Reg::TBLO,
            0x07 => Reg::TBHI,
            0x08 => Reg::TODTS,
            0x09 => Reg::TODSEC,
            0x0a => Reg::TODMIN,
            0x0b => Reg::TODHR,
            0x0c => Reg::SDR,
            0x0d => Reg::ICR,
            0x0e => Reg::CRA,
            0x0f => Reg::CRB,
            _ => panic!("invalid reg {}", reg)
        }
    }

    #[allow(dead_code)]
    pub fn addr(&self) -> u8 {
        *self as u8
    }
}

struct Rtc {}

#[derive(Debug, PartialEq)]
pub enum TimerMode {
    OneShot,
    Continuous,
}

#[derive(Debug, PartialEq)]
pub enum TimerInput {
    SystemClock = 0,
    External = 1,
    TimerA = 2,
    TimerAWithCNT = 3,
}

#[derive(Debug, PartialEq)]
enum TimerOutput {
    Toggle,
    Pulse,
}

struct Timer {
    enabled: bool,
    mode: TimerMode,
    input: TimerInput,
    output: TimerOutput,
    output_enabled: bool,
    latch: u16,
    value: u16,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            enabled: false,
            mode: TimerMode::OneShot,
            input: TimerInput::SystemClock,
            output: TimerOutput::Pulse,
            output_enabled: false,
            latch: 0,
            value: 0,
        }
    }

    pub fn reset(&mut self) {
        self.enabled = false;
        self.mode = TimerMode::OneShot;
        self.input = TimerInput::SystemClock;
        self.output = TimerOutput::Pulse;
        self.output_enabled = false;
        self.latch = 0xffff;
        self.value = 0x0000;
    }

    fn reload(&mut self) {
        /*
        A control bit selects either timer mode. In one-shot
        mode, the timer will count down from the latched value
        to zero, generate an interrupt, reload the latched value,
        then stop. In continuous mode, the timer will count from
        the latched value to zero, generate an interrupt, reload
        the latched value and repeatthe procedure continuously
        */
        match self.mode {
            TimerMode::Continuous => {
                self.value = self.latch;
            },
            TimerMode::OneShot => {
                self.value = self.latch;
                self.enabled = false;
            }
        }
    }

    pub fn update(&mut self, pulse: u16) -> bool {
        if self.enabled {
            if self.value == 0 {
                self.reload();
                true
            } else {
                self.value -= pulse;
                false
            }
        } else {
            false
        }
    }
}

#[allow(dead_code)]
pub struct Cia {
    // Dependencies
    cpu_io: Rc<RefCell<CpuIo>>,
    joystick1: Option<Rc<RefCell<Joystick>>>,
    joystick2: Option<Rc<RefCell<Joystick>>>,
    keyboard: Rc<RefCell<Keyboard>>,
    mode: Mode,
    // Functional Units
    port_a: Port,
    port_b: Port,
    rtc: Rtc,
    timer_a: Timer,
    timer_b: Timer,
    // Interrupts
    int_data: u8,
    int_mask: u8,
    int_triggered: bool,
    // I/O Lines
    cia_io: Rc<RefCell<CiaIo>>,
    cnt_last: bool,
}

impl Cia {
    pub fn new(mode: Mode,
               cia_io: Rc<RefCell<CiaIo>>,
               cpu_io: Rc<RefCell<CpuIo>>,
               joystick1: Option<Rc<RefCell<Joystick>>>,
               joystick2: Option<Rc<RefCell<Joystick>>>,
               keyboard: Rc<RefCell<Keyboard>>) -> Cia {
        Cia {
            cpu_io: cpu_io,
            joystick1: joystick1,
            joystick2: joystick2,
            keyboard: keyboard,
            mode: mode,
            port_a: Port::new(0x00),
            port_b: Port::new(0x00),
            rtc: Rtc {},
            timer_a: Timer::new(),
            timer_b: Timer::new(),
            int_data: 0,
            int_mask: 0,
            int_triggered: false,
            cia_io: cia_io,
            cnt_last: false,
        }
    }

    pub fn reset(&mut self) {
        /*
        A low on the RES pin resets all internal registers.The
        port pins are set as inputs and port registers to zero
        (although a read of the ports will return all highs
        because of passive pullups).The timer control registers
        are set to zero and the timer latches to all ones. All other
        registers are reset to zero.
        */
        self.port_a.reset();
        self.port_b.reset();
        self.timer_a.reset();
        self.timer_b.reset();
        self.int_data = 0x00;
        self.int_mask = 0x00;
        self.int_triggered = false;
        self.cia_io.borrow_mut().reset();
        self.cnt_last = false;
    }

    pub fn step(&mut self) {
        // Process timers
        let timer_a_output = if self.timer_a.enabled {
            let pulse = match self.timer_a.input {
                TimerInput::SystemClock => 1,
                TimerInput::External => if !self.cnt_last && self.cia_io.borrow().cnt { 1 } else { 0 },
                _ => panic!("invalid input source {:?}", self.timer_a.input),
            };
            self.timer_a.update(pulse)
        } else {
            false
        };
        let timer_b_output = if self.timer_b.enabled {
            let pulse = match self.timer_b.input {
                TimerInput::SystemClock => 1,
                TimerInput::External => if !self.cnt_last && self.cia_io.borrow().cnt { 1 } else { 0 },
                TimerInput::TimerA => if timer_a_output { 1 } else { 0 },
                TimerInput::TimerAWithCNT => if timer_a_output && self.cia_io.borrow().cnt { 1 } else { 0 },
            };
            self.timer_b.update(pulse)
        } else {
            false
        };
        // Process interrupts
        /*
        Any interrupt will set the corresponding bit in the DATA
        register. Any interrupt which is enabled by the MASK
        register will set the IR bit (MSB) of the DATA register
        and bring the IRQ pin low.
        */
        if timer_a_output {
            self.int_data |= 1 << 0;
        }
        if timer_b_output {
            self.int_data |= 1 << 1;
        }
        if (self.int_mask & self.int_data) != 0 && !self.int_triggered {
            self.trigger_interrupt();
        }
        // Update internal state
        self.cnt_last = self.cia_io.borrow().cnt;
    }

    // -- Internal Ops

    fn read_cia1_port_a(&self) -> u8 {
        let joystick = self.scan_joystick(&self.joystick2);
        self.port_a.value & joystick
    }

    fn read_cia1_port_b(&self) -> u8 {
        // let timer_a_out = 1u8 << 6;
        // let timer_b_out = 1u8 << 7;
        let keyboard = match self.port_a.value {
            0x00 => 0x00,
            0xff => 0xff,
            _ => self.scan_keyboard(!self.port_a.value),
        };
        let joystick = self.scan_joystick(&self.joystick1);
        self.port_b.value & keyboard & joystick
    }

    fn read_cia2_port_a(&self) -> u8 {
        // iec inputs
        self.port_a.value
    }

    fn read_cia2_port_b(&self) -> u8 {
        self.port_b.value
    }

    fn scan_joystick(&self, joystick: &Option<Rc<RefCell<Joystick>>>) -> u8 {
        if let Some(ref joystick) = *joystick {
            let joy = joystick.borrow();
            let joy_up = bit::bit_set(0, joy.get_y_axis() == joystick::AxisMotion::Positive);
            let joy_down = bit::bit_set(1, joy.get_y_axis() == joystick::AxisMotion::Negative);
            let joy_left = bit::bit_set(2, joy.get_x_axis() == joystick::AxisMotion::Negative);
            let joy_right = bit::bit_set(3, joy.get_x_axis() == joystick::AxisMotion::Positive);
            let joy_fire = bit::bit_set(4, joy.get_button());
            !(joy_left | joy_right | joy_up | joy_down | joy_fire)
        } else {
            0xff
        }
    }

    fn scan_keyboard(&self, columns: u8) -> u8 {
        let mut result = 0;
        for i in 0..8 {
            if bit::bit_test(columns, i) {
                result |= self.keyboard.borrow().get_row(i);
            }
        }
        result
    }

    // -- Interrupt Ops

    fn clear_interrupt(&mut self) {
        match self.mode {
            Mode::Cia1 => self.cpu_io.borrow_mut().irq.clear(interrupt::Source::Cia),
            Mode::Cia2 => self.cpu_io.borrow_mut().nmi.clear(interrupt::Source::Cia),
        }
        self.int_triggered = false;

    }

    fn trigger_interrupt(&mut self) {
        match self.mode {
            Mode::Cia1 => self.cpu_io.borrow_mut().irq.set(interrupt::Source::Cia),
            Mode::Cia2 => self.cpu_io.borrow_mut().nmi.set(interrupt::Source::Cia),
        }
        self.int_triggered = true;
    }

    // -- Device I/O

    #[allow(dead_code)]
    pub fn read(&mut self, reg: u8) -> u8 {
        let value = match Reg::from(reg) {
            Reg::PRA => {
                match self.mode {
                    Mode::Cia1 => self.read_cia1_port_a(),
                    Mode::Cia2 => self.read_cia2_port_a(),
                }
            },
            Reg::PRB => {
                match self.mode {
                    Mode::Cia1 => self.read_cia1_port_b(),
                    Mode::Cia2 => self.read_cia2_port_b(),
                }
            },
            Reg::DDRA => self.port_a.direction,
            Reg::DDRB => self.port_b.direction,
            Reg::TALO => (self.timer_a.value & 0xff) as u8,
            Reg::TAHI => (self.timer_a.value >> 8) as u8,
            Reg::TBLO => (self.timer_b.value & 0xff) as u8,
            Reg::TBHI => (self.timer_b.value >> 8) as u8,
            Reg::TODTS => 0,
            Reg::TODSEC => 0,
            Reg::TODMIN => 0,
            Reg::TODHR => 0,
            Reg::SDR => 0,
            Reg::ICR => {
                /*
                In a multi-chip system, the IR bit can be polled to detect which chip has generated
                an interrupt request. The interrupt DATA register
                is cleared and the IRQ line returns high following a
                read of the DATA register.
                */
                let result = bit::bit_update(self.int_data, 7, (self.int_mask & self.int_data) != 0);
                self.int_data = 0;
                self.clear_interrupt();
                result
            },
            Reg::CRA => {
                let timer = &self.timer_a;
                let timer_enabled = bit::bit_set(0, timer.enabled);
                let timer_output = bit::bit_set(1, timer.output_enabled);
                let timer_output_mode = bit::bit_set(2, timer.output == TimerOutput::Toggle);
                let timer_mode = bit::bit_set(3, timer.mode == TimerMode::OneShot);
                let timer_input = match timer.input {
                    TimerInput::SystemClock => 0,
                    TimerInput::External => bit::bit_set(5, true),
                    _ => panic!("invalid timer input"),
                };
                timer_enabled | timer_output | timer_output_mode | timer_mode | timer_input
            }
            Reg::CRB => {
                let timer = &self.timer_b;
                let timer_enabled = bit::bit_set(0, timer.enabled);
                let timer_output = bit::bit_set(1, timer.output_enabled);
                let timer_output_mode = bit::bit_set(2, timer.output == TimerOutput::Toggle);
                let timer_mode = bit::bit_set(3, timer.mode == TimerMode::OneShot);
                let timer_input = match timer.input {
                    TimerInput::SystemClock => 0,
                    TimerInput::External => bit::bit_set(5, true),
                    TimerInput::TimerA => bit::bit_set(6, true),
                    TimerInput::TimerAWithCNT => bit::bit_set(6, true) | bit::bit_set(7, true),
                };
                timer_enabled | timer_output | timer_output_mode | timer_mode | timer_input
            }
        };
        if log_enabled!(LogLevel::Trace) {
          trace!(target: "cia::reg", "Read 0x{:02x} = 0x{:02x}", reg, value);
        }
        value
    }

    #[allow(dead_code, unused_variables)]
    pub fn write(&mut self, reg: u8, value: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cia::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        match Reg::from(reg) {
            Reg::PRA => {
                self.port_a.set_value(value);
            },
            Reg::PRB => {
                self.port_b.set_value(value);
            },
            Reg::DDRA => {
                self.port_a.set_direction(value);
            },
            Reg::DDRB => {
                self.port_b.set_direction(value);
            },
            Reg::TALO => {
                let value = (self.timer_a.latch & 0xff00) | (value as u16);
                self.timer_a.latch = value;
            },
            Reg::TAHI => {
                let value = (self.timer_a.latch & 0x00ff) | ((value as u16) << 8);
                self.timer_a.latch = value;
                if !self.timer_a.enabled {
                    self.timer_a.value = value;
                }
            },
            Reg::TBLO => {
                let value = (self.timer_b.latch & 0xff00) | (value as u16);
                self.timer_b.latch = value;
            },
            Reg::TBHI => {
                let value = (self.timer_b.latch & 0x00ff) | ((value as u16) << 8);
                self.timer_b.latch = value;
                if !self.timer_b.enabled {
                    self.timer_b.value = value;
                }
            },
            Reg::TODTS => {},
            Reg::TODSEC => {},
            Reg::TODMIN => {},
            Reg::TODHR => {},
            Reg::SDR => {},
            Reg::ICR => {
                /*
                The MASK register provides convenient control of
                individual mask bits. When writing to the MASK register,
                if bit 7 (SET/CLEAR) of the data written is a ZERO,
                any mask bit written with a one will be cleared, while
                those mask bits written with a zero will be unaffected. If
                bit 7 of the data written is a ONE, any mask bit written
                with a one will be set, while those mask bits written with
                a zero will be unaffected. In order for an interrupt flag to
                set IR and generate an Interrupt Request, the corresponding
                MASK bit must be set.
s                */
                if bit::bit_test(value, 7) {
                    self.int_mask |= value & 0x1f;
                } else {
                    self.int_mask &= !(value & 0x1f);
                }
                if (self.int_mask & self.int_data) != 0 && !self.int_triggered {
                    self.trigger_interrupt();
                }
            },
            Reg::CRA => {
                self.timer_a.enabled = bit::bit_test(value, 0);
                self.timer_a.mode = if bit::bit_test(value, 3) {
                    TimerMode::OneShot
                } else {
                    TimerMode::Continuous
                };
                if bit::bit_test(value, 4) {
                    self.timer_a.value = self.timer_a.latch;
                }
                self.timer_a.input = if bit::bit_test(value, 5) {
                    TimerInput::External
                } else {
                    TimerInput::SystemClock
                };
            },
            Reg::CRB => {
                self.timer_b.enabled = bit::bit_test(value, 0);
                self.timer_b.mode = if bit::bit_test(value, 3) {
                    TimerMode::OneShot
                } else {
                    TimerMode::Continuous
                };
                if bit::bit_test(value, 4) {
                    self.timer_b.value = self.timer_b.latch;
                }
                let input = (value & 0x60) >> 5;
                self.timer_b.input = match input {
                    0 => TimerInput::SystemClock,
                    1 => TimerInput::External,
                    2 => TimerInput::TimerA,
                    3 => TimerInput::TimerAWithCNT,
                    _ => panic!("invalid timer input"),
                };
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpu::Cpu;
    use io::keyboard::Keyboard;
    use mem::Memory;
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;
    use std::result::Result;

    fn setup_cpu() -> Result<Cpu, io::Error> {
        let mem = Rc::new(RefCell::new(
            Memory::new()?
        ));
        Ok(Cpu::new(mem))
    }

    fn setup_cia() -> Result<Cia, io::Error> {
        let cpu = setup_cpu()?;
        let keyboard = Keyboard::new();
        let mut cia = Cia::new(Mode::Cia1,
                               Rc::new(RefCell::new(cpu)),
                               Rc::new(RefCell::new(keyboard)));
        Ok(cia)
    }

    #[test]
    fn read_default_cia1_reg_0x00() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0x7f, cia.read(Reg::PRA.addr()));
    }

    #[test]
    fn read_default_cia1_reg_0x02() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0xff, cia.read(Reg::DDRA.addr()));
    }

    #[test]
    fn read_default_cia1_reg_0x03() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0x00, cia.read(Reg::DDRB.addr()));
    }

    #[test]
    fn read_default_cia1_reg_0x0d() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0x00, cia.read(Reg::ICR.addr())); // 0x81
    }

    #[test]
    fn read_default_cia1_reg_0x0e() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0x08, cia.read(Reg::CRA.addr())); // 0x11
    }

    #[test]
    fn read_default_cia1_reg_0x0f() {
        let mut cia = setup_cia().unwrap();
        assert_eq!(0x08, cia.read(Reg::CRB.addr()));
    }

    #[test]
    fn read_keyboard_s() {
        let cpu = setup_cpu().unwrap();
        let mut keyboard = Keyboard::new();
        keyboard.set_row(1, !(1 << 5));
        let mut cia = Cia::new(Mode::Cia1,
                               Rc::new(RefCell::new(cpu)),
                               Rc::new(RefCell::new(keyboard)));
        cia.write(Reg::DDRA.addr(), 0xff);
        cia.write(Reg::DDRB.addr(), 0x00);
        cia.write(Reg::PRA.addr(), 0xfd);
        assert_eq!(!(1 << 5), cia.read(0x01));
    }

    #[test]
    fn write_reg_0x00() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::PRA.addr(), 0xff);
        assert_eq!(0xff, cia.port_a);
    }

    #[test]
    fn write_reg_0x01() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::PRB.addr(), 0xff);
        assert_eq!(0xff, cia.port_b);
    }

    #[test]
    fn write_reg_0x02() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::DDRA.addr(), 0xff);
        assert_eq!(0xff, cia.ddr_a);
    }

    #[test]
    fn write_reg_0x03() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::DDRB.addr(), 0xff);
        assert_eq!(0xff, cia.ddr_b);
    }

    #[test]
    fn write_reg_0x04() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TALO.addr(), 0xab);
        assert_eq!(0xab, cia.timer_a.latch & 0x00ff);
    }

    #[test]
    fn write_reg_0x05() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_a.latch & 0xff00) >> 8);
    }

    #[test]
    fn write_reg_0x06() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TBLO.addr(), 0xab);
        assert_eq!(0xab, cia.timer_b.latch & 0x00ff);
    }

    #[test]
    fn write_reg_0x07() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TBHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_b.latch & 0xff00) >> 8);
    }

    #[test]
    fn write_reg_0x0d() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::ICR.addr(), 1 << 7 | 1 << 1 | 1 << 0);
        assert_eq!(true, cia.timer_a.int_enabled);
        assert_eq!(true, cia.timer_b.int_enabled);
        cia.write(Reg::ICR.addr(), 1 << 1);
        assert_eq!(true, cia.timer_a.int_enabled);
        assert_eq!(false, cia.timer_b.int_enabled);
    }

    #[test]
    fn write_reg_0x0e() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::CRA.addr(), (1 << 0) | (1 << 3) | (1 << 5));
        assert_eq!(true, cia.timer_a.enabled);
        assert_eq!(TimerMode::OneShot, cia.timer_a.mode);
        assert_eq!(TimerInput::External, cia.timer_a.input);
    }

    #[test]
    fn write_reg_0x0f() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::CRB.addr(), (1 << 0) | (1 << 3) | (1 << 5));
        assert_eq!(true, cia.timer_b.enabled);
        assert_eq!(TimerMode::OneShot, cia.timer_b.mode);
        assert_eq!(TimerInput::External, cia.timer_b.input);
    }

    #[test]
    fn load_timer_a_value() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TALO.addr(), 0xab);
        assert_eq!(0x00, cia.timer_a.value);
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0xcdab, cia.timer_a.value);
    }

    #[test]
    fn load_timer_b_value() {
        let mut cia = setup_cia().unwrap();
        cia.write(Reg::TBLO.addr(), 0xab);
        assert_eq!(0x00, cia.timer_b.value);
        cia.write(Reg::TBHI.addr(), 0xcd);
        assert_eq!(0xcdab, cia.timer_b.value);
    }

    /*
    ; This program waits until the key "S" was pushed.
    ; Start with SYS 49152

    *=$c000                  ; startaddress

    PRA  =  $dc00            ; CIA#1 (Port Register A)
    DDRA =  $dc02            ; CIA#1 (Data Direction Register A)

    PRB  =  $dc01            ; CIA#1 (Port Register B)
    DDRB =  $dc03            ; CIA#1 (Data Direction Register B)


    start    sei             ; interrupts deactivated

             lda #%11111111  ; CIA#1 port A = outputs
             sta DDRA

             lda #%00000000  ; CIA#1 port B = inputs
             sta DDRB

             lda #%11111101  ; testing column 1 (COL1) of the matrix
             sta PRA

    loop     lda PRB
             and #%00100000  ; masking row 5 (ROW5)
             bne loop        ; wait until key "S"

             cli             ; interrupts activated

    ende     rts             ; back to BASIC
    */
}
