// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use crate::instruction::{Instruction, Operand};

pub struct Disassembler {
    data: Vec<u8>,
    offset: u16,
}

impl Disassembler {
    pub fn new(data: Vec<u8>, offset: u16) -> Self {
        Self { data, offset }
    }

    pub fn disassemble(&self, address: u16) -> (Instruction, usize) {
        let opcode = self.read_byte(address);
        match opcode {
            0x00 => (Instruction::BRK, 1),
            0x01 => (
                Instruction::ORA(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0x05 => (
                Instruction::ORA(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x06 => (
                Instruction::ASL(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x08 => (Instruction::PHP, 1),
            0x09 => (
                Instruction::ORA(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0x0a => (Instruction::ASL(Operand::Accumulator), 1),
            0x0d => (
                Instruction::ORA(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x0e => (
                Instruction::ASL(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x10 => (
                Instruction::BPL(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0x11 => (
                Instruction::ORA(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0x15 => (
                Instruction::ORA(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x16 => (
                Instruction::ASL(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x18 => (Instruction::CLC, 1),
            0x19 => (
                Instruction::ORA(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0x1d => (
                Instruction::ORA(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x1e => (
                Instruction::ASL(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x20 => (
                Instruction::JSR(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x21 => (
                Instruction::AND(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0x24 => (
                Instruction::BIT(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x25 => (
                Instruction::AND(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x26 => (
                Instruction::ROL(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x28 => (Instruction::PLP, 1),
            0x29 => (
                Instruction::AND(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0x2a => (Instruction::ROL(Operand::Accumulator), 1),
            0x2c => (
                Instruction::BIT(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x2d => (
                Instruction::AND(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x2e => (
                Instruction::ROL(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x30 => (
                Instruction::BMI(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0x31 => (
                Instruction::AND(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0x35 => (
                Instruction::AND(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x36 => (
                Instruction::ROL(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x38 => (Instruction::SEC, 1),
            0x39 => (
                Instruction::AND(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0x3d => (
                Instruction::AND(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x3e => (
                Instruction::ROL(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x40 => (Instruction::RTI, 1),
            0x41 => (
                Instruction::EOR(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0x45 => (
                Instruction::EOR(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x46 => (
                Instruction::LSR(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x48 => (Instruction::PHA, 1),
            0x49 => (
                Instruction::EOR(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0x4a => (Instruction::LSR(Operand::Accumulator), 1),
            0x4c => (
                Instruction::JMP(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x4d => (
                Instruction::EOR(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x4e => (
                Instruction::LSR(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x50 => (
                Instruction::BVC(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0x51 => (
                Instruction::EOR(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0x55 => (
                Instruction::EOR(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x56 => (
                Instruction::LSR(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x58 => (Instruction::CLI, 1),
            0x59 => (
                Instruction::EOR(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0x5d => (
                Instruction::EOR(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x5e => (
                Instruction::LSR(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x60 => (Instruction::RTS, 1),
            0x61 => (
                Instruction::ADC(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0x65 => (
                Instruction::ADC(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x66 => (
                Instruction::ROR(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x68 => (Instruction::PLA, 1),
            0x69 => (
                Instruction::ADC(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0x6a => (Instruction::ROR(Operand::Accumulator), 1),
            0x6c => (
                Instruction::JMP(Operand::Indirect(self.read_word(address + 1))),
                3,
            ),
            0x6d => (
                Instruction::ADC(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x6e => (
                Instruction::ROR(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x70 => (
                Instruction::BVS(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0x71 => (
                Instruction::ADC(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0x75 => (
                Instruction::ADC(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x76 => (
                Instruction::ROR(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x78 => (Instruction::SEI, 1),
            0x79 => (
                Instruction::ADC(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0x7d => (
                Instruction::ADC(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x7e => (
                Instruction::ROR(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0x81 => (
                Instruction::STA(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0x84 => (
                Instruction::STY(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x85 => (
                Instruction::STA(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x86 => (
                Instruction::STX(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0x88 => (Instruction::DEY, 1),
            0x8a => (Instruction::TXA, 1),
            0x8c => (
                Instruction::STY(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x8d => (
                Instruction::STA(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x8e => (
                Instruction::STX(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0x90 => (
                Instruction::BCC(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0x91 => (
                Instruction::STA(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0x94 => (
                Instruction::STY(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x95 => (
                Instruction::STA(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0x96 => (
                Instruction::STX(Operand::ZeroPageY(self.read_byte(address + 1))),
                2,
            ),
            0x98 => (Instruction::TYA, 1),
            0x99 => (
                Instruction::STA(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0x9a => (Instruction::TXS, 1),
            0x9d => (
                Instruction::STA(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xa0 => (
                Instruction::LDY(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xa1 => (
                Instruction::LDA(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0xa2 => (
                Instruction::LDX(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xa4 => (
                Instruction::LDY(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xa5 => (
                Instruction::LDA(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xa6 => (
                Instruction::LDX(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xa7 => (
                Instruction::LAX(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xa8 => (Instruction::TAY, 1),
            0xa9 => (
                Instruction::LDA(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xaa => (Instruction::TAX, 1),
            0xac => (
                Instruction::LDY(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xad => (
                Instruction::LDA(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xae => (
                Instruction::LDX(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xb0 => (
                Instruction::BCS(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0xb1 => (
                Instruction::LDA(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0xb3 => (
                Instruction::LAX(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0xb4 => (
                Instruction::LDY(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xb5 => (
                Instruction::LDA(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xb6 => (
                Instruction::LDX(Operand::ZeroPageY(self.read_byte(address + 1))),
                2,
            ),
            0xb8 => (Instruction::CLV, 1),
            0xb9 => (
                Instruction::LDA(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0xba => (Instruction::TSX, 1),
            0xbc => (
                Instruction::LDY(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xbd => (
                Instruction::LDA(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xbe => (
                Instruction::LDX(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0xc0 => (
                Instruction::CPY(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xc1 => (
                Instruction::CMP(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0xc4 => (
                Instruction::CPY(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xc5 => (
                Instruction::CMP(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xc6 => (
                Instruction::DEC(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xc8 => (Instruction::INY, 1),
            0xc9 => (
                Instruction::CMP(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xca => (Instruction::DEX, 1),
            0xcb => (
                Instruction::AXS(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xcc => (
                Instruction::CPY(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xcd => (
                Instruction::CMP(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xce => (
                Instruction::DEC(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xd0 => (
                Instruction::BNE(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0xd1 => (
                Instruction::CMP(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0xd5 => (
                Instruction::CMP(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xd6 => (
                Instruction::DEC(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xd8 => (Instruction::CLD, 1),
            0xd9 => (
                Instruction::CMP(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0xdd => (
                Instruction::CMP(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xde => (
                Instruction::DEC(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xe0 => (
                Instruction::CPX(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xe1 => (
                Instruction::SBC(Operand::IndirectX(self.read_byte(address + 1))),
                2,
            ),
            0xe4 => (
                Instruction::CPX(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xe5 => (
                Instruction::SBC(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xe6 => (
                Instruction::INC(Operand::ZeroPage(self.read_byte(address + 1))),
                2,
            ),
            0xe8 => (Instruction::INX, 1),
            0xe9 => (
                Instruction::SBC(Operand::Immediate(self.read_byte(address + 1))),
                2,
            ),
            0xea => (Instruction::NOP, 1),
            0xec => (
                Instruction::CPX(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xed => (
                Instruction::SBC(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xee => (
                Instruction::INC(Operand::Absolute(self.read_word(address + 1))),
                3,
            ),
            0xf0 => (
                Instruction::BEQ(Operand::Relative(self.read_byte(address + 1) as i8)),
                2,
            ),
            0xf1 => (
                Instruction::SBC(Operand::IndirectY(self.read_byte(address + 1))),
                2,
            ),
            0xf5 => (
                Instruction::SBC(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xf6 => (
                Instruction::INC(Operand::ZeroPageX(self.read_byte(address + 1))),
                2,
            ),
            0xf8 => (Instruction::SED, 1),
            0xf9 => (
                Instruction::SBC(Operand::AbsoluteY(self.read_word(address + 1))),
                3,
            ),
            0xfd => (
                Instruction::SBC(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            0xfe => (
                Instruction::INC(Operand::AbsoluteX(self.read_word(address + 1))),
                3,
            ),
            _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, address),
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.data[(address - self.offset) as usize]
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let low = self.read_byte(address);
        let high = self.read_byte(address + 1);
        ((high as u16) << 8) | low as u16
    }
}
