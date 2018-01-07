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
use std::rc::Rc;

use bit_field::BitField;
use core::{Chip, Icr, IoPort, IrqLine, Pin};
use log::LogLevel;

use super::timer;
use super::timer::Timer;
use super::rtc::Rtc;

// Spec: 6526 COMPLEX INTERFACE ADAPTER (CIA) Datasheet
// Spec: https://www.c64-wiki.com/index.php/CIA
// http://www.unusedino.de/ec64/technical/project64/mapping_c64.html

// TODO cia: revise timer latency
// - load 1c
// - int 1c
// - count 3c

#[derive(PartialEq)]
pub enum Mode {
    Cia1,
    Cia2,
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
            _ => panic!("invalid reg {}", reg),
        }
    }

    #[allow(dead_code)]
    pub fn addr(&self) -> u8 {
        *self as u8
    }
}

pub struct Cia {
    // Dependencies
    mode: Mode,
    cpu_irq: Rc<RefCell<IrqLine>>,
    cpu_nmi: Rc<RefCell<IrqLine>>,
    joystick_1: Option<Rc<Cell<u8>>>,
    joystick_2: Option<Rc<Cell<u8>>>,
    keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    // Functional Units
    timer_a: Timer,
    timer_b: Timer,
    tod_alarm: Rtc,
    tod_clock: Rtc,
    tod_set_alarm: bool,
    // Interrupts
    int_control: Icr,
    int_triggered: bool,
    // I/O
    cnt: Rc<RefCell<Pin>>,
    flag: Rc<RefCell<Pin>>,
    port_a: Rc<RefCell<IoPort>>,
    port_b: Rc<RefCell<IoPort>>,
}

impl Cia {
    pub fn new(
        mode: Mode,
        cia_flag: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        cpu_irq: Rc<RefCell<IrqLine>>,
        cpu_nmi: Rc<RefCell<IrqLine>>,
        joystick_1: Option<Rc<Cell<u8>>>,
        joystick_2: Option<Rc<Cell<u8>>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Cia {
        Cia {
            mode,
            cpu_irq,
            cpu_nmi,
            joystick_1,
            joystick_2,
            keyboard_matrix,
            timer_a: Timer::new(),
            timer_b: Timer::new(),
            tod_alarm: Rtc::new(),
            tod_clock: Rtc::new(),
            tod_set_alarm: false,
            int_control: Icr::new(),
            int_triggered: false,
            cnt: Rc::new(RefCell::new(Pin::new_high())),
            flag: cia_flag,
            port_a: cia_port_a,
            port_b: cia_port_b,
        }
    }

    pub fn tod_tick(&mut self) {
        self.tod_clock.tick();
        if self.tod_clock == self.tod_alarm {
            self.int_control.set_event(2);
            if self.int_control.get_interrupt_request() && !self.int_triggered {
                self.trigger_interrupt();
            }
        }
    }

    fn read_cia1_port_a(&self) -> u8 {
        let joystick_state = self.scan_joystick(&self.joystick_2);
        self.port_a.borrow().get_value() & joystick_state
    }

    fn read_cia1_port_b(&self) -> u8 {
        // let timer_a_out = 1u8 << 6;
        // let timer_b_out = 1u8 << 7;
        let keyboard_state = match self.port_a.borrow().get_value() {
            0x00 => 0x00,
            0xff => 0xff,
            _ => self.scan_keyboard(!self.port_a.borrow().get_value()),
        };
        let joystick_state = self.scan_joystick(&self.joystick_1);
        self.port_b.borrow().get_value() & keyboard_state & joystick_state
    }

    fn read_cia2_port_a(&self) -> u8 {
        // iec inputs
        self.port_a.borrow().get_value()
    }

    fn read_cia2_port_b(&self) -> u8 {
        self.port_b.borrow().get_value()
    }

    fn scan_joystick(&self, joystick: &Option<Rc<Cell<u8>>>) -> u8 {
        if let Some(ref state) = *joystick {
            !state.get()
        } else {
            0xff
        }
    }

    fn scan_keyboard(&self, columns: u8) -> u8 {
        let mut result = 0;
        for i in 0..8 as usize {
            if columns.get_bit(i) {
                result |= self.keyboard_matrix.borrow()[i];
            }
        }
        result
    }

    // -- Interrupt Ops

    fn clear_interrupt(&mut self) {
        match self.mode {
            Mode::Cia1 => self.cpu_irq.borrow_mut().clear(0), // FIXME magic value
            Mode::Cia2 => self.cpu_nmi.borrow_mut().clear(0), // FIXME magic value
        }
        self.int_triggered = false;
    }

    fn trigger_interrupt(&mut self) {
        match self.mode {
            Mode::Cia1 => self.cpu_irq.borrow_mut().set(0), // FIXME magic value
            Mode::Cia2 => self.cpu_nmi.borrow_mut().set(0), // FIXME magic value
        }
        self.int_triggered = true;
    }
}

impl Chip for Cia {
    fn clock(&mut self) {
        // Process timers
        let timer_a_underflow = if self.timer_a.enabled {
            let pulse = match self.timer_a.input {
                timer::Input::SystemClock => 1,
                timer::Input::External => if self.cnt.borrow().is_rising() {
                    1
                } else {
                    0
                },
                _ => panic!("invalid input source {:?}", self.timer_a.input),
            };
            self.timer_a.update(pulse)
        } else {
            false
        };
        let timer_b_underflow = if self.timer_b.enabled {
            let pulse = match self.timer_b.input {
                timer::Input::SystemClock => 1,
                timer::Input::External => if self.cnt.borrow().is_rising() {
                    1
                } else {
                    0
                },
                timer::Input::TimerA => if timer_a_underflow {
                    1
                } else {
                    0
                },
                timer::Input::TimerAWithCNT => {
                    if timer_a_underflow && self.cnt.borrow().is_high() {
                        1
                    } else {
                        0
                    }
                }
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
        if timer_a_underflow {
            self.int_control.set_event(0);
        }
        if timer_b_underflow {
            self.int_control.set_event(1);
        }
        if self.flag.borrow().is_falling() {
            self.int_control.set_event(4);
        }
        if self.int_control.get_interrupt_request() && !self.int_triggered {
            self.trigger_interrupt();
        }
    }

    fn clock_delta(&mut self, delta: u32) {
        for _i in 0..delta {
            self.clock();
        }
    }

    fn process_vsync(&mut self) {
        // FIXME tod counter
    }

    fn reset(&mut self) {
        /*
        A low on the RES pin resets all internal registers.The
        port pins are set as inputs and port registers to zero
        (although a read of the ports will return all highs
        because of passive pullups).The timer control registers
        are set to zero and the timer latches to all ones. All other
        registers are reset to zero.
        */
        self.port_a.borrow_mut().reset();
        self.port_b.borrow_mut().reset();
        self.timer_a.reset();
        self.timer_b.reset();
        self.tod_set_alarm = false;
        self.int_control.reset();
        self.int_triggered = false;
        self.cnt.borrow_mut().set_active(true);
        self.flag.borrow_mut().set_active(false);
    }

    // I/O

    fn read(&mut self, reg: u8) -> u8 {
        let value = match Reg::from(reg) {
            Reg::PRA => match self.mode {
                Mode::Cia1 => self.read_cia1_port_a(),
                Mode::Cia2 => self.read_cia2_port_a(),
            },
            Reg::PRB => match self.mode {
                Mode::Cia1 => self.read_cia1_port_b(),
                Mode::Cia2 => self.read_cia2_port_b(),
            },
            Reg::DDRA => self.port_a.borrow().get_direction(),
            Reg::DDRB => self.port_b.borrow().get_direction(),
            Reg::TALO => (self.timer_a.value & 0xff) as u8,
            Reg::TAHI => (self.timer_a.value >> 8) as u8,
            Reg::TBLO => (self.timer_b.value & 0xff) as u8,
            Reg::TBHI => (self.timer_b.value >> 8) as u8,
            Reg::TODTS => {
                self.tod_clock.set_enabled(true);
                to_bcd(self.tod_clock.get_tenth())
            }
            Reg::TODSEC => to_bcd(self.tod_clock.get_seconds()),
            Reg::TODMIN => to_bcd(self.tod_clock.get_minutes()),
            Reg::TODHR => {
                let mut result = to_bcd(self.tod_clock.get_hours());
                result.set_bit(7, self.tod_clock.get_pm());
                result
            },
            Reg::SDR => 0,
            Reg::ICR => {
                /*
                In a multi-chip system, the IR bit can be polled to detect which chip has generated
                an interrupt request. The interrupt DATA register
                is cleared and the IRQ line returns high following a
                read of the DATA register.
                */
                let data = self.int_control.get_data();
                self.int_control.clear();
                self.clear_interrupt();
                data
            }
            Reg::CRA => {
                let timer = &self.timer_a;
                let mut result = 0;
                result
                    .set_bit(0, timer.enabled)
                    .set_bit(1, timer.output_enabled)
                    .set_bit(2, timer.output == timer::Output::Toggle)
                    .set_bit(3, timer.mode == timer::Mode::OneShot)
                    .set_bit(5, timer.input == timer::Input::External);
                result
            }
            Reg::CRB => {
                let timer = &self.timer_b;
                let mut result = 0;
                result
                    .set_bit(0, timer.enabled)
                    .set_bit(1, timer.output_enabled)
                    .set_bit(2, timer.output == timer::Output::Toggle)
                    .set_bit(3, timer.mode == timer::Mode::OneShot)
                    .set_bit(7, self.tod_set_alarm);
                match timer.input {
                    timer::Input::SystemClock => result.set_bit(5, false),
                    timer::Input::External => result.set_bit(5, true),
                    timer::Input::TimerA => result.set_bit(6, true),
                    timer::Input::TimerAWithCNT => {
                        result
                            .set_bit(5, true)
                            .set_bit(6, true)
                    },
                };
                result
            }
        };
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cia::reg", "Read 0x{:02x} = 0x{:02x}", reg, value);
        }
        value
    }

    fn write(&mut self, reg: u8, value: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cia::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        match Reg::from(reg) {
            Reg::PRA => {
                self.port_a.borrow_mut().set_value(value);
            }
            Reg::PRB => {
                self.port_b.borrow_mut().set_value(value);
            }
            Reg::DDRA => {
                self.port_a.borrow_mut().set_direction(value);
            }
            Reg::DDRB => {
                self.port_b.borrow_mut().set_direction(value);
            }
            Reg::TALO => {
                let result = (self.timer_a.latch & 0xff00) | (value as u16);
                self.timer_a.latch = result;
            }
            Reg::TAHI => {
                let result = ((value as u16) << 8) | (self.timer_a.latch & 0x00ff);
                self.timer_a.latch = result;
                if !self.timer_a.enabled {
                    self.timer_a.value = result;
                }
            }
            Reg::TBLO => {
                let result = (self.timer_b.latch & 0xff00) | (value as u16);
                self.timer_b.latch = result;
            }
            Reg::TBHI => {
                let result = ((value as u16) << 8) | (self.timer_b.latch & 0x00ff);
                self.timer_b.latch = result;
                if !self.timer_b.enabled {
                    self.timer_b.value = result;
                }
            }
            Reg::TODTS => {
                let mut tod = if !self.tod_set_alarm {
                    &mut self.tod_clock
                } else {
                    &mut self.tod_alarm
                };
                tod.set_tenth(from_bcd(value & 0x0f));
            }
            Reg::TODSEC => {
                let mut tod = if !self.tod_set_alarm {
                    &mut self.tod_clock
                } else {
                    &mut self.tod_alarm
                };
                tod.set_seconds(from_bcd(value & 0x7f));
            }
            Reg::TODMIN => {
                let mut tod = if !self.tod_set_alarm {
                    &mut self.tod_clock
                } else {
                    &mut self.tod_alarm
                };
                tod.set_minutes(from_bcd(value & 0x7f));
            }
            Reg::TODHR => {
                let mut tod = if !self.tod_set_alarm {
                    &mut self.tod_clock
                } else {
                    &mut self.tod_alarm
                };
                tod.set_enabled(false);
                tod.set_hours(from_bcd(value & 0x7f));
                tod.set_pm(value.get_bit(7));
            }
            Reg::SDR => {}
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
                self.int_control.update_mask(value);
                if self.int_control.get_interrupt_request() && !self.int_triggered {
                    self.trigger_interrupt();
                }
            }
            Reg::CRA => {
                self.timer_a.enabled = value.get_bit(0);
                self.timer_a.mode = if value.get_bit(3) {
                    timer::Mode::OneShot
                } else {
                    timer::Mode::Continuous
                };
                if value.get_bit(4) {
                    self.timer_a.value = self.timer_a.latch;
                }
                self.timer_a.input = if value.get_bit(5) {
                    timer::Input::External
                } else {
                    timer::Input::SystemClock
                };
            }
            Reg::CRB => {
                self.timer_b.enabled = value.get_bit(0);
                self.timer_b.mode = if value.get_bit(3) {
                    timer::Mode::OneShot
                } else {
                    timer::Mode::Continuous
                };
                if value.get_bit(4) {
                    self.timer_b.value = self.timer_b.latch;
                }
                let input = (value & 0x60) >> 5;
                self.timer_b.input = match input {
                    0 => timer::Input::SystemClock,
                    1 => timer::Input::External,
                    2 => timer::Input::TimerA,
                    3 => timer::Input::TimerAWithCNT,
                    _ => panic!("invalid timer input"),
                };
                self.tod_set_alarm = value.get_bit(7);
            }
        }
    }
}

#[inline]
fn from_bcd(decimal: u8) -> u8 {
    (decimal >> 4) * 10 + (decimal & 0x0f)
}

#[inline]
fn to_bcd(num: u8) -> u8 {
    ((num / 10) << 4) | (num % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_cia() -> Cia {
        let cia_flag = Rc::new(RefCell::new(Pin::new_low()));
        let cia_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
        let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
        let keyboard_matrix = Rc::new(RefCell::new([0xff; 8]));
        let mut cia = Cia::new(
            Mode::Cia1,
            cia_flag,
            cia_port_a,
            cia_port_b,
            cpu_irq,
            cpu_nmi,
            None,
            None,
            keyboard_matrix
        );
        cia.reset();
        cia
    }

    #[allow(dead_code)]
    fn setup_cia_with_keyboard(keyboard_matrix: Rc<RefCell<[u8; 8]>>) -> Cia {
        let cia_flag = Rc::new(RefCell::new(Pin::new_low()));
        let cia_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cia_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
        let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
        let mut cia = Cia::new(
            Mode::Cia1,
            cia_flag,
            cia_port_a,
            cia_port_b,
            cpu_irq,
            cpu_nmi,
            None,
            None,
            keyboard_matrix,
        );
        cia.reset();
        cia
    }

    #[test]
    fn read_regs() {
        let mut cia = setup_cia();
        assert_eq!(0xff, cia.read(Reg::PRA.addr()));
        assert_eq!(0xff, cia.read(Reg::PRB.addr()));
        assert_eq!(0x00, cia.read(Reg::DDRA.addr()));
        assert_eq!(0x00, cia.read(Reg::DDRB.addr()));
        assert_eq!(0x00, cia.read(Reg::TALO.addr()));
        assert_eq!(0x00, cia.read(Reg::TAHI.addr()));
        assert_eq!(0x00, cia.read(Reg::TBLO.addr()));
        assert_eq!(0x00, cia.read(Reg::TBHI.addr()));
        assert_eq!(0x00, cia.read(Reg::TODTS.addr()));
        assert_eq!(0x00, cia.read(Reg::TODSEC.addr()));
        assert_eq!(0x00, cia.read(Reg::TODMIN.addr()));
        assert_eq!(0x00, cia.read(Reg::TODHR.addr()));
        assert_eq!(0x00, cia.read(Reg::SDR.addr()));
        assert_eq!(0x00, cia.read(Reg::ICR.addr()));
        assert_eq!(0x08, cia.read(Reg::CRA.addr()));
        assert_eq!(0x08, cia.read(Reg::CRB.addr()));
    }

    /*
    #[test]
    fn read_keyboard_s() {
        let keyboard_matrix = Rc::new(RefCell::new([0xff; 8]));
        let keyboard = Rc::new(RefCell::new(Keyboard::new(keyboard_matrix.clone())));
        keyboard.borrow_mut().reset();
        let mut cia = setup_cia_with_keyboard(keyboard_matrix.clone());
        keyboard.borrow_mut().enqueue("S");
        keyboard.borrow_mut().drain_event();
        cia.write(Reg::DDRA.addr(), 0xff);
        cia.write(Reg::DDRB.addr(), 0x00);
        cia.write(Reg::PRA.addr(), 0xfd);
        assert_eq!(!(1 << 5), cia.read(Reg::PRB.addr()));
    }
    */

    #[test]
    fn trigger_timer_a_interrupt() {
        let mut cia = setup_cia();
        cia.write(Reg::TALO.addr(), 0x01);
        cia.write(Reg::TAHI.addr(), 0x00);
        cia.write(Reg::ICR.addr(), 0x81); // enable irq for timer a
        cia.write(Reg::CRA.addr(), 0b00011001u8);
        cia.clock();
        {
            let cpu_irq = cia.cpu_irq.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        cia.clock();
        {
            let cpu_irq = cia.cpu_irq.borrow();
            assert_eq!(true, cpu_irq.is_low());
        }
    }

    #[test]
    fn trigger_timer_b_interrupt() {
        let mut cia = setup_cia();
        cia.write(Reg::TBLO.addr(), 0x01);
        cia.write(Reg::TBHI.addr(), 0x00);
        cia.write(Reg::ICR.addr(), 0x82); // enable irq for timer b
        cia.write(Reg::CRB.addr(), 0b00011001u8);
        cia.clock();
        {
            let cpu_irq = cia.cpu_irq.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        cia.clock();
        {
            let cpu_irq = cia.cpu_irq.borrow();
            assert_eq!(true, cpu_irq.is_low());
        }
    }

    #[test]
    fn write_reg_0x00() {
        let mut cia = setup_cia();
        cia.write(Reg::PRA.addr(), 0xff);
        assert_eq!(0xff, cia.port_a.borrow().get_value());
    }

    #[test]
    fn write_reg_0x01() {
        let mut cia = setup_cia();
        cia.write(Reg::PRB.addr(), 0xff);
        assert_eq!(0xff, cia.port_b.borrow().get_value());
    }

    #[test]
    fn write_reg_0x02() {
        let mut cia = setup_cia();
        cia.write(Reg::DDRA.addr(), 0xff);
        assert_eq!(0xff, cia.port_a.borrow().get_direction());
    }

    #[test]
    fn write_reg_0x03() {
        let mut cia = setup_cia();
        cia.write(Reg::DDRB.addr(), 0xff);
        assert_eq!(0xff, cia.port_b.borrow().get_direction());
    }

    #[test]
    fn write_reg_0x04() {
        let mut cia = setup_cia();
        cia.write(Reg::TALO.addr(), 0xab);
        assert_eq!(0xab, cia.timer_a.latch & 0x00ff);
    }

    #[test]
    fn write_reg_0x05() {
        let mut cia = setup_cia();
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_a.latch & 0xff00) >> 8);
    }

    #[test]
    fn write_reg_0x06() {
        let mut cia = setup_cia();
        cia.write(Reg::TBLO.addr(), 0xab);
        assert_eq!(0xab, cia.timer_b.latch & 0x00ff);
    }

    #[test]
    fn write_reg_0x07() {
        let mut cia = setup_cia();
        cia.write(Reg::TBHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_b.latch & 0xff00) >> 8);
    }

    #[test]
    fn write_reg_0x0d() {
        let mut cia = setup_cia();
        cia.write(Reg::ICR.addr(), 0b10000011u8);
        assert_eq!(0b00000011u8, cia.int_control.get_mask());
        cia.write(Reg::ICR.addr(), 0b00000010u8);
        assert_eq!(0b00000001u8, cia.int_control.get_mask());
    }

    #[test]
    fn write_reg_0x0e() {
        let mut cia = setup_cia();
        cia.write(Reg::CRA.addr(), 0b00101001u8);
        assert_eq!(true, cia.timer_a.enabled);
        assert_eq!(timer::Mode::OneShot, cia.timer_a.mode);
        assert_eq!(timer::Input::External, cia.timer_a.input);
    }

    #[test]
    fn write_reg_0x0f() {
        let mut cia = setup_cia();
        cia.write(Reg::CRB.addr(), 0b00101001u8);
        assert_eq!(true, cia.timer_b.enabled);
        assert_eq!(timer::Mode::OneShot, cia.timer_b.mode);
        assert_eq!(timer::Input::External, cia.timer_b.input);
    }

    #[test]
    fn write_timer_a_value() {
        let mut cia = setup_cia();
        cia.write(Reg::TALO.addr(), 0xab);
        assert_eq!(0x00, cia.timer_a.value);
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0xcdab, cia.timer_a.value);
    }

    #[test]
    fn write_timer_b_value() {
        let mut cia = setup_cia();
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
