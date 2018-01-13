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

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use core::{Cpu, IoPort, IrqLine, MemoryController, TickFn};
use log::LogLevel;

use super::instruction::Instruction;

// Spec: http://nesdev.com/6502.txt
// Design:
//   CPU is responsible for decoding and executing instructions. Its state consists of registers
//   and interrupt lines. Instruction decoding is delegated to Instruction class. Addressing modes
//   are delegated to Operand class. Execution decodes one instruction and forwards it to execution
//   engine which handles logic for each instruction. On each iteration, interrupt lines are check
//   to see if program flow should be interrupted by interrupt request.
//   6510 has two port registers at 0x0000 and 0x0001 that control PLA configuration so they
//   are also handled here.

enum Flag {
    Carry = 1 << 0,
    Zero = 1 << 1,
    IntDisable = 1 << 2,
    Decimal = 1 << 3,
    Break = 1 << 4,
    Reserved = 1 << 5,
    Overflow = 1 << 6,
    Negative = 1 << 7,
}

#[derive(Debug)]
enum Interrupt {
    Break = 1 << 0,
    Irq = 1 << 1,
    Nmi = 1 << 2,
    Reset = 1 << 3,
}

impl Interrupt {
    pub fn vector(&self) -> u16 {
        match *self {
            Interrupt::Break => 0xfffe,
            Interrupt::Irq => 0xfffe,
            Interrupt::Nmi => 0xfffa,
            Interrupt::Reset => 0xfffc,
        }
    }
}

pub struct Cpu6510 {
    // Dependencies
    mem: Rc<RefCell<MemoryController>>,
    // Registers
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    pc: u16,
    sp: u8,
    // I/O
    io_port: Rc<RefCell<IoPort>>,
    irq: Rc<RefCell<IrqLine>>,
    nmi: Rc<RefCell<IrqLine>>,
}

impl Cpu6510 {
    pub fn new(io_port: Rc<RefCell<IoPort>>,
               irq: Rc<RefCell<IrqLine>>,
               nmi: Rc<RefCell<IrqLine>>,
               mem: Rc<RefCell<MemoryController>>) -> Cpu6510 {
        Cpu6510 {
            mem,
            a: 0,
            x: 0,
            y: 0,
            p: 0,
            pc: 0,
            sp: 0,
            io_port,
            irq,
            nmi,
        }
    }

    pub fn get_a(&self) -> u8 {
        self.a
    }

    pub fn get_x(&self) -> u8 {
        self.x
    }

    pub fn get_y(&self) -> u8 {
        self.y
    }

    pub fn set_a(&mut self, value: u8) {
        self.a = value;
    }

    #[inline]
    fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.p |= flag as u8;
        } else {
            self.p &= !(flag as u8);
        }
    }

    pub fn set_x(&mut self, value: u8) {
        self.x = value;
    }

    pub fn set_y(&mut self, value: u8) {
        self.y = value;
    }

    #[inline]
    fn execute(&mut self, instr: &Instruction, tick_fn: &TickFn) {
        match *instr {
            //  Data Movement
            Instruction::LDA(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.a = value;
            }
            Instruction::LDX(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.x = value;
            }
            Instruction::LDY(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.y = value;
            }
            Instruction::PHA => {
                let value = self.a;
                self.push(value, tick_fn);
                tick_fn();
            }
            Instruction::PHP => {
                // NOTE undocumented behavior
                let value = self.p | (Flag::Break as u8) | (Flag::Reserved as u8);
                self.push(value, tick_fn);
                tick_fn();
            }
            Instruction::PLA => {
                let value = self.pop(tick_fn);
                self.update_nz(value);
                self.a = value;
                tick_fn();
                tick_fn();
            }
            Instruction::PLP => {
                let value = self.pop(tick_fn);
                self.p = value;
                tick_fn();
                tick_fn();
            }
            Instruction::STA(ref op) => {
                let value = self.a;
                op.set(self, value, true, tick_fn);
            }
            Instruction::STX(ref op) => {
                let value = self.x;
                op.set(self, value, true, tick_fn);
            }
            Instruction::STY(ref op) => {
                let value = self.y;
                op.set(self, value, true, tick_fn);
            }
            Instruction::TAX => {
                let value = self.a;
                self.update_nz(value);
                self.x = value;
                tick_fn();
            }
            Instruction::TAY => {
                let value = self.a;
                self.update_nz(value);
                self.y = value;
                tick_fn();
            }
            Instruction::TSX => {
                let value = self.sp;
                self.update_nz(value);
                self.x = value;
                tick_fn();
            }
            Instruction::TXA => {
                let value = self.x;
                self.update_nz(value);
                self.a = value;
                tick_fn();
            }
            Instruction::TXS => {
                let value = self.x;
                // NOTE do not set nz
                self.sp = value;
                tick_fn();
            }
            Instruction::TYA => {
                let value = self.y;
                self.update_nz(value);
                self.a = value;
                tick_fn();
            }
            // Arithmetic
            Instruction::ADC(ref op) => {
                let ac = self.a as u16;
                let value = op.get(self, tick_fn) as u16;
                let carry = if self.test_flag(Flag::Carry) { 1 } else { 0 };
                let temp = if !self.test_flag(Flag::Decimal) {
                    ac.wrapping_add(value).wrapping_add(carry)
                } else {
                    let mut t = (ac & 0x0f) + (value & 0x0f) + carry;
                    if t > 0x09 {
                        t += 0x06;
                    }
                    t += (ac & 0xf0) + (value & 0xf0);
                    if t & 0x01f0 > 0x90 {
                        t += 0x60;
                    }
                    t
                };
                self.set_flag(
                    Flag::Overflow,
                    (ac ^ value) & 0x80 == 0 && (ac ^ temp) & 0x80 == 0x80,
                );
                self.set_flag(Flag::Carry, temp > 0xff);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                self.a = result;
            }
            Instruction::SBC(ref op) => {
                let ac = self.a as u16;
                let value = op.get(self, tick_fn) as u16;
                let carry = if self.test_flag(Flag::Carry) { 0 } else { 1 };
                let temp = if !self.test_flag(Flag::Decimal) {
                    ac.wrapping_sub(value).wrapping_sub(carry)
                } else {
                    let mut t = (ac & 0x0f).wrapping_sub(value & 0x0f).wrapping_sub(carry);
                    if t & 0x10 != 0 {
                        t = (t.wrapping_sub(0x06) & 0x0f)
                            | ((ac & 0xf0).wrapping_sub(value & 0xf0).wrapping_sub(0x10));
                    } else {
                        t = (t & 0x0f) | ((ac & 0xf0).wrapping_sub(value & 0xf0));
                    }
                    if t & 0x0100 != 0 {
                        t -= 0x60;
                    }
                    t
                };
                self.set_flag(
                    Flag::Overflow,
                    (ac ^ temp) & 0x80 != 0 && (ac ^ value) & 0x80 == 0x80,
                );
                self.set_flag(Flag::Carry, temp < 0x100);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                self.a = result;
            }
            Instruction::CMP(ref op) => {
                let result = (self.a as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
            }
            Instruction::CPX(ref op) => {
                let result = (self.x as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
            }
            Instruction::CPY(ref op) => {
                let result = (self.y as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
            }
            Instruction::DEC(ref op) => {
                let result = op.get(self, tick_fn).wrapping_sub(1);
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::DEX => {
                let result = self.x.wrapping_sub(1);
                self.update_nz(result);
                self.x = result;
                tick_fn();
            }
            Instruction::DEY => {
                let result = self.y.wrapping_sub(1);
                self.update_nz(result);
                self.y = result;
                tick_fn();
            }
            Instruction::INC(ref op) => {
                let result = op.get(self, tick_fn).wrapping_add(1);
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::INX => {
                let result = self.x.wrapping_add(1);
                self.update_nz(result);
                self.x = result;
                tick_fn();
            }
            Instruction::INY => {
                let result = self.y.wrapping_add(1);
                self.update_nz(result);
                self.y = result;
                tick_fn();
            }
            // Logical
            Instruction::AND(ref op) => {
                let result = op.get(self, tick_fn) & self.a;
                self.update_nz(result);
                self.a = result;
            }
            Instruction::EOR(ref op) => {
                let result = op.get(self, tick_fn) ^ self.a;
                self.update_nz(result);
                self.a = result;
            }
            Instruction::ORA(ref op) => {
                let result = op.get(self, tick_fn) | self.a;
                self.update_nz(result);
                self.a = result;
            }
            // Shift and Rotate
            Instruction::ASL(ref op) => {
                let value = op.get(self, tick_fn);
                self.set_flag(Flag::Carry, (value & 0x80) != 0);
                let result = value << 1;
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::LSR(ref op) => {
                let value = op.get(self, tick_fn);
                self.set_flag(Flag::Carry, (value & 0x01) != 0);
                let result = value >> 1;
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::ROL(ref op) => {
                let value = op.get(self, tick_fn);
                let mut temp = (value as u16) << 1;
                if self.test_flag(Flag::Carry) {
                    temp |= 0x01
                };
                self.set_flag(Flag::Carry, temp > 0xff);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::ROR(ref op) => {
                let value = op.get(self, tick_fn) as u16;
                let mut temp = if self.test_flag(Flag::Carry) {
                    value | 0x100
                } else {
                    value
                };
                self.set_flag(Flag::Carry, temp & 0x01 != 0);
                temp >>= 1;
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            // Control Flow
            Instruction::BCC(ref op) => {
                if !self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BCS(ref op) => {
                if self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BEQ(ref op) => {
                if self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BMI(ref op) => {
                if self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BNE(ref op) => {
                if !self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BPL(ref op) => {
                if !self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BVC(ref op) => {
                if !self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BVS(ref op) => {
                if self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::JMP(ref op) => {
                self.pc = op.ea(self, false, tick_fn);
            }
            Instruction::JSR(ref op) => {
                let pc = self.pc.wrapping_sub(1);
                self.push(((pc >> 8) & 0xff) as u8, tick_fn);
                self.push((pc & 0xff) as u8, tick_fn);
                self.pc = op.ea(self, false, tick_fn);
                tick_fn();
            }
            Instruction::RTS => {
                let address = (self.pop(tick_fn) as u16) | ((self.pop(tick_fn) as u16) << 8);
                self.pc = address.wrapping_add(1);
                tick_fn();
                tick_fn();
                tick_fn();
            }
            // Misc
            Instruction::BIT(ref op) => {
                let value = op.get(self, tick_fn);
                let a = self.a;
                self.set_flag(Flag::Negative, value & 0x80 != 0);
                self.set_flag(Flag::Overflow, 0x40 & value != 0);
                self.set_flag(Flag::Zero, value & a == 0);
            }
            Instruction::BRK => {
                self.interrupt(Interrupt::Break, tick_fn);
            }
            Instruction::CLC => {
                self.set_flag(Flag::Carry, false);
                tick_fn();
            }
            Instruction::CLD => {
                self.set_flag(Flag::Decimal, false);
                tick_fn();
            }
            Instruction::CLI => {
                self.set_flag(Flag::IntDisable, false);
                tick_fn();
            }
            Instruction::CLV => {
                self.set_flag(Flag::Overflow, false);
                tick_fn();
            }
            Instruction::NOP => {
                tick_fn();
            }
            Instruction::SEC => {
                self.set_flag(Flag::Carry, true);
                tick_fn();
            }
            Instruction::SED => {
                self.set_flag(Flag::Decimal, true);
                tick_fn();
            }
            Instruction::SEI => {
                self.set_flag(Flag::IntDisable, true);
                tick_fn();
            }
            Instruction::RTI => {
                self.p = self.pop(tick_fn);
                self.pc = (self.pop(tick_fn) as u16) | ((self.pop(tick_fn) as u16) << 8);
                tick_fn();
                tick_fn();
            }
            // Undocumented
            Instruction::AXS(ref op) => {
                let result = ((self.a & self.x) as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.x = (result & 0xff) as u8;
            }
            Instruction::LAX(ref op) => {
                let value = op.get(&self, tick_fn);
                self.update_nz(value);
                self.a = value;
                self.x = value;
            }
        };
    }

    #[inline]
    pub fn fetch_byte(&mut self, tick_fn: &TickFn) -> u8 {
        let byte = self.read(self.pc, tick_fn);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    #[inline]
    pub fn fetch_word(&mut self, tick_fn: &TickFn) -> u16 {
        let word = self.read_word(self.pc, tick_fn);
        self.pc = self.pc.wrapping_add(2);
        word
    }

    fn interrupt(&mut self, interrupt: Interrupt, tick_fn: &TickFn) -> u8 {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Interrupt {:?}", interrupt);
        }
        let pc = self.pc;
        let p = self.p;
        match interrupt {
            Interrupt::Irq => {
                self.push(((pc >> 8) & 0xff) as u8, tick_fn);
                self.push((pc & 0xff) as u8, tick_fn);
                self.push(p & 0xef, tick_fn);
                self.set_flag(Flag::IntDisable, true);
            }
            Interrupt::Nmi => {
                self.push(((pc >> 8) & 0xff) as u8, tick_fn);
                self.push((pc & 0xff) as u8, tick_fn);
                self.push(p & 0xef, tick_fn);
                self.set_flag(Flag::IntDisable, true);
                self.nmi.borrow_mut().reset();
            }
            Interrupt::Break => {
                self.push((((pc + 1) >> 8) & 0xff) as u8, tick_fn);
                self.push(((pc + 1) & 0xff) as u8, tick_fn);
                self.push(p | (Flag::Break as u8) | (Flag::Reserved as u8), tick_fn);
                self.set_flag(Flag::IntDisable, true);
            }
            Interrupt::Reset => {}
        }
        self.pc = self.read_word(interrupt.vector(), tick_fn);
        tick_fn();
        7
    }

    #[inline]
    fn pop(&mut self, tick_fn: &TickFn) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 + self.sp as u16;
        self.read(addr, tick_fn)
    }

    #[inline]
    fn push(&mut self, value: u8, tick_fn: &TickFn) {
        let addr = 0x0100 + self.sp as u16;
        self.sp = self.sp.wrapping_sub(1);
        self.write(addr, value, tick_fn);
    }

    #[inline]
    fn test_flag(&self, flag: Flag) -> bool {
        (self.p & (flag as u8)) != 0
    }

    #[inline]
    fn update_nz(&mut self, value: u8) {
        self.set_flag(Flag::Negative, value & 0x80 != 0);
        self.set_flag(Flag::Zero, value == 0);
    }

    // -- Memory Ops

    #[inline]
    pub fn read(&self, address: u16, tick_fn: &TickFn) -> u8 {
        let value = match address {
            0x0000 => self.io_port.borrow().get_direction(),
            0x0001 => self.io_port.borrow().get_value(),
            _ => self.mem.borrow().read(address),
        };
        tick_fn();
        value
    }

    #[inline]
    pub fn read_word(&self, address: u16, tick_fn: &TickFn) -> u16 {
        let low = self.read(address, tick_fn);
        let high = self.read(address + 1, tick_fn);
        ((high as u16) << 8) | low as u16
    }

    #[inline]
    pub fn write(&mut self, address: u16, value: u8, tick_fn: &TickFn) {
        match address {
            0x0000 => self.io_port.borrow_mut().set_direction(value),
            0x0001 => self.io_port.borrow_mut().set_value(value),
            _ => {}
        }
        self.mem.borrow_mut().write(address, value);
        tick_fn();
    }
}

impl Cpu for Cpu6510 {
    fn get_pc(&self) -> u16 {
        self.pc
    }

    fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }

    fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.p = 0;
        self.pc = 0;
        self.sp = 0;
        self.io_port.borrow_mut().set_value(0xff);
        self.irq.borrow_mut().reset();
        self.nmi.borrow_mut().reset();
        let tick_fn: TickFn = Box::new(move || {});
        self.write(0x0000, 0b0010_1111, &tick_fn);
        self.write(0x0001, 0b0001_1111, &tick_fn);
        self.interrupt(Interrupt::Reset, &tick_fn);
    }

    fn step(&mut self, tick_fn: &TickFn) {
        if self.nmi.borrow().is_low() {
            self.interrupt(Interrupt::Nmi, tick_fn);
        } else if self.irq.borrow().is_low() && !self.test_flag(Flag::IntDisable) {
            self.interrupt(Interrupt::Irq, tick_fn);
        }
        let pc = self.pc;
        let opcode = self.fetch_byte(tick_fn);
        let instr = Instruction::decode(self, opcode, tick_fn);
        if log_enabled!(LogLevel::Trace) {
            let op_value = format!("{}", instr);
            trace!(target: "cpu::ins", "0x{:04x}: {:14}; {}", pc, op_value, &self);
        }
        self.execute(&instr, tick_fn);
    }

    // I/O

    fn read_debug(&self, address: u16) -> u8 {
        let tick_fn: TickFn = Box::new(move || {});
        self.read(address, &tick_fn)
    }

    fn write_debug(&mut self, address: u16, value: u8) {
        match address {
            0x0000 => self.io_port.borrow_mut().set_direction(value),
            0x0001 => self.io_port.borrow_mut().set_value(value),
            _ => {}
        }
        self.mem.borrow_mut().write(address, value);
    }
}

impl fmt::Display for Cpu6510 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02x} {:02x} {:02x} {:02x} {}{}{}{}{}{}{}",
            self.a,
            self.x,
            self.y,
            self.sp,
            if (self.p & Flag::Negative as u8) != 0 {
                "N"
            } else {
                "n"
            },
            if (self.p & Flag::Overflow as u8) != 0 {
                "V"
            } else {
                "v"
            },
            if (self.p & Flag::Decimal as u8) != 0 {
                "B"
            } else {
                "b"
            },
            if (self.p & Flag::Decimal as u8) != 0 {
                "D"
            } else {
                "d"
            },
            if (self.p & Flag::IntDisable as u8) != 0 {
                "I"
            } else {
                "i"
            },
            if (self.p & Flag::Zero as u8) != 0 {
                "Z"
            } else {
                "z"
            },
            if (self.p & Flag::Carry as u8) != 0 {
                "C"
            } else {
                "c"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::operand::Operand;
    use core::Ram;
    use std::cell::Cell;


    struct MockMemory {
        ram: Ram,
    }

    impl MockMemory {
        pub fn new(ram: Ram) -> Self {
            MockMemory { ram }
        }
    }

    impl MemoryController for MockMemory {
        fn switch_banks(&mut self, _mode: u8) {}

        fn read(&self, address: u16) -> u8 {
            self.ram.read(address)
        }

        fn write(&mut self, address: u16, value: u8) {
            self.ram.write(address, value);
        }
    }

    fn setup_cpu() -> Cpu6510 {
        let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
        let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
        let mem = Rc::new(RefCell::new(MockMemory::new(Ram::new(0x10000))));
        Cpu6510::new(cpu_io_port, cpu_irq, cpu_nmi, mem)
    }

    #[test]
    fn adc_80_16() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.a = 80;
        cpu.set_flag(Flag::Carry, false);
        cpu.execute(&Instruction::ADC(Operand::Immediate(16)), &tick_fn);
        assert_eq!(96, cpu.a);
        assert_eq!(false, cpu.test_flag(Flag::Carry));
        assert_eq!(false, cpu.test_flag(Flag::Negative));
        assert_eq!(false, cpu.test_flag(Flag::Overflow));
    }

    #[test]
    fn inc_with_overflow() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.a = 0xff;
        cpu.execute(&Instruction::INC(Operand::Accumulator), &tick_fn);
        assert_eq!(0x00, cpu.a);
        assert_eq!(false, cpu.test_flag(Flag::Negative));
        assert_eq!(true, cpu.test_flag(Flag::Zero));
    }

    // Based on 65xx Processor Data from http://www.romhacking.net/documents/318/

    const OPCODE_TIMING: [u8; 256] = [
        7, // 00 BRK #$ab
        6, // 01 ORA ($ab,X)
        0, // 02 HLT*
        0, // 03 ASO* ($ab,X)
        0, // 04 SKB* $ab
        3, // 05 ORA $ab
        5, // 06 ASL $ab
        0, // 07 ASO* $ab
        3, // 08 PHP
        2, // 09 ORA #$ab
        2, // 0A ASL A
        0, // 0B ANC* #$ab
        0, // 0C SKW* $abcd
        4, // 0D ORA $abcd
        6, // 0E ASL $abcd
        0, // 0F ASO* $abcd
        2, // 10 BPL nearlabel
        5, // 11 ORA ($ab),Y
        0, // 12 HLT*
        0, // 13 ASO* ($ab),Y
        0, // 14 SKB* $ab,X
        4, // 15 ORA $ab,X
        6, // 16 ASL $ab,X
        0, // 17 ASO* $ab,X
        2, // 18 CLC
        4, // 19 ORA $abcd,Y
        0, // 1A NOP*
        0, // 1B ASO* $abcd,Y
        0, // 1C SKW* $abcd,X
        4, // 1D ORA $abcd,X
        7, // 1E ASL $abcd,X
        0, // 1F ASO* $abcd,X
        6, // 20 JSR $abcd
        6, // 21 AND ($ab,X)
        0, // 22 HLT*
        0, // 23 RLA* ($ab,X)
        3, // 24 BIT $ab
        3, // 25 AND $ab
        5, // 26 ROL $ab
        0, // 27 RLA* $ab
        4, // 28 PLP
        2, // 29 AND #$ab
        2, // 2A ROL A
        0, // 2B ANC* #$ab
        4, // 2C BIT $abcd
        4, // 2D AND $abcd
        6, // 2E ROL $abcd
        0, // 2F RLA* $abcd
        2, // 30 BMI nearlabel
        5, // 31 AND ($ab),Y
        0, // 32 HLT*
        0, // 33 RLA* ($ab),Y
        0, // 34 SKB* $ab,X
        4, // 35 AND $ab,X
        6, // 36 ROL $ab,X
        0, // 37 RLA* $ab,X
        2, // 38 SEC
        4, // 39 AND $abcd,Y
        0, // 3A NOP*
        0, // 3B RLA* $abcd,Y
        0, // 3C SKW* $abcd,X
        4, // 3D AND $abcd,X
        7, // 3E ROL $abcd,X
        0, // 3F RLA* $abcd,X
        6, // 40 RTI
        6, // 41 EOR ($ab,X)
        0, // 42 HLT*
        0, // 43 LSE* ($ab,X)
        0, // 44 SKB* $ab
        3, // 45 EOR $ab
        5, // 46 LSR $ab
        0, // 47 LSE* $ab
        3, // 48 PHA
        2, // 49 EOR #$ab
        2, // 4A LSR A
        0, // 4B ALR* #$ab
        3, // 4C JMP $abcd
        4, // 4D EOR $abcd
        6, // 4E LSR $abcd
        0, // 4F LSE* $abcd
        2, // 50 BVC nearlabel
        5, // 51 EOR ($ab),Y
        0, // 52 HLT*
        0, // 53 LSE* ($ab),Y
        0, // 54 SKB* $ab,X
        4, // 55 EOR $ab,X
        6, // 56 LSR $ab,X
        0, // 57 LSE* $ab,X
        2, // 58 CLI
        4, // 59 EOR $abcd,Y
        0, // 5A NOP*
        0, // 5B LSE* $abcd,Y
        0, // 5C SKW* $abcd,X
        4, // 5D EOR $abcd,X
        7, // 5E LSR $abcd,X
        0, // 5F LSE* $abcd,X
        6, // 60 RTS
        6, // 61 ADC ($ab,X)
        0, // 62 HLT*
        0, // 63 RRA* ($ab,X)
        0, // 64 SKB* $ab
        3, // 65 ADC $ab
        5, // 66 ROR $ab
        0, // 67 RRA* $ab
        4, // 68 PLA
        2, // 69 ADC #$ab
        2, // 6A ROR A
        0, // 6B ARR* #$ab
        5, // 6C JMP ($abcd)
        4, // 6D ADC $abcd
        6, // 6E ROR $abcd
        0, // 6F RRA* $abcd
        2, // 70 BVS nearlabel
        5, // 71 ADC ($ab),Y
        0, // 72 HLT*
        0, // 73 RRA* ($ab),Y
        0, // 74 SKB* $ab,X
        4, // 75 ADC $ab,X
        6, // 76 ROR $ab,X
        0, // 77 RRA* $ab,X
        2, // 78 SEI
        4, // 79 ADC $abcd,Y
        0, // 7A NOP*
        0, // 7B RRA* $abcd,Y
        0, // 7C SKW* $abcd,X
        4, // 7D ADC $abcd,X
        7, // 7E ROR $abcd,X
        0, // 7F RRA* $abcd,X
        0, // 80 SKB* #$ab
        6, // 81 STA ($ab,X)
        0, // 82 SKB* #$ab
        0, // 83 SAX* ($ab,X)
        3, // 84 STY $ab
        3, // 85 STA $ab
        3, // 86 STX $ab
        0, // 87 SAX* $ab
        2, // 88 DEY
        0, // 89 SKB* #$ab
        2, // 8A TXA
        0, // 8B ANE* #$ab
        4, // 8C STY $abcd
        4, // 8D STA $abcd
        4, // 8E STX $abcd
        0, // 8F SAX* $abcd
        2, // 90 BCC nearlabel
        6, // 91 STA ($ab),Y
        0, // 92 HLT*
        0, // 93 SHA* ($ab),Y
        3, // FIXME 4 cycles 94 STY $ab,X
        3, // FIXME 4 cycles 95 STA $ab,X
        4, // 96 STX $ab,Y
        0, // 97 SAX* $ab,Y
        2, // 98 TYA
        5, // 99 STA $abcd,Y
        2, // 9A TXS
        0, // 9B SHS* $abcd,Y
        0, // 9C SHY* $abcd,X
        5, // 9D STA $abcd,X
        0, // 9E SHX* $abcd,Y
        0, // 9F SHA* $abcd,Y
        2, // A0 LDY #$ab
        6, // A1 LDA ($ab,X)
        2, // A2 LDX #$ab
        0, // A3 LAX* ($ab,X)
        3, // A4 LDY $ab
        3, // A5 LDA $ab
        3, // A6 LDX $ab
        3, // A7 LAX* $ab
        2, // A8 TAY
        2, // A9 LDA #$ab
        2, // AA TAX
        0, // AB ANX* #$ab
        4, // AC LDY $abcd
        4, // AD LDA $abcd
        4, // AE LDX $abcd
        0, // AF LAX* $abcd
        2, // B0 BCS nearlabel
        5, // B1 LDA ($ab),Y
        0, // B2 HLT*
        5, // B3 LAX* ($ab),Y
        4, // B4 LDY $ab,X
        4, // B5 LDA $ab,X
        4, // B6 LDX $ab,Y
        0, // B7 LAX* $ab,Y
        2, // B8 CLV
        4, // B9 LDA $abcd,Y
        2, // BA TSX
        0, // BB LAS* $abcd,Y
        4, // BC LDY $abcd,X
        4, // BD LDA $abcd,X
        4, // BE LDX $abcd,Y
        0, // BF LAX* $abcd,Y
        2, // C0 CPY #$ab
        6, // C1 CMP ($ab,X)
        0, // C2 SKB* #$ab
        0, // C3 DCM* ($ab,X)
        3, // C4 CPY $ab
        3, // C5 CMP $ab
        5, // C6 DEC $ab
        0, // C7 DCM* $ab
        2, // C8 INY
        2, // C9 CMP #$ab
        2, // CA DEX
        2, // CB SBX* #$ab
        4, // CC CPY $abcd
        4, // CD CMP $abcd
        6, // CE DEC $abcd
        0, // CF DCM* $abcd
        2, // D0 BNE nearlabel
        5, // D1 CMP ($ab),Y
        0, // D2 HLT*
        0, // D3 DCM* ($ab),Y
        0, // D4 SKB* $ab,X
        4, // D5 CMP $ab,X
        6, // D6 DEC $ab,X
        0, // D7 DCM* $ab,X
        2, // D8 CLD
        4, // D9 CMP $abcd,Y
        0, // DA NOP*
        0, // DB DCM* $abcd,Y
        0, // DC SKW* $abcd,X
        4, // DD CMP $abcd,X
        7, // DE DEC $abcd,X
        0, // DF DCM* $abcd,X
        2, // E0 CPX #$ab
        6, // E1 SBC ($ab,X)
        0, // E2 SKB* #$ab
        0, // E3 INS* ($ab,X)
        3, // E4 CPX $ab
        3, // E5 SBC $ab
        5, // E6 INC $ab
        0, // E7 INS* $ab
        2, // E8 INX
        2, // E9 SBC #$ab
        2, // EA NOP
        0, // EB SBC* #$ab
        4, // EC CPX $abcd
        4, // ED SBC $abcd
        6, // EE INC $abcd
        0, // EF INS* $abcd
        2, // F0 BEQ nearlabel
        5, // F1 SBC ($ab),Y
        0, // F2 HLT*
        0, // F3 INS* ($ab),Y
        0, // F4 SKB* $ab,X
        4, // F5 SBC $ab,X
        6, // F6 INC $ab,X
        0, // F7 INS* $ab,X
        2, // F8 SED
        4, // F9 SBC $abcd,Y
        0, // FA NOP*
        0, // FB INS* $abcd,Y
        0, // FC SKW* $abcd,X
        4, // FD SBC $abcd,X
        7, // FE INC $abcd,X
        0, // FF INS* $abcd,X
    ];

    #[test]
    fn opcode_timing() {
        let mut cpu = setup_cpu();
        for opcode in 0..256 {
            let cycles = OPCODE_TIMING[opcode];
            if cycles > 0 {
                let clock = Rc::new(Cell::new(0u8));
                let clock_clone = clock.clone();
                let tick_fn: TickFn = Box::new(move || {
                    clock_clone.set(clock_clone.get().wrapping_add(1));
                });
                cpu.write_debug(0x1000, opcode as u8);
                cpu.write_debug(0x1001, 0x00);
                cpu.write_debug(0x1002, 0x10);
                cpu.set_pc(0x1000);
                cpu.step(&tick_fn);
                assert_eq!(cycles, clock.get(), "opcode {:02x} timing failed", opcode as u8);
            }
        }
    }
}
