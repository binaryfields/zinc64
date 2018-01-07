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

use std::fmt;

use core::{Cpu, TickFn};

use super::Cpu6510;
use super::operand::Operand;

// Spec: MCS6510 MICROPROCESSOR INSTRUCTION SET p.232
// Design:
//   UAE uses I_dec_tab_entry with instr_func and instr_params(src,dst). We want something
//   similar to I_dec_tab_entry.
//   C64 instructions have zero or one operand so we encode them using Instruction(Operand)
//   variants.

// Categories: Asm One Manual Sec 10.1
pub enum Instruction {
    // Data Movement (16)
    LDA(Operand, u8),
    LDX(Operand, u8),
    LDY(Operand, u8),
    PHA(u8),
    PHP(u8),
    PLA(u8),
    PLP(u8),
    STA(Operand, u8),
    STX(Operand, u8),
    STY(Operand, u8),
    TAX(u8),
    TAY(u8),
    TSX(u8),
    TXA(u8),
    TXS(u8),
    TYA(u8),
    // Arithmetic (11)
    ADC(Operand, u8),
    SBC(Operand, u8),
    CMP(Operand, u8),
    CPX(Operand, u8),
    CPY(Operand, u8),
    DEC(Operand, u8),
    DEX(u8),
    DEY(u8),
    INC(Operand, u8),
    INX(u8),
    INY(u8),
    // Logical (3)
    AND(Operand, u8),
    EOR(Operand, u8),
    ORA(Operand, u8),
    // Shift and Rotate (4)
    ASL(Operand, u8),
    LSR(Operand, u8),
    ROL(Operand, u8),
    ROR(Operand, u8),
    // Control Flow (11)
    BCC(Operand, u8),
    BCS(Operand, u8),
    BEQ(Operand, u8),
    BMI(Operand, u8),
    BNE(Operand, u8),
    BPL(Operand, u8),
    BVC(Operand, u8),
    BVS(Operand, u8),
    JMP(Operand, u8),
    JSR(Operand, u8),
    RTS(u8),
    // Misc (11)
    BIT(Operand, u8),
    BRK(u8),
    CLC(u8),
    CLD(u8),
    CLI(u8),
    CLV(u8),
    NOP(u8),
    SEC(u8),
    SED(u8),
    SEI(u8),
    RTI(u8),
}

impl Instruction {
    #[inline]
    pub fn decode(cpu: &mut Cpu6510, opcode: u8, tick_fn: &TickFn) -> Instruction {
        match opcode {
            // BRK
            0x00 => Instruction::BRK(7),
            // ORA (Oper,X)
            0x01 => Instruction::ORA(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // ORA Oper
            0x05 => Instruction::ORA(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // ASL Oper
            0x06 => Instruction::ASL(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // PHP
            0x08 => Instruction::PHP(3),
            // ORA #Oper
            0x09 => Instruction::ORA(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // ASL A
            0x0a => Instruction::ASL(Operand::Accumulator, 2),
            // ORA Oper
            0x0d => Instruction::ORA(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // ASL Oper
            0x0e => Instruction::ASL(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BPL
            0x10 => Instruction::BPL(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // ORA (Oper),Y
            0x11 => Instruction::ORA(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // ORA Oper,X
            0x15 => Instruction::ORA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // ASL Oper,X
            0x16 => Instruction::ASL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // CLC
            0x18 => Instruction::CLC(2),
            // ORA Oper,Y
            0x19 => Instruction::ORA(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // ORA Oper,X (NOTE doc lists as 0x10)
            0x1d => Instruction::ORA(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // ASL Oper,X
            0x1e => Instruction::ASL(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // JSR Oper
            0x20 => Instruction::JSR(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // AND (Oper,X)
            0x21 => Instruction::AND(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // BIT Oper
            0x24 => Instruction::BIT(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // AND Oper
            0x25 => Instruction::AND(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // ROL Oper
            0x26 => Instruction::ROL(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // PLP
            0x28 => Instruction::PLP(4),
            // AND #Oper
            0x29 => Instruction::AND(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // ROL A
            0x2a => Instruction::ROL(Operand::Accumulator, 2),
            // BIT Oper
            0x2c => Instruction::BIT(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // AND Oper
            0x2d => Instruction::AND(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // ROL Oper
            0x2e => Instruction::ROL(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BMI
            0x30 => Instruction::BMI(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // AND (Oper),Y
            0x31 => Instruction::AND(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // AND Oper,X
            0x35 => Instruction::AND(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // ROL Oper,X
            0x36 => Instruction::ROL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // SEC
            0x38 => Instruction::SEC(2),
            // AND Oper,Y
            0x39 => Instruction::AND(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // AND Oper,X
            0x3d => Instruction::AND(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // ROL Oper,X
            0x3e => Instruction::ROL(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // RTI (doc lists as 0x4d)
            0x40 => Instruction::RTI(6),
            // EOR (Oper,X)
            0x41 => Instruction::EOR(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // EOR Oper
            0x45 => Instruction::EOR(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // LSR Oper
            0x46 => Instruction::LSR(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // PHA
            0x48 => Instruction::PHA(3),
            // EOR #Oper
            0x49 => Instruction::EOR(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // LSR A
            0x4a => Instruction::LSR(Operand::Accumulator, 2),
            // JMP Oper
            0x4c => Instruction::JMP(Operand::Absolute(cpu.fetch_word(tick_fn)), 3),
            // EOR Oper (doc lists as 0x40)
            0x4d => Instruction::EOR(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // LSR Oper
            0x4e => Instruction::LSR(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BVC
            0x50 => Instruction::BVC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // EOR (Oper),Y
            0x51 => Instruction::EOR(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // EOR Oper,X
            0x55 => Instruction::EOR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // LSR Oper,X
            0x56 => Instruction::LSR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // CLI
            0x58 => Instruction::CLI(2),
            // EOR Oper,Y
            0x59 => Instruction::EOR(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // EOR Oper,X (doc lists as 0x50)
            0x5d => Instruction::EOR(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // LSR Oper,X
            0x5e => Instruction::LSR(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // RTS
            0x60 => Instruction::RTS(6),
            // ADC (Oper,X)
            0x61 => Instruction::ADC(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // ADC Oper
            0x65 => Instruction::ADC(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // ROR Oper
            0x66 => Instruction::ROR(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // PLA
            0x68 => Instruction::PLA(4),
            // ADC #Oper
            0x69 => Instruction::ADC(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // ROR A
            0x6a => Instruction::ROR(Operand::Accumulator, 2),
            // JMP (Oper)
            0x6c => Instruction::JMP(Operand::Indirect(cpu.fetch_word(tick_fn)), 5),
            // ADC Oper (doc lists as 0x60)
            0x6d => Instruction::ADC(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // ROR Oper
            0x6e => Instruction::ROR(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BVS
            0x70 => Instruction::BVS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // ADC (Oper),Y
            0x71 => Instruction::ADC(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // ADC Oper,X
            0x75 => Instruction::ADC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // ROR Oper,X
            0x76 => Instruction::ROR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // SEI
            0x78 => Instruction::SEI(2),
            // ADC Oper,Y
            0x79 => Instruction::ADC(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // ADC Oper,X (doc lists as 0x70)
            0x7d => Instruction::ADC(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // ROR Oper,X
            0x7e => Instruction::ROR(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // STA (Oper,X)
            0x81 => Instruction::STA(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // STY Oper
            0x84 => Instruction::STY(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // STA Oper
            0x85 => Instruction::STA(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // STX Oper
            0x86 => Instruction::STX(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // DEY
            0x88 => Instruction::DEY(2),
            // TXA
            0x8a => Instruction::TXA(2),
            // STY Oper
            0x8c => Instruction::STY(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // STA Oper (doc lists as 0x80)
            0x8d => Instruction::STA(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // STX Oper
            0x8e => Instruction::STX(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // BCC
            0x90 => Instruction::BCC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // STA (Oper),Y
            0x91 => Instruction::STA(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 6),
            // STY Oper,X FIXME rmw 4
            0x94 => Instruction::STY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 3),
            // STA Oper,X FIXME rmw 4
            0x95 => Instruction::STA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 3),
            // STX Oper,Y
            0x96 => Instruction::STX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn)), 4),
            // TYA
            0x98 => Instruction::TYA(2),
            // STA Oper,Y
            0x99 => Instruction::STA(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 5),
            // TXS
            0x9a => Instruction::TXS(2),
            // STA Oper,X (doc lists as 0x90)
            0x9d => Instruction::STA(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 5),
            // LDY #Oper
            0xa0 => Instruction::LDY(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // LDA (Oper,X)
            0xa1 => Instruction::LDA(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6), //?
            // LDX #Oper
            0xa2 => Instruction::LDX(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // LDY Oper
            0xa4 => Instruction::LDY(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // LDA Oper
            0xa5 => Instruction::LDA(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // LDX Oper
            0xa6 => Instruction::LDX(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // TAY
            0xa8 => Instruction::TAY(2),
            // LDA #Oper
            0xa9 => Instruction::LDA(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // TAX
            0xaa => Instruction::TAX(2),
            // LDY Oper
            0xac => Instruction::LDY(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // LDA Oper
            0xad => Instruction::LDA(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // LDX Oper
            0xae => Instruction::LDX(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // BCS
            0xb0 => Instruction::BCS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // LDA (Oper),Y
            0xb1 => Instruction::LDA(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // LDY Oper,X
            0xb4 => Instruction::LDY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // LDA Oper,X
            0xb5 => Instruction::LDA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // LDX Oper,Y
            0xb6 => Instruction::LDX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn)), 4),
            // CLV
            0xb8 => Instruction::CLV(2),
            // LDA Oper,Y
            0xb9 => Instruction::LDA(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // TSX
            0xba => Instruction::TSX(2),
            // LDY Oper,X
            0xbc => Instruction::LDY(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // LDA Oper,X
            0xbd => Instruction::LDA(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // LDX Oper,Y
            0xbe => Instruction::LDX(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // CPY *Oper
            0xc0 => Instruction::CPY(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // CMP (Oper,X)
            0xc1 => Instruction::CMP(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // CPY Oper
            0xc4 => Instruction::CPY(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // CMP Oper
            0xc5 => Instruction::CMP(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // DEC Oper
            0xc6 => Instruction::DEC(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // INY
            0xc8 => Instruction::INY(2),
            // CMP #Oper
            0xc9 => Instruction::CMP(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // DEX
            0xca => Instruction::DEX(2),
            // CPY Oper
            0xcc => Instruction::CPY(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // CMP Oper
            0xcd => Instruction::CMP(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // DEC Oper
            0xce => Instruction::DEC(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BNE
            0xd0 => Instruction::BNE(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // CMP (Oper),Y
            0xd1 => Instruction::CMP(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // CMP Oper,X
            0xd5 => Instruction::CMP(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // DEC Oper,X
            0xd6 => Instruction::DEC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // CLD
            0xd8 => Instruction::CLD(2),
            // CMP Oper,Y
            0xd9 => Instruction::CMP(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // CMP Oper,X
            0xdd => Instruction::CMP(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // DEC Oper,X
            0xde => Instruction::DEC(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // CPX *Oper
            0xe0 => Instruction::CPX(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // SBC (Oper,X)
            0xe1 => Instruction::SBC(Operand::IndirectX(cpu.fetch_byte(tick_fn)), 6),
            // CPX Oper
            0xe4 => Instruction::CPX(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // SBC Oper
            0xe5 => Instruction::SBC(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 3),
            // INC Oper
            0xe6 => Instruction::INC(Operand::ZeroPage(cpu.fetch_byte(tick_fn)), 5),
            // INX
            0xe8 => Instruction::INX(2),
            // SBC #Oper
            0xe9 => Instruction::SBC(Operand::Immediate(cpu.fetch_byte(tick_fn)), 2),
            // NOP
            0xea => Instruction::NOP(2),
            // CPX Oper
            0xec => Instruction::CPX(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // SBC Oper
            0xed => Instruction::SBC(Operand::Absolute(cpu.fetch_word(tick_fn)), 4),
            // INC Oper
            0xee => Instruction::INC(Operand::Absolute(cpu.fetch_word(tick_fn)), 6),
            // BEQ
            0xf0 => Instruction::BEQ(Operand::Relative(cpu.fetch_byte(tick_fn) as i8), 2),
            // SBC (Oper),Y
            0xf1 => Instruction::SBC(Operand::IndirectY(cpu.fetch_byte(tick_fn)), 5),
            // SBC Oper,X
            0xf5 => Instruction::SBC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 4),
            // INC Oper,X
            0xf6 => Instruction::INC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)), 6),
            // SED
            0xf8 => Instruction::SED(2),
            // SBC Oper,Y
            0xf9 => Instruction::SBC(Operand::AbsoluteY(cpu.fetch_word(tick_fn)), 4),
            // SBC Oper,X
            0xfd => Instruction::SBC(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 4),
            // INC Oper,X
            0xfe => Instruction::INC(Operand::AbsoluteX(cpu.fetch_word(tick_fn)), 7),
            // catch all
            _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, cpu.get_pc()),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Data Movement
            &Instruction::LDA(ref operand, _) => write!(f, "lda {}", operand),
            &Instruction::LDX(ref operand, _) => write!(f, "ldx {}", operand),
            &Instruction::LDY(ref operand, _) => write!(f, "ldy {}", operand),
            &Instruction::PHA(_) => write!(f, "pha"),
            &Instruction::PHP(_) => write!(f, "php"),
            &Instruction::PLA(_) => write!(f, "pla"),
            &Instruction::PLP(_) => write!(f, "plp"),
            &Instruction::STA(ref operand, _) => write!(f, "sta {}", operand),
            &Instruction::STX(ref operand, _) => write!(f, "stx {}", operand),
            &Instruction::STY(ref operand, _) => write!(f, "sty {}", operand),
            &Instruction::TAX(_) => write!(f, "tax"),
            &Instruction::TAY(_) => write!(f, "tay"),
            &Instruction::TSX(_) => write!(f, "tsx"),
            &Instruction::TXA(_) => write!(f, "txa"),
            &Instruction::TXS(_) => write!(f, "txs"),
            &Instruction::TYA(_) => write!(f, "tya"),
            // Arithmetic
            &Instruction::ADC(ref operand, _) => write!(f, "adc {}", operand),
            &Instruction::SBC(ref operand, _) => write!(f, "sbc {}", operand),
            &Instruction::CMP(ref operand, _) => write!(f, "cmp {}", operand),
            &Instruction::CPX(ref operand, _) => write!(f, "cpx {}", operand),
            &Instruction::CPY(ref operand, _) => write!(f, "cpy {}", operand),
            &Instruction::DEC(ref operand, _) => write!(f, "dec {}", operand),
            &Instruction::DEX(_) => write!(f, "dex"),
            &Instruction::DEY(_) => write!(f, "dey"),
            &Instruction::INC(ref operand, _) => write!(f, "inc {}", operand),
            &Instruction::INX(_) => write!(f, "inx"),
            &Instruction::INY(_) => write!(f, "iny"),
            // Logical
            &Instruction::AND(ref operand, _) => write!(f, "and {}", operand),
            &Instruction::EOR(ref operand, _) => write!(f, "eor {}", operand),
            &Instruction::ORA(ref operand, _) => write!(f, "ora {}", operand),
            // Shift and Rotate
            &Instruction::ASL(ref operand, _) => write!(f, "asl {}", operand),
            &Instruction::LSR(ref operand, _) => write!(f, "lsr {}", operand),
            &Instruction::ROL(ref operand, _) => write!(f, "rol {}", operand),
            &Instruction::ROR(ref operand, _) => write!(f, "ror {}", operand),
            // Control Flow
            &Instruction::BCC(ref operand, _) => write!(f, "bcc {}", operand),
            &Instruction::BCS(ref operand, _) => write!(f, "bcs {}", operand),
            &Instruction::BEQ(ref operand, _) => write!(f, "beq {}", operand),
            &Instruction::BMI(ref operand, _) => write!(f, "bmi {}", operand),
            &Instruction::BNE(ref operand, _) => write!(f, "bne {}", operand),
            &Instruction::BPL(ref operand, _) => write!(f, "bpl {}", operand),
            &Instruction::BVC(ref operand, _) => write!(f, "bvc {}", operand),
            &Instruction::BVS(ref operand, _) => write!(f, "bvs {}", operand),
            &Instruction::JMP(ref operand, _) => write!(f, "jmp {}", operand),
            &Instruction::JSR(ref operand, _) => write!(f, "jsr {}", operand),
            &Instruction::RTS(_) => write!(f, "rts"),
            // Misc
            &Instruction::BIT(ref operand, _) => write!(f, "bit {}", operand),
            &Instruction::BRK(_) => write!(f, "brk"),
            &Instruction::CLC(_) => write!(f, "clc"),
            &Instruction::CLD(_) => write!(f, "cld"),
            &Instruction::CLI(_) => write!(f, "cli"),
            &Instruction::CLV(_) => write!(f, "clv"),
            &Instruction::NOP(_) => write!(f, "nop"),
            &Instruction::SEC(_) => write!(f, "sec"),
            &Instruction::SED(_) => write!(f, "sed"),
            &Instruction::SEI(_) => write!(f, "sei"),
            &Instruction::RTI(_) => write!(f, "rti"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{IoPort, IrqLine, MemoryController, Ram};
    use std::cell::RefCell;
    use std::rc::Rc;

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
    fn decode_brk() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0x00, &tick_fn) {
            Instruction::BRK(_) => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }

    #[test]
    fn decode_lda_absolute() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0xad, &tick_fn) {
            Instruction::LDA(Operand::Absolute(_), _) => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }
}