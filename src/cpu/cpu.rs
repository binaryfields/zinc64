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
use std::fmt;
use std::rc::Rc;

use mem::{Addressable, Memory};
use log::LogLevel;
use util::bit;

use super::instruction::Instruction;
use super::interrupt;
use super::interrupt::Interrupt;

// Spec: http://nesdev.com/6502.txt
// Design:
//   CPU is responsible for decoding and executing instructions. Its state consists of registers
//   and interrupt lines. Instruction decoding is delegated to Instruction class. Addressing modes
//   are delegated to Operand class. Execution decodes one instruction and forwards it to execution
//   engine which handles logic for each instruction. On each iteration, interrupt lines are check
//   to see if program flow should be interrupted by interrupt request.
//   6510 has two port registers at 0x0000 and 0x0001 that control PLA configuration so they
//   are also handled here.

// TODO cpu: switch to clock accurate emulation

pub struct CpuIo {
    pub loram: bool,
    pub hiram: bool,
    pub charen: bool,
    pub cassette_switch: bool,
    pub cassette_motor: bool,
    pub irq: Interrupt,
    pub nmi: Interrupt,
}

impl CpuIo {
    pub fn new() -> CpuIo {
        CpuIo {
            loram: false,
            hiram: false,
            charen: false,
            cassette_switch: true,
            cassette_motor: true,
            irq: Interrupt::new(interrupt::Type::Irq),
            nmi: Interrupt::new(interrupt::Type::Nmi),
        }
    }

    pub fn reset(&mut self) {
        self.loram = true;
        self.hiram = true;
        self.charen = true;
        self.cassette_switch = true;
        self.cassette_motor = true;
        self.irq.reset();
        self.nmi.reset();
    }
}

pub enum Flag {
    Carry = 1 << 0,
    Zero = 1 << 1,
    IntDisable = 1 << 2,
    Decimal = 1 << 3,
    Break = 1 << 4,
    Reserved = 1 << 5,
    Overflow = 1 << 6,
    Negative = 1 << 7,
}

pub struct Cpu {
    // Dependencies
    mem: Rc<RefCell<Memory>>,
    // Registers
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    p: u8,
    // I/O Lines
    io: Rc<RefCell<CpuIo>>,
}

impl Cpu {
    pub fn new(cpu_io: Rc<RefCell<CpuIo>>,
               mem: Rc<RefCell<Memory>>) -> Cpu {
        Cpu {
            mem: mem,
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            p: 0,
            io: cpu_io,
        }
    }

    pub fn get_a(&self) -> u8 {
        self.a
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    #[allow(dead_code)]
    pub fn get_x(&self) -> u8 {
        self.x
    }

    #[allow(dead_code)]
    pub fn get_y(&self) -> u8 {
        self.y
    }

    pub fn set_a(&mut self, value: u8) {
        self.a = value;
    }

    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }

    #[allow(dead_code)]
    pub fn set_x(&mut self, value: u8) {
        self.x = value;
    }

    #[allow(dead_code)]
    pub fn set_y(&mut self, value: u8) {
        self.y = value;
    }

    pub fn execute(&mut self) -> u8 {
        if self.io.borrow().nmi.is_low() {
            self.interrupt(interrupt::Type::Nmi);
        } else if self.io.borrow().irq.is_low() && !self.test_flag(Flag::IntDisable) {
            self.interrupt(interrupt::Type::Irq);
        }
        let pc = self.pc;
        let opcode = self.fetch_op();
        let op = Instruction::decode(self, opcode);
        if log_enabled!(LogLevel::Trace) {
            let op_value = format!("{}", op);
            trace!(target: "cpu::ins", "0x{:04x}: {:14}; {}", pc, op_value, &self);
        }
        self.execute_instruction(&op)
    }

    fn execute_instruction(&mut self, instr: &Instruction) -> u8 {
        match *instr {
            // -- Data Movement
            Instruction::LDA(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.a = value;
                self.tick(cycles)
            },
            Instruction::LDX(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.x = value;
                self.tick(cycles)
            },
            Instruction::LDY(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.y = value;
                self.tick(cycles)
            },
            Instruction::STA(ref op, cycles) => {
                let value = self.a;
                op.set(self, value);
                self.tick(cycles)
            },
            Instruction::STX(ref op, cycles) => {
                let value = self.x;
                op.set(self, value);
                self.tick(cycles)
            },
            Instruction::STY(ref op, cycles) => {
                let value = self.y;
                op.set(self, value);
                self.tick(cycles)
            },
            Instruction::TAX(cycles) => {
                let value = self.a;
                self.update_nz(value);
                self.x = value;
                self.tick(cycles)
            },
            Instruction::TAY(cycles) => {
                let value = self.a;
                self.update_nz(value);
                self.y = value;
                self.tick(cycles)
            },
            Instruction::TSX(cycles) => {
                let value = self.sp;
                self.update_nz(value);
                self.x = value;
                self.tick(cycles)
            },
            Instruction::TXA(cycles) => {
                let value = self.x;
                self.update_nz(value);
                self.a = value;
                self.tick(cycles)
            },
            Instruction::TXS(cycles) => {
                let value = self.x;
                // NOTE do not set nz
                self.sp = value;
                self.tick(cycles)
            },
            Instruction::TYA(cycles) => {
                let value = self.y;
                self.update_nz(value);
                self.a = value;
                self.tick(cycles)
            },
            // -- Stack
            Instruction::PHA(cycles) => {
                let value = self.a;
                self.push(value);
                self.tick(cycles)
            },
            Instruction::PHP(cycles) => {
                // NOTE undocumented behavior
                let value = self.p | (Flag::Break as u8) | (Flag::Reserved as u8);
                self.push(value);
                self.tick(cycles)
            },
            Instruction::PLA(cycles) => {
                let value = self.pop();
                self.update_nz(value);
                self.a = value;
                self.tick(cycles)
            },
            Instruction::PLP(cycles) => {
                let value = self.pop();
                self.p = value;
                self.tick(cycles)
            },
            // -- Arithmetic
            Instruction::ADC(ref op, cycles) => {
                let ac = self.a as u16;
                let value = op.get(self) as u16;
                let carry = if self.test_flag(Flag::Carry) { 1 } else { 0 };
                let temp = if !self.test_flag(Flag::Decimal) {
                    ac.wrapping_add(value).wrapping_add(carry)
                } else {
                    let mut t = (ac & 0x0f) + (value & 0x0f) + carry;
                    if t > 0x09 { t += 0x06; }
                    t += (ac & 0xf0) + (value & 0xf0);
                    if t & 0x01f0 > 0x90 { t += 0x60; }
                    t
                };
                self.update_f(Flag::Overflow, (ac ^ value) & 0x80 == 0 && (ac ^ temp) & 0x80 == 0x80);
                self.update_f(Flag::Carry, temp > 0xff);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles)
            },
            Instruction::SBC(ref op, cycles) => {
                let ac = self.a as u16;
                let value = op.get(self) as u16;
                let carry = if self.test_flag(Flag::Carry) { 0 } else { 1 };
                let temp = if !self.test_flag(Flag::Decimal) {
                    ac.wrapping_sub(value).wrapping_sub(carry)
                } else {
                    let mut t = (ac & 0x0f).wrapping_sub(value & 0x0f).wrapping_sub(carry);
                    if t & 0x10 != 0 {
                        t = (t.wrapping_sub(0x06) & 0x0f) | ((ac & 0xf0).wrapping_sub(value & 0xf0).wrapping_sub(0x10));
                    } else {
                        t = (t & 0x0f) | ((ac & 0xf0).wrapping_sub(value & 0xf0));
                    }
                    if t & 0x0100 != 0 { t -= 0x60; }
                    t
                };
                self.update_f(Flag::Overflow, (ac ^ temp) & 0x80 != 0 && (ac ^ value) & 0x80 == 0x80);
                self.update_f(Flag::Carry, temp < 0x100);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles)
            },
            Instruction::DEC(ref op, cycles) => {
                let result = op.get(self).wrapping_sub(1);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            Instruction::DEX(cycles) => {
                let result = self.x.wrapping_sub(1);
                self.update_nz(result);
                self.x = result;
                self.tick(cycles)
            },
            Instruction::DEY(cycles) => {
                let result = self.y.wrapping_sub(1);
                self.update_nz(result);
                self.y = result;
                self.tick(cycles)
            },
            Instruction::INC(ref op, cycles) => {
                let result = op.get(self).wrapping_add(1);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            Instruction::INX(cycles) => {
                let result = self.x.wrapping_add(1);
                self.update_nz(result);
                self.x = result;
                self.tick(cycles)
            },
            Instruction::INY(cycles) => {
                let result = self.y.wrapping_add(1);
                self.update_nz(result);
                self.y = result;
                self.tick(cycles)
            },
            Instruction::CMP(ref op, cycles) => {
                let result = (self.a as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles)
            },
            Instruction::CPX(ref op, cycles) => {
                let result = (self.x as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles)
            },
            Instruction::CPY(ref op, cycles) => {
                let result = (self.y as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles)
            },
            // -- Logical
            Instruction::AND(ref op, cycles) => {
                let result = op.get(self) & self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles)
            },
            Instruction::EOR(ref op, cycles) => {
                let result = op.get(self) ^ self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles)
            },
            Instruction::ORA(ref op, cycles) => {
                let result = op.get(self) | self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles)
            },
            // -- Shift and Rotate
            Instruction::ASL(ref op, cycles) => {
                let value = op.get(self);
                let result = value << 1;
                self.update_f(Flag::Carry, (value & 0x80) != 0);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            Instruction::LSR(ref op, cycles) => {
                let value = op.get(self);
                let result = value >> 1;
                self.update_f(Flag::Carry, (value & 0x01) != 0);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            Instruction::ROL(ref op, cycles) => {
                let value = op.get(self);
                let mut temp = (value as u16) << 1;
                if self.test_flag(Flag::Carry) { temp |= 0x01 };
                self.update_f(Flag::Carry, temp > 0xff);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            Instruction::ROR(ref op, cycles) => {
                let value = op.get(self) as u16;
                let mut temp = if self.test_flag(Flag::Carry) { value | 0x100 } else { value };
                self.update_f(Flag::Carry, temp & 0x01 != 0);
                temp >>= 1;
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles)
            },
            // -- Control Flow
            Instruction::BCC(ref op, cycles) => {
                if !self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BCS(ref op, cycles) => {
                if self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BEQ(ref op, cycles) => {
                if self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BMI(ref op, cycles) => {
                if self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BNE(ref op, cycles) => {
                if !self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BPL(ref op, cycles) => {
                if !self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BVC(ref op, cycles) => {
                if !self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::BVS(ref op, cycles) => {
                if self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles)
            },
            Instruction::JMP(ref op, cycles) => {
                self.pc = op.ea(self);
                self.tick(cycles)
            },
            Instruction::JSR(ref op, cycles) => {
                let pc = self.pc.wrapping_sub(1);
                self.push(((pc >> 8) & 0xff) as u8);
                self.push((pc & 0xff) as u8);
                self.pc = op.ea(self);
                self.tick(cycles)
            },
            Instruction::RTS(cycles) => {
                let address = (self.pop() as u16) | ((self.pop() as u16) << 8);
                self.pc = address.wrapping_add(1);
                self.tick(cycles)
            },
            // -- Flag
            Instruction::CLC(cycles) => {
                self.clear_flag(Flag::Carry);
                self.tick(cycles)
            },
            Instruction::CLD(cycles) => {
                self.clear_flag(Flag::Decimal);
                self.tick(cycles)
            },
            Instruction::CLI(cycles) => {
                self.clear_flag(Flag::IntDisable);
                self.tick(cycles)
            },
            Instruction::CLV(cycles) => {
                self.clear_flag(Flag::Overflow);
                self.tick(cycles)
            },
            Instruction::SEC(cycles) => {
                self.set_flag(Flag::Carry);
                self.tick(cycles)
            },
            Instruction::SED(cycles) => {
                self.set_flag(Flag::Decimal);
                self.tick(cycles)
            },
            Instruction::SEI(cycles) => {
                self.set_flag(Flag::IntDisable);
                self.tick(cycles)
            },
            // -- System
            Instruction::BRK(cycles) => {
                self.interrupt(interrupt::Type::Break);
                self.tick(cycles)
            },
            Instruction::RTI(cycles) => {
                self.p = self.pop();
                self.pc = (self.pop() as u16) | ((self.pop() as u16) << 8);
                self.tick(cycles)
            },
            // -- Misc
            Instruction::NOP(cycles) => {
                self.tick(cycles)
            },
            Instruction::BIT(ref op, cycles) => {
                let value = op.get(self);
                let a = self.a;
                self.update_f(Flag::Negative, value & 0x80 != 0);
                self.update_f(Flag::Overflow, 0x40 & value != 0);
                self.update_f(Flag::Zero, value & a == 0);
                self.tick(cycles)
            },
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0;
        self.p = 0;
        self.io.borrow_mut().reset();
        self.write(0x0000, 0x2f);
        self.write(0x0001, 31);
        self.interrupt(interrupt::Type::Reset);
    }

    #[inline(always)]
    fn tick(&self, elapsed: u8) -> u8 {
        elapsed
    }

    // -- Flag Ops

    #[inline(always)]
    fn clear_flag(&mut self, flag: Flag) {
        self.p &= !(flag as u8);
    }

    #[inline(always)]
    fn set_flag(&mut self, flag: Flag) {
        self.p |= flag as u8;
    }

    #[inline(always)]
    fn test_flag(&self, flag: Flag) -> bool {
        (self.p & (flag as u8)) != 0
    }

    #[inline(always)]
    fn update_f(&mut self, flag: Flag, value: bool) {
        if value {
            self.set_flag(flag);
        } else {
            self.clear_flag(flag);
        }
    }

    fn update_nz(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.set_flag(Flag::Negative);
        } else {
            self.clear_flag(Flag::Negative);
        }
        if value == 0 {
            self.set_flag(Flag::Zero);
        } else {
            self.clear_flag(Flag::Zero);
        }
    }

    // -- Interrupt Ops

    fn interrupt(&mut self, interrupt: interrupt::Type) -> u8 {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::int", "Interrupt {:?}", interrupt);
        }
        let pc = self.pc;
        let p = self.p;
        match interrupt {
            interrupt::Type::Irq => {
                self.push(((pc >> 8) & 0xff) as u8);
                self.push((pc & 0xff) as u8);
                self.push(p & 0xef);
                self.set_flag(Flag::IntDisable);
            },
            interrupt::Type::Nmi => {
                self.push(((pc >> 8) & 0xff) as u8);
                self.push((pc & 0xff) as u8);
                self.push(p & 0xef);
                self.set_flag(Flag::IntDisable);
                self.io.borrow_mut().nmi.reset();
            },
            interrupt::Type::Break => {
                self.push((((pc + 1) >> 8) & 0xff) as u8);
                self.push(((pc + 1) & 0xff) as u8);
                self.push(p | (Flag::Break as u8) | (Flag::Reserved as u8));
                self.set_flag(Flag::IntDisable);
            },
            interrupt::Type::Reset => {},
        }
        self.pc = self.read_word(interrupt.vector());
        7
    }

    // -- Memory Ops

    pub fn fetch_op(&mut self) -> u8 {
        let op = self.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        op
    }

    pub fn fetch_word(&mut self) -> u16 {
        let word = self.read_word(self.pc);
        self.pc = self.pc.wrapping_add(2);
        word
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0001 => {
                let io = self.io.borrow();
                let loram = bit::value(0, io.loram);
                let hiram = bit::value(1, io.hiram);
                let charen = bit::value(2, io.charen);
                let cassette_switch = bit::value(4, io.cassette_switch);
                let cassette_motor = bit::value(5, io.cassette_motor);
                loram | hiram | charen | cassette_switch | cassette_motor
            }
            _ => self.mem.borrow().read(address),
        }
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let low = self.read(address) as u16;
        let high = self.read(address + 1) as u16;
        low | (high << 8)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0001 => {
                {
                    let mut io = self.io.borrow_mut();
                    io.loram = bit::test(value, 0);
                    io.hiram = bit::test(value, 1);
                    io.charen = bit::test(value, 2);
                    io.cassette_motor = bit::test(value, 5);
                }
                self.mem.borrow_mut().switch_banks();
                self.mem.borrow_mut().write(address, value);
            },
            _ => self.mem.borrow_mut().write(address, value),
        }
    }

    // -- Stack Ops

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = 0x0100 + self.sp as u16;
        self.read(addr)
    }

    fn push(&mut self, value: u8) {
        let addr = 0x0100 + self.sp as u16;
        self.write(addr, value);
        self.sp = self.sp.wrapping_sub(1);
    }
}

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x} {:02x} {:02x} {:02x} {}{}{}{}{}{}{}",
               self.a,
               self.x,
               self.y,
               self.sp,
               if (self.p & Flag::Negative as u8) != 0 { "N" } else { "n" },
               if (self.p & Flag::Overflow as u8) != 0 { "V" } else { "v" },
               if (self.p & Flag::Decimal as u8) != 0 { "B" } else { "b" },
               if (self.p & Flag::Decimal as u8) != 0 { "D" } else { "d" },
               if (self.p & Flag::IntDisable as u8) != 0 { "I" } else { "i" },
               if (self.p & Flag::Zero as u8) != 0 { "Z" } else { "z" },
               if (self.p & Flag::Carry as u8) != 0 { "C" } else { "c" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpu::Instruction;
    use cpu::Operand;
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

    fn setup_reg_a(cpu: &mut Cpu, value: u8) {
        cpu.execute_instruction(&Instruction::LDA(Operand::Immediate(value), 1));
    }

    #[test]
    fn execute_adc_80_16() {
        let mut cpu = setup_cpu().unwrap();
        setup_reg_a(&mut cpu, 80);
        cpu.clear_flag(Flag::Carry);
        cpu.execute_instruction(&Instruction::ADC(Operand::Immediate(16), 1));
        assert_eq!(96, cpu.a);
        assert_eq!(false, cpu.test_flag(Flag::Carry));
        assert_eq!(false, cpu.test_flag(Flag::Negative));
        assert_eq!(false, cpu.test_flag(Flag::Overflow));
    }
}
