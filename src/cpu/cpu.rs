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

use cpu::Instruction;
use mem::{Addressable, Memory};

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

#[allow(dead_code)]
pub struct Cpu {
    mem: Rc<RefCell<Memory>>,
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    p: u8,
    irq_line: bool,
    nmi_line: bool,
    cycles: u32,
}

#[allow(dead_code)]
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

#[derive(Debug, PartialEq)]
pub enum Interrupt {
    Break = 1 << 0,
    Irq = 1 << 1,
    Nmi = 1 << 2,
    Reset = 1 << 3,
}

impl Interrupt {
    pub fn vector(&self) -> u16 {
        match *self {
            Interrupt::Break => InterruptVector::Irq as u16,
            Interrupt::Irq => InterruptVector::Irq as u16,
            Interrupt::Nmi => InterruptVector::Nmi as u16,
            Interrupt::Reset => InterruptVector::Reset as u16,
        }
    }
}

enum InterruptVector {
    Nmi = 0xfffa,
    Reset = 0xfffc,
    Irq = 0xfffe,
}

impl Cpu {
    pub fn new(mem: Rc<RefCell<Memory>>) -> Cpu {
        Cpu {
            mem: mem,
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            sp: 0,
            p: 0,
            irq_line: false,
            nmi_line: false,
            cycles: 0,
        }
    }

    pub fn get_a(&self) -> u8 { self.a }
    pub fn get_x(&self) -> u8 { self.x }
    pub fn get_y(&self) -> u8 { self.y }
    pub fn get_pc(&self) -> u16 { self.pc }
    pub fn get_cycles(&self) -> u32 { self.cycles }

    pub fn set_a(&mut self, value: u8) { self.a = value; }
    pub fn set_x(&mut self, value: u8) { self.x = value; }
    pub fn set_y(&mut self, value: u8) { self.y = value; }
    pub fn set_pc(&mut self, address: u16) { self.pc = address; }
    pub fn set_irq(&mut self) { self.irq_line = true; }
    pub fn set_nmi(&mut self) { self.nmi_line = true; }

    #[allow(dead_code)]
    fn dump_registers(&self) {
        println!("A: {:x} X: {:x} Y: {:x} S: {:x} P: {:x}",
                self.a, self.x, self.y, self.sp, self.p);
    }

    pub fn execute(&mut self) {
        if self.nmi_line {
            self.interrupt(Interrupt::Nmi);
        } else if self.irq_line && !self.test_flag(Flag::IntDisable) {
            self.interrupt(Interrupt::Irq);
        }
        let pc = self.pc;
        let opcode = self.fetch_op();
        let op = Instruction::decode(self, opcode);
        self.dump_registers();
        println!("exec 0x{:x}: {:?}", pc, op);
        self.execute_instruction(&op);
    }

    fn execute_instruction(&mut self, instr: &Instruction) {
        match *instr {
            // -- Data Movement
            Instruction::LDA(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.a = value;
                self.tick(cycles);
            },
            Instruction::LDX(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.x = value;
                self.tick(cycles);
            },
            Instruction::LDY(ref op, cycles) => {
                let value = op.get(self);
                self.update_nz(value);
                self.y = value;
                self.tick(cycles);
            },
            Instruction::STA(ref op, cycles) => {
                let value = self.a;
                op.set(self, value);
                self.tick(cycles);
            },
            Instruction::STX(ref op, cycles) => {
                let value = self.x;
                op.set(self, value);
                self.tick(cycles);
            },
            Instruction::STY(ref op, cycles) => {
                let value = self.y;
                op.set(self, value);
                self.tick(cycles);
            },
            Instruction::TAX(cycles) => {
                let value = self.a;
                self.update_nz(value);
                self.x = value;
                self.tick(cycles);
            },
            Instruction::TAY(cycles) => {
                let value = self.a;
                self.update_nz(value);
                self.y = value;
                self.tick(cycles);
            },
            Instruction::TSX(cycles) => {
                let value = self.sp;
                self.update_nz(value);
                self.x = value;
                self.tick(cycles);
            },
            Instruction::TXA(cycles) => {
                let value = self.x;
                self.update_nz(value);
                self.a = value;
                self.tick(cycles);
            },
            Instruction::TXS(cycles) => {
                let value = self.x;
                // NOTE do not set nz
                self.sp = value;
                self.tick(cycles);
            },
            Instruction::TYA(cycles) => {
                let value = self.y;
                self.update_nz(value);
                self.a = value;
                self.tick(cycles);
            },
            // -- Stack
            Instruction::PHA(cycles) => {
                let value = self.a;
                self.push(value);
                self.tick(cycles);
            },
            Instruction::PHP(cycles) => {
                // NOTE undocumented behavior
                let value = self.p | (Flag::Break as u8) | (Flag::Reserved as u8);
                self.push(value);
                self.tick(cycles);
            },
            Instruction::PLA(cycles) => {
                let value = self.pop();
                self.update_nz(value);
                self.a = value;
                self.tick(cycles);
            },
            Instruction::PLP(cycles) => {
                let value = self.pop();
                self.p = value;
                self.tick(cycles);
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
                self.tick(cycles);
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
                self.tick(cycles);
            },
            Instruction::DEC(ref op, cycles) => {
                let result = op.get(self).wrapping_sub(1);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            Instruction::DEX(cycles) => {
                let result = self.x.wrapping_sub(1);
                self.update_nz(result);
                self.x = result;
                self.tick(cycles);
            },
            Instruction::DEY(cycles) => {
                let result = self.y.wrapping_sub(1);
                self.update_nz(result);
                self.y = result;
                self.tick(cycles);
            },
            Instruction::INC(ref op, cycles) => {
                let result = op.get(self).wrapping_add(1);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            Instruction::INX(cycles) => {
                let result = self.x.wrapping_add(1);
                self.update_nz(result);
                self.x = result;
                self.tick(cycles);
            },
            Instruction::INY(cycles) => {
                let result = self.y.wrapping_add(1);
                self.update_nz(result);
                self.y = result;
                self.tick(cycles);
            },
            Instruction::CMP(ref op, cycles) => {
                let result = (self.a as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles);
            },
            Instruction::CPX(ref op, cycles) => {
                let result = (self.x as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles);
            },
            Instruction::CPY(ref op, cycles) => {
                let result = (self.y as u16).wrapping_sub(op.get(self) as u16);
                self.update_f(Flag::Carry, result < 0x100);
                self.update_nz((result & 0xff) as u8);
                self.tick(cycles);
            },
            // -- Logical
            Instruction::AND(ref op, cycles) => {
                let result = op.get(self) & self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles);
            },
            Instruction::EOR(ref op, cycles) => {
                let result = op.get(self) ^ self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles);
            },
            Instruction::ORA(ref op, cycles) => {
                let result = op.get(self) | self.a;
                self.update_nz(result);
                self.a = result;
                self.tick(cycles);
            },
            // -- Shift and Rotate
            Instruction::ASL(ref op, cycles) => {
                let value = op.get(self);
                let result = value << 1;
                self.update_f(Flag::Carry, (value & 0x80) != 0);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            Instruction::LSR(ref op, cycles) => {
                let value = op.get(self);
                let result = value >> 1;
                self.update_f(Flag::Carry, (value & 0x01) != 0);
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            Instruction::ROL(ref op, cycles) => {
                let value = op.get(self);
                let mut temp = (value as u16) << 1;
                if self.test_flag(Flag::Carry) { temp |= 0x01 };
                self.update_f(Flag::Carry, temp > 0xff);
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            Instruction::ROR(ref op, cycles) => {
                let value = op.get(self) as u16;
                let mut temp = if self.test_flag(Flag::Carry) { value | 0x100 } else { value };
                self.update_f(Flag::Carry, temp & 0x01 != 0);
                temp >>= 1;
                let result = (temp & 0xff) as u8;
                self.update_nz(result);
                op.set(self, result);
                self.tick(cycles);
            },
            // -- Control Flow
            Instruction::BCC(ref op, cycles) => {
                if !self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BCS(ref op, cycles) => {
                if self.test_flag(Flag::Carry) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BEQ(ref op, cycles) => {
                if self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BMI(ref op, cycles) => {
                if self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BNE(ref op, cycles) => {
                if !self.test_flag(Flag::Zero) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BPL(ref op, cycles) => {
                if !self.test_flag(Flag::Negative) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BVC(ref op, cycles) => {
                if !self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::BVS(ref op, cycles) => {
                if self.test_flag(Flag::Overflow) {
                    self.pc = op.ea(self);
                }
                self.tick(cycles);
            },
            Instruction::JMP(ref op, cycles) => {
                self.pc = op.ea(self);
                self.tick(cycles);
            },
            Instruction::JSR(ref op, cycles) => {
                let pc = self.pc.wrapping_sub(1);
                self.push(((pc >> 8) & 0xff) as u8);
                self.push((pc & 0xff) as u8);
                self.pc = op.ea(self);
                self.tick(cycles);
            },
            Instruction::RTS(cycles) => {
                let address = (self.pop() as u16) | ((self.pop() as u16) << 8);
                self.pc = address.wrapping_add(1);
                self.tick(cycles);
            },
            // -- Flag
            Instruction::CLC(cycles) => {
                self.clear_flag(Flag::Carry);
                self.tick(cycles);
            },
            Instruction::CLD(cycles) => {
                self.clear_flag(Flag::Decimal);
                self.tick(cycles);
            },
            Instruction::CLI(cycles) => {
                self.clear_flag(Flag::IntDisable);
                self.tick(cycles);
            },
            Instruction::CLV(cycles) => {
                self.clear_flag(Flag::Overflow);
                self.tick(cycles);
            },
            Instruction::SEC(cycles) => {
                self.set_flag(Flag::Carry);
                self.tick(cycles);
            },
            Instruction::SED(cycles) => {
                self.set_flag(Flag::Decimal);
                self.tick(cycles);
            },
            Instruction::SEI(cycles) => {
                self.set_flag(Flag::IntDisable);
                self.tick(cycles);
            },
            // -- System
            Instruction::BRK(cycles) => {
                self.interrupt(Interrupt::Break);
                self.tick(cycles);
            },
            Instruction::RTI(cycles) => {
                self.p = self.pop();
                self.pc = (self.pop() as u16) | ((self.pop() as u16) << 8);
                self.tick(cycles);
            },
            // -- Misc
            Instruction::NOP(cycles) => {
                self.tick(cycles);
            },
            Instruction::BIT(ref op, cycles) => {
                let value = op.get(self);
                let a = self.a;
                self.update_f(Flag::Negative, value & 0x80 != 0);
                self.update_f(Flag::Overflow, 0x40 & value != 0);
                self.update_f(Flag::Zero, value & a == 0);
                self.tick(cycles);
            },
        }
    }

    fn tick(&mut self, elapsed: u8) {
        self.cycles.wrapping_add(elapsed as u32);
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

    fn interrupt(&mut self, interrupt: Interrupt) -> u8 {
        if interrupt != Interrupt::Reset {
            let pc = if interrupt == Interrupt::Break {
                self.pc + 1
            } else {
                self.pc
            };
            self.push(((pc >> 8) & 0xff) as u8);
            self.push((pc & 0xff) as u8);
            let sr = if interrupt == Interrupt::Break {
                self.p | (Flag::Break as u8) | (Flag::Reserved as u8)
            } else {
                self.p
            };
            self.push(sr);
        }
            self.set_flag(Flag::IntDisable);
        self.pc = self.read_word(interrupt.vector());
        if interrupt == Interrupt::Nmi && self.nmi_line {
            self.nmi_line = false;
        }
        if interrupt == Interrupt::Irq && self.irq_line {
            self.irq_line = false;
        }
        6
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
        self.mem.borrow().read(address)
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let low = self.read(address) as u16;
        let high = self.read(address + 1) as u16;
        low | (high << 8)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0001 => {
                self.mem.borrow_mut().switch_banks(value);
                self.mem.borrow_mut().write(address, value); // FIXME
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

    // --

    #[allow(dead_code)]
    pub fn reset(&mut self) -> u8 {
        self.pc = 0;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0;
        self.p = 0;
        self.irq_line = false;
        self.nmi_line = false;
        self.cycles = 0;
        self.interrupt(Interrupt::Reset)
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
