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
use std::rc::Rc;

use zinc64::core::Cpu;
use zinc64::cpu::{Instruction, Operand};

pub struct Disassembler {
    cpu: Rc<RefCell<Cpu>>,
}

impl Disassembler {

    pub fn new(cpu: Rc<RefCell<Cpu>>) -> Self {
        Self {
            cpu
        }
    }

    pub fn disassemble(&self, address: u16) -> (Instruction, usize) {
        let opcode = self.cpu.borrow().read_debug(address);
        match opcode {
            // BRK
            0x00 => (Instruction::BRK, 1),
            // ORA (Oper,X)
            0x01 => (Instruction::ORA(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // ORA Oper
            0x05 => (Instruction::ORA(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // ASL Oper
            0x06 => (Instruction::ASL(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // PHP
            0x08 => (Instruction::PHP, 1),
            // ORA #Oper
            0x09 => (Instruction::ORA(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // ASL A
            0x0a => (Instruction::ASL(Operand::Accumulator), 1),
            // ORA Oper
            0x0d => (Instruction::ORA(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // ASL Oper
            0x0e => (Instruction::ASL(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BPL
            0x10 => (Instruction::BPL(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // ORA (Oper),Y
            0x11 => (Instruction::ORA(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // ORA Oper,X
            0x15 => (Instruction::ORA(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // ASL Oper,X
            0x16 => (Instruction::ASL(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // CLC
            0x18 => (Instruction::CLC, 1),
            // ORA Oper,Y
            0x19 => (Instruction::ORA(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // ORA Oper,X (NOTE doc lists as 0x10)
            0x1d => (Instruction::ORA(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // ASL Oper,X
            0x1e => (Instruction::ASL(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // JSR Oper
            0x20 => (Instruction::JSR(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // AND (Oper,X)
            0x21 => (Instruction::AND(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // BIT Oper
            0x24 => (Instruction::BIT(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // AND Oper
            0x25 => (Instruction::AND(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // ROL Oper
            0x26 => (Instruction::ROL(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // PLP
            0x28 => (Instruction::PLP, 1),
            // AND #Oper
            0x29 => (Instruction::AND(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // ROL A
            0x2a => (Instruction::ROL(Operand::Accumulator), 1),
            // BIT Oper
            0x2c => (Instruction::BIT(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // AND Oper
            0x2d => (Instruction::AND(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // ROL Oper
            0x2e => (Instruction::ROL(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BMI
            0x30 => (Instruction::BMI(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // AND (Oper),Y
            0x31 => (Instruction::AND(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // AND Oper,X
            0x35 => (Instruction::AND(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // ROL Oper,X
            0x36 => (Instruction::ROL(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // SEC
            0x38 => (Instruction::SEC, 1),
            // AND Oper,Y
            0x39 => (Instruction::AND(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // AND Oper,X
            0x3d => (Instruction::AND(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // ROL Oper,X
            0x3e => (Instruction::ROL(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // RTI (doc lists as 0x4d)
            0x40 => (Instruction::RTI, 1),
            // EOR (Oper,X)
            0x41 => (Instruction::EOR(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // EOR Oper
            0x45 => (Instruction::EOR(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // LSR Oper
            0x46 => (Instruction::LSR(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // PHA
            0x48 => (Instruction::PHA, 1),
            // EOR #Oper
            0x49 => (Instruction::EOR(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // LSR A
            0x4a => (Instruction::LSR(Operand::Accumulator), 1),
            // JMP Oper
            0x4c => (Instruction::JMP(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // EOR Oper (doc lists as 0x40)
            0x4d => (Instruction::EOR(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // LSR Oper
            0x4e => (Instruction::LSR(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BVC
            0x50 => (Instruction::BVC(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // EOR (Oper),Y
            0x51 => (Instruction::EOR(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // EOR Oper,X
            0x55 => (Instruction::EOR(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // LSR Oper,X
            0x56 => (Instruction::LSR(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // CLI
            0x58 => (Instruction::CLI, 1),
            // EOR Oper,Y
            0x59 => (Instruction::EOR(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // EOR Oper,X (doc lists as 0x50)
            0x5d => (Instruction::EOR(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // LSR Oper,X
            0x5e => (Instruction::LSR(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // RTS
            0x60 => (Instruction::RTS, 1),
            // ADC (Oper,X)
            0x61 => (Instruction::ADC(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // ADC Oper
            0x65 => (Instruction::ADC(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // ROR Oper
            0x66 => (Instruction::ROR(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // PLA
            0x68 => (Instruction::PLA, 1),
            // ADC #Oper
            0x69 => (Instruction::ADC(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // ROR A
            0x6a => (Instruction::ROR(Operand::Accumulator), 1),
            // JMP (Oper)
            0x6c => (Instruction::JMP(Operand::Indirect(self.fetch_word(address + 1))), 3),
            // ADC Oper (doc lists as 0x60)
            0x6d => (Instruction::ADC(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // ROR Oper
            0x6e => (Instruction::ROR(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BVS
            0x70 => (Instruction::BVS(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // ADC (Oper),Y
            0x71 => (Instruction::ADC(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // ADC Oper,X
            0x75 => (Instruction::ADC(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // ROR Oper,X
            0x76 => (Instruction::ROR(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // SEI
            0x78 => (Instruction::SEI, 1),
            // ADC Oper,Y
            0x79 => (Instruction::ADC(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // ADC Oper,X (doc lists as 0x70)
            0x7d => (Instruction::ADC(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // ROR Oper,X
            0x7e => (Instruction::ROR(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // STA (Oper,X)
            0x81 => (Instruction::STA(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // STY Oper
            0x84 => (Instruction::STY(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // STA Oper
            0x85 => (Instruction::STA(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // STX Oper
            0x86 => (Instruction::STX(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // DEY
            0x88 => (Instruction::DEY, 1),
            // TXA
            0x8a => (Instruction::TXA, 1),
            // STY Oper
            0x8c => (Instruction::STY(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // STA Oper (doc lists as 0x80)
            0x8d => (Instruction::STA(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // STX Oper
            0x8e => (Instruction::STX(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BCC
            0x90 => (Instruction::BCC(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // STA (Oper),Y
            0x91 => (Instruction::STA(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // STY Oper,X
            0x94 => (Instruction::STY(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // STA Oper,X
            0x95 => (Instruction::STA(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // STX Oper,Y
            0x96 => (Instruction::STX(Operand::ZeroPageY(self.fetch_byte(address + 1))), 2),
            // TYA
            0x98 => (Instruction::TYA, 1),
            // STA Oper,Y
            0x99 => (Instruction::STA(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // TXS
            0x9a => (Instruction::TXS, 1),
            // STA Oper,X (doc lists as 0x90)
            0x9d => (Instruction::STA(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // LDY #Oper
            0xa0 => (Instruction::LDY(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // LDA (Oper,X)
            0xa1 => (Instruction::LDA(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // LDX #Oper
            0xa2 => (Instruction::LDX(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // LDY Oper
            0xa4 => (Instruction::LDY(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // LDA Oper
            0xa5 => (Instruction::LDA(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // LDX Oper
            0xa6 => (Instruction::LDX(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // LAX Oper
            0xa7 => (Instruction::LAX(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // TAY
            0xa8 => (Instruction::TAY, 1),
            // LDA #Oper
            0xa9 => (Instruction::LDA(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // TAX
            0xaa => (Instruction::TAX, 1),
            // LDY Oper
            0xac => (Instruction::LDY(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // LDA Oper
            0xad => (Instruction::LDA(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // LDX Oper
            0xae => (Instruction::LDX(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BCS
            0xb0 => (Instruction::BCS(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // LDA (Oper),Y
            0xb1 => (Instruction::LDA(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // LAX (Oper),Y
            0xb3 => (Instruction::LAX(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // LDY Oper,X
            0xb4 => (Instruction::LDY(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // LDA Oper,X
            0xb5 => (Instruction::LDA(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // LDX Oper,Y
            0xb6 => (Instruction::LDX(Operand::ZeroPageY(self.fetch_byte(address + 1))), 2),
            // CLV
            0xb8 => (Instruction::CLV, 1),
            // LDA Oper,Y
            0xb9 => (Instruction::LDA(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // TSX
            0xba => (Instruction::TSX, 1),
            // LDY Oper,X
            0xbc => (Instruction::LDY(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // LDA Oper,X
            0xbd => (Instruction::LDA(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // LDX Oper,Y
            0xbe => (Instruction::LDX(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // CPY *Oper
            0xc0 => (Instruction::CPY(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // CMP (Oper,X)
            0xc1 => (Instruction::CMP(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // CPY Oper
            0xc4 => (Instruction::CPY(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // CMP Oper
            0xc5 => (Instruction::CMP(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // DEC Oper
            0xc6 => (Instruction::DEC(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // INY
            0xc8 => (Instruction::INY, 1),
            // CMP #Oper
            0xc9 => (Instruction::CMP(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // DEX
            0xca => (Instruction::DEX, 1),
            // AXS
            0xcb => (Instruction::AXS(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // CPY Oper
            0xcc => (Instruction::CPY(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // CMP Oper
            0xcd => (Instruction::CMP(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // DEC Oper
            0xce => (Instruction::DEC(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BNE
            0xd0 => (Instruction::BNE(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // CMP (Oper),Y
            0xd1 => (Instruction::CMP(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // CMP Oper,X
            0xd5 => (Instruction::CMP(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // DEC Oper,X
            0xd6 => (Instruction::DEC(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // CLD
            0xd8 => (Instruction::CLD, 1),
            // CMP Oper,Y
            0xd9 => (Instruction::CMP(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // CMP Oper,X
            0xdd => (Instruction::CMP(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // DEC Oper,X
            0xde => (Instruction::DEC(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // CPX *Oper
            0xe0 => (Instruction::CPX(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // SBC (Oper,X)
            0xe1 => (Instruction::SBC(Operand::IndirectX(self.fetch_byte(address + 1))), 2),
            // CPX Oper
            0xe4 => (Instruction::CPX(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // SBC Oper
            0xe5 => (Instruction::SBC(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // INC Oper
            0xe6 => (Instruction::INC(Operand::ZeroPage(self.fetch_byte(address + 1))), 2),
            // INX
            0xe8 => (Instruction::INX, 1),
            // SBC #Oper
            0xe9 => (Instruction::SBC(Operand::Immediate(self.fetch_byte(address + 1))), 2),
            // NOP
            0xea => (Instruction::NOP, 1),
            // CPX Oper
            0xec => (Instruction::CPX(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // SBC Oper
            0xed => (Instruction::SBC(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // INC Oper
            0xee => (Instruction::INC(Operand::Absolute(self.fetch_word(address + 1))), 3),
            // BEQ
            0xf0 => (Instruction::BEQ(Operand::Relative(self.fetch_byte(address + 1) as i8)), 2),
            // SBC (Oper),Y
            0xf1 => (Instruction::SBC(Operand::IndirectY(self.fetch_byte(address + 1))), 2),
            // SBC Oper,X
            0xf5 => (Instruction::SBC(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // INC Oper,X
            0xf6 => (Instruction::INC(Operand::ZeroPageX(self.fetch_byte(address + 1))), 2),
            // SED
            0xf8 => (Instruction::SED, 1),
            // SBC Oper,Y
            0xf9 => (Instruction::SBC(Operand::AbsoluteY(self.fetch_word(address + 1))), 3),
            // SBC Oper,X
            0xfd => (Instruction::SBC(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // INC Oper,X
            0xfe => (Instruction::INC(Operand::AbsoluteX(self.fetch_word(address + 1))), 3),
            // catch all
            _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, address),
        }
    }

    fn fetch_byte(&self, address: u16) -> u8 {
        self.cpu.borrow().read_debug(address)
    }

    fn fetch_word(&self, address: u16) -> u16 {
        let low = self.cpu.borrow().read_debug(address);
        let high = self.cpu.borrow().read_debug(address + 1);
        ((high as u16) << 8) | low as u16
    }
}
