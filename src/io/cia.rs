/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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
use core::{Chip, IoPort, IrqLine, Pin};
use log::LogLevel;

use super::cycle_counter::CycleCounter;
use super::icr::Icr;
use super::rtc::Rtc;
use super::timer;
use super::timer::Timer;

// Spec: 6526 COMPLEX INTERFACE ADAPTER (CIA) Datasheet
// Spec: https://www.c64-wiki.com/index.php/CIA
// http://www.unusedino.de/ec64/technical/project64/mapping_c64.html

enum IntDelay {
    Interrupt0 = 1 << 0,
    Interrupt1 = 1 << 1,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Cia1 = 0,
    Cia2 = 1,
}

impl Mode {
    #[inline]
    pub fn irq_source(&self) -> usize {
        *self as usize
    }
}

#[derive(Copy, Clone)]
pub enum Reg {
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
    #[inline]
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
    #[inline]
    pub fn addr(&self) -> u8 {
        *self as u8
    }
}

pub struct Cia {
    // Dependencies
    mode: Mode,
    irq_line: Rc<RefCell<IrqLine>>,
    joystick_1: Option<Rc<Cell<u8>>>,
    joystick_2: Option<Rc<Cell<u8>>>,
    keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    // Functional Units
    int_control: Icr,
    int_delay: CycleCounter,
    timer_a: Timer,
    timer_b: Timer,
    tod_alarm: Rtc,
    tod_clock: Rtc,
    tod_set_alarm: bool,
    // I/O
    cnt_pin: Rc<RefCell<Pin>>,
    flag_pin: Rc<RefCell<Pin>>,
    port_a: Rc<RefCell<IoPort>>,
    port_b: Rc<RefCell<IoPort>>,
}

impl Cia {
    pub fn new(
        mode: Mode,
        cia_flag: Rc<RefCell<Pin>>,
        cia_port_a: Rc<RefCell<IoPort>>,
        cia_port_b: Rc<RefCell<IoPort>>,
        irq_line: Rc<RefCell<IrqLine>>,
        joystick_1: Option<Rc<Cell<u8>>>,
        joystick_2: Option<Rc<Cell<u8>>>,
        keyboard_matrix: Rc<RefCell<[u8; 8]>>,
    ) -> Cia {
        let cnt_pin = Rc::new(RefCell::new(Pin::new_high()));
        Cia {
            mode,
            irq_line,
            joystick_1,
            joystick_2,
            keyboard_matrix,
            int_control: Icr::new(),
            int_delay: CycleCounter::new(0xffff),
            timer_a: Timer::new(timer::Mode::TimerA, cnt_pin.clone()),
            timer_b: Timer::new(timer::Mode::TimerB, cnt_pin.clone()),
            tod_alarm: Rtc::new(),
            tod_clock: Rtc::new(),
            tod_set_alarm: false,
            cnt_pin: cnt_pin.clone(),
            flag_pin: cia_flag,
            port_a: cia_port_a,
            port_b: cia_port_b,
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
        let mut result = self.port_b.borrow().get_value() & keyboard_state & joystick_state;
        if self.timer_a.is_pb_on() {
            result.set_bit(6, self.timer_a.get_pb_output());
        }
        if self.timer_b.is_pb_on() {
            result.set_bit(7, self.timer_b.get_pb_output());
        }
        result
    }

    fn read_cia2_port_a(&self) -> u8 {
        // iec inputs
        self.port_a.borrow().get_value()
    }

    fn read_cia2_port_b(&self) -> u8 {
        let mut result = self.port_b.borrow().get_value();
        if self.timer_a.is_pb_on() {
            result.set_bit(6, self.timer_a.get_pb_output());
        }
        if self.timer_b.is_pb_on() {
            result.set_bit(7, self.timer_b.get_pb_output());
        }
        result
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

    fn set_interrupt(&mut self, enabled: bool) {
        if enabled {
            self.irq_line.borrow_mut().set(self.mode.irq_source());
        } else {
            self.irq_line.borrow_mut().clear(self.mode.irq_source());
        }
    }
}

impl Chip for Cia {
    fn clock(&mut self) {
        // Process timers
        self.timer_a.feed_source(false);
        let timer_a_output = self.timer_a.clock();
        self.timer_b.feed_source(timer_a_output);
        let timer_b_output = self.timer_b.clock();

        // Process interrupts
        /*
        Any interrupt will set the corresponding bit in the DATA
        register. Any interrupt which is enabled by the MASK
        register will set the IR bit (MSB) of the DATA register
        and bring the IRQ pin low.
        */
        let mut int_event = false;
        if timer_a_output {
            self.int_control.set_event(0);
            int_event = true;
        }
        if timer_b_output {
            self.int_control.set_event(1);
            int_event = true;
        }
        if self.flag_pin.borrow().is_falling() {
            self.int_control.set_event(4);
            int_event = true;
        }
        if int_event && self.int_control.get_interrupt_request() {
            self.int_delay.feed(IntDelay::Interrupt0 as u16);
        }
        if self.int_delay.has_cycle(IntDelay::Interrupt1 as u16) {
            self.set_interrupt(true);
        }
        self.int_delay.clock();
    }

    fn clock_delta(&mut self, delta: u32) {
        for _i in 0..delta {
            self.clock();
        }
    }

    fn process_vsync(&mut self) {
        // FIXME cia: tod counter
        self.tod_clock.tick();
        /*
        self.tod_clock.tick();
        if self.tod_clock == self.tod_alarm {
            self.int_control.set_event(2);
            if self.int_control.get_interrupt_request() && !self.int_triggered {
                self.trigger_interrupt();
            }
        }
        */
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
        self.int_control.reset();
        self.int_delay.reset();
        self.timer_a.reset();
        self.timer_b.reset();
        self.tod_set_alarm = false;
        self.cnt_pin.borrow_mut().set_active(true);
        self.flag_pin.borrow_mut().set_active(false);
        self.port_a.borrow_mut().reset();
        self.port_b.borrow_mut().reset();
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
            Reg::TALO => self.timer_a.get_counter_lo(),
            Reg::TAHI => self.timer_a.get_counter_hi(),
            Reg::TBLO => self.timer_b.get_counter_lo(),
            Reg::TBHI => self.timer_b.get_counter_hi(),
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
            }
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
                self.int_delay.reset();
                self.set_interrupt(false);
                data
            }
            Reg::CRA => {
                self.timer_a.get_config()
            }
            Reg::CRB => {
                let mut config = self.timer_b.get_config();
                config.set_bit(7, self.tod_set_alarm);
                config
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
                self.timer_a.set_latch_lo(value);
            }
            Reg::TAHI => {
                self.timer_a.set_latch_hi(value);
            }
            Reg::TBLO => {
                self.timer_b.set_latch_lo(value);
            }
            Reg::TBHI => {
                self.timer_b.set_latch_hi(value);
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
                if self.int_control.get_interrupt_request() {
                    // FIXME cia: check source in irq_line
                    // && !self.irq_line.borrow().is_low()
                    self.int_delay.feed(IntDelay::Interrupt0 as u16);
                }
            }
            Reg::CRA => {
                self.timer_a.set_config(value);
            }
            Reg::CRB => {
                self.timer_b.set_config(value);
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
        let keyboard_matrix = Rc::new(RefCell::new([0xff; 8]));
        let mut cia = Cia::new(
            Mode::Cia1,
            cia_flag,
            cia_port_a,
            cia_port_b,
            cpu_irq,
            None,
            None,
            keyboard_matrix,
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
        let mut cia = Cia::new(
            Mode::Cia1,
            cia_flag,
            cia_port_a,
            cia_port_b,
            cpu_irq,
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

    #[test]
    fn timer_a_interrupt() {
        let mut cia = setup_cia();
        cia.write(Reg::TALO.addr(), 0x01);
        cia.write(Reg::TAHI.addr(), 0x00);
        cia.clock(); // LOAD0
        cia.clock(); // LOAD1
        cia.write(Reg::ICR.addr(), 0x81); // enable irq for timer a
        cia.write(Reg::CRA.addr(), 0b_0000_1001_u8);
        {
            cia.clock(); // COUNT0|COUNT1
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // COUNT2
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // COUNT3|INT0
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // INT1
            assert_eq!(1, cia.timer_a.get_counter());
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(true, cpu_irq.is_low());
        }
    }

    #[test]
    fn timer_b_interrupt() {
        let mut cia = setup_cia();
        cia.write(Reg::TBLO.addr(), 0x01);
        cia.write(Reg::TBHI.addr(), 0x00);
        cia.clock(); // LOAD0
        assert_eq!(0, cia.timer_b.get_counter());
        cia.clock(); // LOAD1
        assert_eq!(1, cia.timer_b.get_counter());
        cia.write(Reg::ICR.addr(), 0x82); // enable irq for timer b
        cia.write(Reg::CRB.addr(), 0b_0000_1001_u8);
        {
            cia.clock(); // COUNT0|COUNT1
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // COUNT2
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // COUNT3|INT0
            let cpu_irq = cia.irq_line.borrow();
            assert_eq!(false, cpu_irq.is_low());
        }
        {
            cia.clock(); // INT1
            assert_eq!(1, cia.timer_b.get_counter());
            let cpu_irq = cia.irq_line.borrow();
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
        assert_eq!(0xab, cia.timer_a.get_latch() & 0x00ff);
    }

    #[test]
    fn write_reg_0x05() {
        let mut cia = setup_cia();
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_a.get_latch() & 0xff00) >> 8);
    }

    #[test]
    fn write_reg_0x06() {
        let mut cia = setup_cia();
        cia.write(Reg::TBLO.addr(), 0xab);
        assert_eq!(0xab, cia.timer_b.get_latch() & 0x00ff);
    }

    #[test]
    fn write_reg_0x07() {
        let mut cia = setup_cia();
        cia.write(Reg::TBHI.addr(), 0xcd);
        assert_eq!(0xcd, (cia.timer_b.get_latch() & 0xff00) >> 8);
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
    fn write_timer_a_value() {
        let mut cia = setup_cia();
        cia.write(Reg::TALO.addr(), 0xab);
        assert_eq!(0x0000, cia.timer_a.get_counter());
        cia.write(Reg::TAHI.addr(), 0xcd);
        assert_eq!(0x0000, cia.timer_a.get_counter());
        cia.clock();
        cia.clock();
        assert_eq!(0xcdab, cia.timer_a.get_counter());
    }

    #[test]
    fn write_timer_b_value() {
        let mut cia = setup_cia();
        cia.write(Reg::TBLO.addr(), 0xab);
        assert_eq!(0x00, cia.timer_b.get_counter());
        cia.write(Reg::TBHI.addr(), 0xcd);
        cia.clock();
        cia.clock();
        assert_eq!(0xcdab, cia.timer_b.get_counter());
    }

    /*
    Program CIA1TAB - TA, TB, PB67 and ICR in cascaded mode

    Both latches are set to 2. TA counts system clocks, TB counts TA underflows (cascaded).
    PB6 is high for one cycle when TA underflows, PB7 is toggled when TB underflows. IMR is $02.

    TA  01 02 02 01 02 02 01 02 02 01 02 02
    TB  02 02 02 01 01 01 00 00 02 02 02 02
    PB  80 C0 80 80 C0 80 80 C0 00 00 40 00
    ICR 00 01 01 01 01 01 01 01 03 83 83 83
    */

    #[test]
    fn program_cia1tab() {
        let mut cia = setup_cia();
        cia.write(Reg::DDRB.addr(), 0x7f);
        cia.write(Reg::ICR.addr(), 0x82);
        cia.write(Reg::CRA.addr(), 0x00);
        cia.write(Reg::CRB.addr(), 0x00);
        cia.write(Reg::TALO.addr(), 0x02);
        cia.write(Reg::TAHI.addr(), 0x00);
        cia.write(Reg::TBLO.addr(), 0x02);
        cia.write(Reg::TBHI.addr(), 0x00);
        cia.clock();
        cia.clock();
        cia.write(Reg::CRB.addr(), 0x47);
        cia.write(Reg::CRA.addr(), 0x03);
        cia.clock(); // Count0|Count1
        cia.clock(); // Count2
        cia.clock(); // Count3
        assert_eq!(cia.timer_a.get_counter(), 0x01);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x80);
        assert_eq!(cia.int_control.get_raw_data(), 0x00);
        cia.clock(); // Count3|Underflow|Load1
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0xc0);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count2
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x80);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count3
        assert_eq!(cia.timer_a.get_counter(), 0x01);
        assert_eq!(cia.timer_b.get_counter(), 0x01);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x80);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count3|Underflow|Load1
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x01);
        assert_eq!(cia.read(Reg::PRB.addr()), 0xc0);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count2
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x01);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x80);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count3
        assert_eq!(cia.timer_a.get_counter(), 0x01);
        assert_eq!(cia.timer_b.get_counter(), 0x00);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x80);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count3|Underflow|Load1
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x00);
        assert_eq!(cia.read(Reg::PRB.addr()), 0xc0);
        assert_eq!(cia.int_control.get_raw_data(), 0x01);
        cia.clock(); // Count2
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x00);
        assert_eq!(cia.int_control.get_raw_data(), 0x03);
        cia.clock(); // Count3
        assert_eq!(cia.timer_a.get_counter(), 0x01);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x00);
        assert_eq!(cia.int_control.get_raw_data(), 0x03); // 0x83
        cia.clock(); // Count3|Underflow|Load1
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x40);
        assert_eq!(cia.int_control.get_raw_data(), 0x03); // 0x83
        cia.clock(); // Count2
        assert_eq!(cia.timer_a.get_counter(), 0x02);
        assert_eq!(cia.timer_b.get_counter(), 0x02);
        assert_eq!(cia.read(Reg::PRB.addr()), 0x00);
        assert_eq!(cia.int_control.get_raw_data(), 0x03); // 0x83
    }
}
