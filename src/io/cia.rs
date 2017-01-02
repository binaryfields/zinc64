/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::cell::RefCell;
use std::rc::Rc;

use cpu::Cpu;
use io::Keyboard;
use util::bit;

// Spec: https://www.c64-wiki.com/index.php/CIA

// TODO cia: fix joy reading
// TODO cia: add cia2 port logic
// TODO cia: add timer output logic
// TODO cia: add rtc
// TODO cia: test cia read regs 0d-of
// TODO cia: test timers

#[allow(dead_code)]
pub struct Cia {
    cpu: Rc<RefCell<Cpu>>,
    keyboard: Rc<RefCell<Keyboard>>,
    mode: Mode,
    port_a: u8,
    port_b: u8,
    ddr_a: u8,
    ddr_b: u8,
    rtc: Rtc,
    timer_a: Timer,
    timer_b: Timer,
    cnt_line: bool,
    cnt_last: bool,
}

#[derive(PartialEq)]
pub enum Mode {
    Cia1,
    Cia2,
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
    int_enabled: bool,
    latch: u16,
    value: u16,
    triggered: bool,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            enabled: false,
            mode: TimerMode::OneShot,
            input: TimerInput::SystemClock,
            output: TimerOutput::Pulse,
            output_enabled: false,
            int_enabled: false,
            latch: 0,
            value: 0,
            triggered: false,
        }
    }

    fn reset(&mut self) {
        match self.mode {
            TimerMode::Continuous => {
                self.value = self.latch;
            },
            TimerMode::OneShot => {
                self.enabled = false;
            }
        }
    }

    pub fn update(&mut self, pulses: u16) {
        if self.enabled {
            self.value = if pulses <= self.value { self.value - pulses } else { 0 };
            if self.value == 0 {
                if self.int_enabled {
                    self.triggered = true;
                }
                self.reset();
            }
        }
    }
}

impl Cia {
    pub fn new(mode: Mode, cpu: Rc<RefCell<Cpu>>, keyboard: Rc<RefCell<Keyboard>>) -> Cia {
        Cia {
            cpu: cpu,
            keyboard: keyboard,
            mode: mode,
            port_a: 0,
            port_b: 0,
            ddr_a: 0xff,
            ddr_b: 0x00,
            rtc: Rtc {},
            timer_a: Timer::new(),
            timer_b: Timer::new(),
            cnt_line: false,
            cnt_last: false,
        }
    }

    pub fn set_cnt(&mut self, value: bool) { self.cnt_line = value; }

    pub fn step(&mut self) {
        let timer_a_int = if self.timer_a.enabled {
            let pulses = match self.timer_a.input {
                TimerInput::SystemClock => 1,
                TimerInput::External => if !self.cnt_last && self.cnt_line { 1 } else { 0 },
                _ => panic!("invalid input source {:?}", self.timer_a.input),
            };
            let prev_triggered = self.timer_a.triggered;
            self.timer_a.update(pulses);
            self.timer_a.triggered && !prev_triggered
        } else {
            false
        };
        let timer_b_int = if self.timer_b.enabled {
            let pulses = match self.timer_b.input {
                TimerInput::SystemClock => 1,
                TimerInput::External => if !self.cnt_last && self.cnt_line { 1 } else { 0 },
                TimerInput::TimerA => if timer_a_int { 1 } else { 0 },
                TimerInput::TimerAWithCNT => if timer_a_int && self.cnt_line { 1 } else { 0 },
            };
            let prev_triggered = self.timer_b.triggered;
            self.timer_b.update(pulses);
            self.timer_b.triggered && !prev_triggered
        } else {
            false
        };
        if timer_a_int || timer_b_int {
            match self.mode {
                Mode::Cia1 => self.cpu.borrow_mut().set_irq(),
                Mode::Cia2 => self.cpu.borrow_mut().set_nmi(),
            }
        }
        self.cnt_last = self.cnt_line;
    }

    // -- Internal Ops

    fn read_cia1_port_a(&self) -> u8 {
        // paddles on 01 = port 1, 10 = port 2
        let paddles = 1u8 << 6;
        // joystick A on port 2
        let joy_up = 1u8 << 0;
        let joy_down = 1u8 << 1;
        let joy_left = 1u8 << 2;
        let joy_right = 1u8 << 3;
        let joy_fire = 1u8 << 4;
        joy_left | joy_right | joy_up | joy_down | joy_fire | 1 << 5 | paddles
    }

    fn read_cia1_port_b(&self) -> u8 {
        // paddles on 01 = port 1, 10 = port 2
        let paddles = 1u8 << 6;
        // joystick B on port 1
        let joy_up = 1u8 << 0;
        let joy_down = 1u8 << 1;
        let joy_left = 1u8 << 2;
        let joy_right = 1u8 << 3;
        let joy_fire = 1u8 << 4;
        let timer_a_out = 1u8 << 6;
        let timer_b_out = 1u8 << 7;
        let keyboard = match self.port_a {
            0x00 => 0x00,
            0xff => 0xff,
            _ => self.scan_keyboard(!self.port_a),
        };
        keyboard  // FIXME | joy_left | joy_right | joy_up | joy_down | joy_fire | timer_a_out | timer_b_out
    }

    fn read_cia2_port_a(&self) -> u8 {
        self.port_a
    }

    fn read_cia2_port_b(&self) -> u8 {
        self.port_b
    }

    fn scan_keyboard(&self, columns: u8) -> u8 {
        let mut result = 0;
        for i in 0..8 {
            if bit::bit_test(columns, i) {
                result = result | self.keyboard.borrow().get_row(i);
            }
        }
        result
    }

    // -- Device I/O

    #[allow(dead_code)]
    pub fn read(&mut self, reg: u8) -> u8 {
        match Reg::from(reg) {
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
            Reg::DDRA => self.ddr_a,
            Reg::DDRB => self.ddr_b,
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
                let timer_a_int = if self.timer_a.triggered { 1 << 0 } else { 0 };
                let timer_b_int = if self.timer_b.triggered { 1 << 1 } else { 0 };
                let int_data = timer_a_int | timer_b_int;
                let int_occurred = if int_data > 0 { 1 << 7 } else { 0 };
                // Clear int data
                self.timer_a.triggered = false;
                self.timer_b.triggered = false;
                int_data | int_occurred
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
        }
    }

    #[allow(dead_code, unused_variables)]
    pub fn write(&mut self, reg: u8, value: u8) {
        match Reg::from(reg) {
            Reg::PRA => {
                self.port_a = value;
            },
            Reg::PRB => {
                self.port_b = value;
            },
            Reg::DDRA => {
                self.ddr_a = value;
            },
            Reg::DDRB => {
                self.ddr_b = value;
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
                let fill = bit::bit_test(value, 7);
                if bit::bit_test(value, 0) {
                    self.timer_a.int_enabled = fill;
                }
                if bit::bit_test(value, 1) {
                    self.timer_b.int_enabled = fill;
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
