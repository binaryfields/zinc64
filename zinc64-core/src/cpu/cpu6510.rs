// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use crate::factory::{make_noop, Addressable, Cpu, TickFn};
use crate::util::{IoPort, IrqLine, Pin, Shared};
use core::fmt;
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
    Carry = 1,
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
    Break = 1,
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

struct Registers {
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    pc: u16,
    p: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            pc: 0,
            p: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0;
        self.pc = 0;
        self.p = 0;
    }
}

pub struct Cpu6510 {
    // Dependencies
    mem: Shared<dyn Addressable>,
    // Runtime State
    regs: Registers,
    last_nmi: bool,
    // I/O
    ba_line: Shared<Pin>,
    io_port: Shared<IoPort>,
    irq_line: Shared<IrqLine>,
    nmi_line: Shared<IrqLine>,
}

impl Cpu6510 {
    pub fn new(
        mem: Shared<dyn Addressable>,
        io_port: Shared<IoPort>,
        ba_line: Shared<Pin>,
        irq_line: Shared<IrqLine>,
        nmi_line: Shared<IrqLine>,
    ) -> Self {
        Self {
            mem,
            regs: Registers::new(),
            last_nmi: false,
            ba_line,
            io_port,
            irq_line,
            nmi_line,
        }
    }

    fn execute(&mut self, instr: &Instruction, tick_fn: &TickFn) {
        match *instr {
            //  Data Movement
            Instruction::LDA(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.regs.a = value;
            }
            Instruction::LDX(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.regs.x = value;
            }
            Instruction::LDY(ref op) => {
                let value = op.get(self, tick_fn);
                self.update_nz(value);
                self.regs.y = value;
            }
            Instruction::PHA => {
                let value = self.regs.a;
                self.push(value, tick_fn);
                tick_fn();
            }
            Instruction::PHP => {
                // NOTE undocumented behavior
                let value = self.regs.p | (Flag::Break as u8) | (Flag::Reserved as u8);
                self.push(value, tick_fn);
                tick_fn();
            }
            Instruction::PLA => {
                let value = self.pop(tick_fn);
                self.update_nz(value);
                self.regs.a = value;
                tick_fn();
                tick_fn();
            }
            Instruction::PLP => {
                let value = self.pop(tick_fn);
                self.regs.p = value;
                tick_fn();
                tick_fn();
            }
            Instruction::STA(ref op) => {
                let value = self.regs.a;
                op.set(self, value, true, tick_fn);
            }
            Instruction::STX(ref op) => {
                let value = self.regs.x;
                op.set(self, value, true, tick_fn);
            }
            Instruction::STY(ref op) => {
                let value = self.regs.y;
                op.set(self, value, true, tick_fn);
            }
            Instruction::TAX => {
                let value = self.regs.a;
                self.update_nz(value);
                self.regs.x = value;
                tick_fn();
            }
            Instruction::TAY => {
                let value = self.regs.a;
                self.update_nz(value);
                self.regs.y = value;
                tick_fn();
            }
            Instruction::TSX => {
                let value = self.regs.sp;
                self.update_nz(value);
                self.regs.x = value;
                tick_fn();
            }
            Instruction::TXA => {
                let value = self.regs.x;
                self.update_nz(value);
                self.regs.a = value;
                tick_fn();
            }
            Instruction::TXS => {
                let value = self.regs.x;
                // NOTE do not set nz
                self.regs.sp = value;
                tick_fn();
            }
            Instruction::TYA => {
                let value = self.regs.y;
                self.update_nz(value);
                self.regs.a = value;
                tick_fn();
            }
            // Arithmetic
            Instruction::ADC(ref op) => {
                let ac = self.regs.a as u16;
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
                self.regs.a = result;
            }
            Instruction::SBC(ref op) => {
                let ac = self.regs.a as u16;
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
                self.regs.a = result;
            }
            Instruction::CMP(ref op) => {
                let result = (self.regs.a as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
            }
            Instruction::CPX(ref op) => {
                let result = (self.regs.x as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
            }
            Instruction::CPY(ref op) => {
                let result = (self.regs.y as u16).wrapping_sub(op.get(self, tick_fn) as u16);
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
                let result = self.regs.x.wrapping_sub(1);
                self.update_nz(result);
                self.regs.x = result;
                tick_fn();
            }
            Instruction::DEY => {
                let result = self.regs.y.wrapping_sub(1);
                self.update_nz(result);
                self.regs.y = result;
                tick_fn();
            }
            Instruction::INC(ref op) => {
                let result = op.get(self, tick_fn).wrapping_add(1);
                self.update_nz(result);
                op.set(self, result, true, tick_fn);
                tick_fn();
            }
            Instruction::INX => {
                let result = self.regs.x.wrapping_add(1);
                self.update_nz(result);
                self.regs.x = result;
                tick_fn();
            }
            Instruction::INY => {
                let result = self.regs.y.wrapping_add(1);
                self.update_nz(result);
                self.regs.y = result;
                tick_fn();
            }
            // Logical
            Instruction::AND(ref op) => {
                let result = op.get(self, tick_fn) & self.regs.a;
                self.update_nz(result);
                self.regs.a = result;
            }
            Instruction::EOR(ref op) => {
                let result = op.get(self, tick_fn) ^ self.regs.a;
                self.update_nz(result);
                self.regs.a = result;
            }
            Instruction::ORA(ref op) => {
                let result = op.get(self, tick_fn) | self.regs.a;
                self.update_nz(result);
                self.regs.a = result;
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
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BCS(ref op) => {
                if self.test_flag(Flag::Carry) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BEQ(ref op) => {
                if self.test_flag(Flag::Zero) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BMI(ref op) => {
                if self.test_flag(Flag::Negative) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BNE(ref op) => {
                if !self.test_flag(Flag::Zero) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BPL(ref op) => {
                if !self.test_flag(Flag::Negative) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BVC(ref op) => {
                if !self.test_flag(Flag::Overflow) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::BVS(ref op) => {
                if self.test_flag(Flag::Overflow) {
                    self.regs.pc = op.ea(self, false, tick_fn);
                }
            }
            Instruction::JMP(ref op) => {
                self.regs.pc = op.ea(self, false, tick_fn);
            }
            Instruction::JSR(ref op) => {
                let pc = self.regs.pc.wrapping_sub(1);
                self.push(((pc >> 8) & 0xff) as u8, tick_fn);
                self.push((pc & 0xff) as u8, tick_fn);
                self.regs.pc = op.ea(self, false, tick_fn);
                tick_fn();
            }
            Instruction::RTS => {
                let address = (self.pop(tick_fn) as u16) | ((self.pop(tick_fn) as u16) << 8);
                self.regs.pc = address.wrapping_add(1);
                tick_fn();
                tick_fn();
                tick_fn();
            }
            // Misc
            Instruction::BIT(ref op) => {
                let value = op.get(self, tick_fn);
                let a = self.regs.a;
                self.set_flag(Flag::Negative, value & 0x80 != 0);
                self.set_flag(Flag::Overflow, 0x40 & value != 0);
                self.set_flag(Flag::Zero, value & a == 0);
            }
            Instruction::BRK => {
                self.interrupt(&Interrupt::Break, tick_fn);
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
                self.regs.p = self.pop(tick_fn);
                self.regs.pc = (self.pop(tick_fn) as u16) | ((self.pop(tick_fn) as u16) << 8);
                tick_fn();
                tick_fn();
            }
            // Undocumented
            Instruction::ANE(ref op) => {
                let result = self.regs.a & self.regs.x & op.get(self, tick_fn);
                self.update_nz(result);
                self.regs.a = result;
            }
            Instruction::ANX(ref op) => {
                let result = self.regs.a & op.get(self, tick_fn);
                self.update_nz(result);
                self.regs.a = result;
                self.regs.x = result;
            }
            Instruction::ALR(ref op) => {
                let value = self.regs.a & op.get(self, tick_fn);
                self.set_flag(Flag::Carry, (value & 0x01) != 0);
                let result = value >> 1;
                self.update_nz(result);
                self.regs.a = result;
            }
            Instruction::AXS(ref op) => {
                let result =
                    ((self.regs.a & self.regs.x) as u16).wrapping_sub(op.get(self, tick_fn) as u16);
                self.set_flag(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.regs.x = (result & 0xff) as u8;
            }
            Instruction::LAX(ref op) => {
                let value = op.get(&self, tick_fn);
                self.update_nz(value);
                self.regs.a = value;
                self.regs.x = value;
            }
            Instruction::LSE(ref op) => {
                let value = op.get(self, tick_fn);
                self.set_flag(Flag::Carry, (value & 0x01) != 0);
                let result = self.regs.a ^ (value >> 1);
                self.update_nz(result);
                self.regs.a = result;
                tick_fn();
                tick_fn();
            }
        };
    }

    pub fn fetch_byte(&mut self, tick_fn: &TickFn) -> u8 {
        let byte = self.read_internal(self.regs.pc, tick_fn);
        self.regs.pc = self.regs.pc.wrapping_add(1);
        byte
    }

    pub fn fetch_word(&mut self, tick_fn: &TickFn) -> u16 {
        let word = self.read_internal_u16(self.regs.pc, tick_fn);
        self.regs.pc = self.regs.pc.wrapping_add(2);
        word
    }

    fn interrupt(&mut self, interrupt: &Interrupt, tick_fn: &TickFn) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Interrupt {:?}", interrupt);
        }
        let pc = self.regs.pc;
        let p = self.regs.p;
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
                self.nmi_line.borrow_mut().reset();
            }
            Interrupt::Break => {
                self.push((((pc + 1) >> 8) & 0xff) as u8, tick_fn);
                self.push(((pc + 1) & 0xff) as u8, tick_fn);
                self.push(p | (Flag::Break as u8) | (Flag::Reserved as u8), tick_fn);
                self.set_flag(Flag::IntDisable, true);
            }
            Interrupt::Reset => {}
        }
        self.regs.pc = self.read_internal_u16(interrupt.vector(), tick_fn);
        tick_fn();
    }

    fn pop(&mut self, tick_fn: &TickFn) -> u8 {
        self.regs.sp = self.regs.sp.wrapping_add(1);
        let addr = 0x0100 + self.regs.sp as u16;
        self.read_internal(addr, tick_fn)
    }

    fn push(&mut self, value: u8, tick_fn: &TickFn) {
        let addr = 0x0100 + self.regs.sp as u16;
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.write_internal(addr, value, tick_fn);
    }

    fn set_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.regs.p |= flag as u8;
        } else {
            self.regs.p &= !(flag as u8);
        }
    }

    fn test_flag(&self, flag: Flag) -> bool {
        (self.regs.p & (flag as u8)) != 0
    }

    fn update_nz(&mut self, value: u8) {
        self.set_flag(Flag::Negative, value & 0x80 != 0);
        self.set_flag(Flag::Zero, value == 0);
    }

    // -- Memory Ops

    pub fn read_internal(&self, address: u16, tick_fn: &TickFn) -> u8 {
        let value = match address {
            0x0000 => self.io_port.borrow().get_direction(),
            0x0001 => self.io_port.borrow().get_value() & 0x3f,
            _ => self.mem.borrow().read(address),
        };
        tick_fn();
        value
    }

    pub fn read_internal_u16(&self, address: u16, tick_fn: &TickFn) -> u16 {
        let low = self.read_internal(address, tick_fn);
        let high = self.read_internal(address + 1, tick_fn);
        ((high as u16) << 8) | low as u16
    }

    pub fn write_internal(&mut self, address: u16, value: u8, tick_fn: &TickFn) {
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
    fn get_a(&self) -> u8 {
        self.regs.a
    }

    fn get_p(&self) -> u8 {
        self.regs.p
    }

    fn get_pc(&self) -> u16 {
        self.regs.pc
    }

    fn get_sp(&self) -> u8 {
        self.regs.sp
    }

    fn get_x(&self) -> u8 {
        self.regs.x
    }

    fn get_y(&self) -> u8 {
        self.regs.y
    }

    fn set_a(&mut self, value: u8) {
        self.regs.a = value;
    }

    fn set_p(&mut self, value: u8) {
        self.regs.p = value;
    }

    fn set_pc(&mut self, value: u16) {
        self.regs.pc = value;
    }

    fn set_sp(&mut self, value: u8) {
        self.regs.sp = value;
    }

    fn set_x(&mut self, value: u8) {
        self.regs.x = value;
    }

    fn set_y(&mut self, value: u8) {
        self.regs.y = value;
    }

    fn reset(&mut self) {
        self.regs.reset();
        self.last_nmi = false;
        self.io_port.borrow_mut().set_value(0xff);
        self.irq_line.borrow_mut().reset();
        self.nmi_line.borrow_mut().reset();
        self.write(0x0000, 0b_0010_1111);
        self.write(0x0001, 0b_0001_1111);
        self.interrupt(&Interrupt::Reset, &make_noop());
    }

    fn step(&mut self, tick_fn: &TickFn) {
        while self.ba_line.borrow().is_low() {
            tick_fn();
        }
        if self.nmi_line.borrow().is_low() {
            self.interrupt(&Interrupt::Nmi, tick_fn);
        } else if self.irq_line.borrow().is_low() && !self.test_flag(Flag::IntDisable) {
            self.interrupt(&Interrupt::Irq, tick_fn);
        }
        let pc = self.regs.pc;
        let opcode = self.fetch_byte(tick_fn);
        let instr = Instruction::decode(self, opcode, tick_fn);
        if log_enabled!(LogLevel::Trace) {
            let op_value = format!("{}", instr);
            trace!(target: "cpu::ins", "0x{:04x}: {:14}; {}", pc, op_value, &self);
        }
        self.execute(&instr, tick_fn);
        self.last_nmi = self.nmi_line.borrow().is_low();
    }

    // -- I/O

    fn read(&self, address: u16) -> u8 {
        self.read_internal(address, &make_noop())
    }

    fn write(&mut self, address: u16, value: u8) {
        self.write_internal(address, value, &make_noop());
    }
}

impl fmt::Display for Cpu6510 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x} {:02x} {:02x} {:02x} {}{}{}{}{}{}{}",
            self.regs.a,
            self.regs.x,
            self.regs.y,
            self.regs.sp,
            if (self.regs.p & Flag::Negative as u8) != 0 {
                "N"
            } else {
                "n"
            },
            if (self.regs.p & Flag::Overflow as u8) != 0 {
                "V"
            } else {
                "v"
            },
            if (self.regs.p & Flag::Decimal as u8) != 0 {
                "B"
            } else {
                "b"
            },
            if (self.regs.p & Flag::Decimal as u8) != 0 {
                "D"
            } else {
                "d"
            },
            if (self.regs.p & Flag::IntDisable as u8) != 0 {
                "I"
            } else {
                "i"
            },
            if (self.regs.p & Flag::Zero as u8) != 0 {
                "Z"
            } else {
                "z"
            },
            if (self.regs.p & Flag::Carry as u8) != 0 {
                "C"
            } else {
                "c"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::operand::Operand;
    use super::*;
    use crate::util::{new_shared, Ram};

    struct MockMemory {
        ram: Ram,
    }

    impl MockMemory {
        pub fn new(ram: Ram) -> Self {
            Self { ram }
        }
    }

    impl Addressable for MockMemory {
        fn read(&self, address: u16) -> u8 {
            self.ram.read(address)
        }

        fn write(&mut self, address: u16, value: u8) {
            self.ram.write(address, value);
        }
    }

    fn setup_cpu() -> Cpu6510 {
        let ba_line = new_shared(Pin::new_high());
        let cpu_io_port = new_shared(IoPort::new(0x00, 0xff));
        let cpu_irq = new_shared(IrqLine::new("irq"));
        let cpu_nmi = new_shared(IrqLine::new("nmi"));
        let mem = new_shared(MockMemory::new(Ram::new(0x10000)));
        Cpu6510::new(mem, cpu_io_port, ba_line, cpu_irq, cpu_nmi)
    }

    #[test]
    fn adc_80_16() {
        let mut cpu = setup_cpu();
        cpu.set_a(80);
        cpu.set_flag(Flag::Carry, false);
        cpu.execute(&Instruction::ADC(Operand::Immediate(16)), &make_noop());
        assert_eq!(96, cpu.get_a());
        assert_eq!(false, cpu.test_flag(Flag::Carry));
        assert_eq!(false, cpu.test_flag(Flag::Negative));
        assert_eq!(false, cpu.test_flag(Flag::Overflow));
    }

    #[test]
    fn inc_with_overflow() {
        let mut cpu = setup_cpu();
        cpu.set_a(0xff);
        cpu.execute(&Instruction::INC(Operand::Accumulator), &make_noop());
        assert_eq!(0x00, cpu.get_a());
        assert_eq!(false, cpu.test_flag(Flag::Negative));
        assert_eq!(true, cpu.test_flag(Flag::Zero));
    }
}
