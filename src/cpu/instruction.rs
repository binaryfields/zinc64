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

use std::fmt;

use core::{Cpu, TickFn};

use super::operand::Operand;
use super::Cpu6510;

// Spec: MCS6510 MICROPROCESSOR INSTRUCTION SET p.232
// Design:
//   UAE uses I_dec_tab_entry with instr_func and instr_params(src,dst). We want something
//   similar to I_dec_tab_entry.
//   C64 instructions have zero or one operand so we encode them using Instruction(Operand)
//   variants.

// Categories: Asm One Manual Sec 10.1
pub enum Instruction {
    // Data Movement (16)
    LDA(Operand),
    LDX(Operand),
    LDY(Operand),
    PHA,
    PHP,
    PLA,
    PLP,
    STA(Operand),
    STX(Operand),
    STY(Operand),
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    // Arithmetic (11)
    ADC(Operand),
    SBC(Operand),
    CMP(Operand),
    CPX(Operand),
    CPY(Operand),
    DEC(Operand),
    DEX,
    DEY,
    INC(Operand),
    INX,
    INY,
    // Logical (3)
    AND(Operand),
    EOR(Operand),
    ORA(Operand),
    // Shift and Rotate (4)
    ASL(Operand),
    LSR(Operand),
    ROL(Operand),
    ROR(Operand),
    // Control Flow (11)
    BCC(Operand),
    BCS(Operand),
    BEQ(Operand),
    BMI(Operand),
    BNE(Operand),
    BPL(Operand),
    BVC(Operand),
    BVS(Operand),
    JMP(Operand),
    JSR(Operand),
    RTS,
    // Misc (11)
    BIT(Operand),
    BRK,
    CLC,
    CLD,
    CLI,
    CLV,
    NOP,
    SEC,
    SED,
    SEI,
    RTI,
    // Undocumented
    AXS(Operand),
    LAX(Operand),
}

impl Instruction {
    pub fn decode(cpu: &mut Cpu6510, opcode: u8, tick_fn: &TickFn) -> Instruction {
        match opcode {
            // BRK
            0x00 => Instruction::BRK,
            // ORA (Oper,X)
            0x01 => Instruction::ORA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // ORA Oper
            0x05 => Instruction::ORA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // ASL Oper
            0x06 => Instruction::ASL(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // PHP
            0x08 => Instruction::PHP,
            // ORA #Oper
            0x09 => Instruction::ORA(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // ASL A
            0x0a => Instruction::ASL(Operand::Accumulator),
            // ORA Oper
            0x0d => Instruction::ORA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // ASL Oper
            0x0e => Instruction::ASL(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BPL
            0x10 => Instruction::BPL(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // ORA (Oper),Y
            0x11 => Instruction::ORA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // ORA Oper,X
            0x15 => Instruction::ORA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // ASL Oper,X
            0x16 => Instruction::ASL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // CLC
            0x18 => Instruction::CLC,
            // ORA Oper,Y
            0x19 => Instruction::ORA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // ORA Oper,X (NOTE doc lists as 0x10)
            0x1d => Instruction::ORA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // ASL Oper,X
            0x1e => Instruction::ASL(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // JSR Oper
            0x20 => Instruction::JSR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // AND (Oper,X)
            0x21 => Instruction::AND(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // BIT Oper
            0x24 => Instruction::BIT(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // AND Oper
            0x25 => Instruction::AND(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // ROL Oper
            0x26 => Instruction::ROL(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // PLP
            0x28 => Instruction::PLP,
            // AND #Oper
            0x29 => Instruction::AND(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // ROL A
            0x2a => Instruction::ROL(Operand::Accumulator),
            // BIT Oper
            0x2c => Instruction::BIT(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // AND Oper
            0x2d => Instruction::AND(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // ROL Oper
            0x2e => Instruction::ROL(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BMI
            0x30 => Instruction::BMI(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // AND (Oper),Y
            0x31 => Instruction::AND(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // AND Oper,X
            0x35 => Instruction::AND(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // ROL Oper,X
            0x36 => Instruction::ROL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // SEC
            0x38 => Instruction::SEC,
            // AND Oper,Y
            0x39 => Instruction::AND(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // AND Oper,X
            0x3d => Instruction::AND(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // ROL Oper,X
            0x3e => Instruction::ROL(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // RTI (doc lists as 0x4d)
            0x40 => Instruction::RTI,
            // EOR (Oper,X)
            0x41 => Instruction::EOR(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // EOR Oper
            0x45 => Instruction::EOR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // LSR Oper
            0x46 => Instruction::LSR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // PHA
            0x48 => Instruction::PHA,
            // EOR #Oper
            0x49 => Instruction::EOR(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // LSR A
            0x4a => Instruction::LSR(Operand::Accumulator),
            // JMP Oper
            0x4c => Instruction::JMP(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // EOR Oper (doc lists as 0x40)
            0x4d => Instruction::EOR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // LSR Oper
            0x4e => Instruction::LSR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BVC
            0x50 => Instruction::BVC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // EOR (Oper),Y
            0x51 => Instruction::EOR(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // EOR Oper,X
            0x55 => Instruction::EOR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // LSR Oper,X
            0x56 => Instruction::LSR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // CLI
            0x58 => Instruction::CLI,
            // EOR Oper,Y
            0x59 => Instruction::EOR(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // EOR Oper,X (doc lists as 0x50)
            0x5d => Instruction::EOR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // LSR Oper,X
            0x5e => Instruction::LSR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // RTS
            0x60 => Instruction::RTS,
            // ADC (Oper,X)
            0x61 => Instruction::ADC(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // ADC Oper
            0x65 => Instruction::ADC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // ROR Oper
            0x66 => Instruction::ROR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // PLA
            0x68 => Instruction::PLA,
            // ADC #Oper
            0x69 => Instruction::ADC(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // ROR A
            0x6a => Instruction::ROR(Operand::Accumulator),
            // JMP (Oper)
            0x6c => Instruction::JMP(Operand::Indirect(cpu.fetch_word(tick_fn))),
            // ADC Oper (doc lists as 0x60)
            0x6d => Instruction::ADC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // ROR Oper
            0x6e => Instruction::ROR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BVS
            0x70 => Instruction::BVS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // ADC (Oper),Y
            0x71 => Instruction::ADC(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // ADC Oper,X
            0x75 => Instruction::ADC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // ROR Oper,X
            0x76 => Instruction::ROR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // SEI
            0x78 => Instruction::SEI,
            // ADC Oper,Y
            0x79 => Instruction::ADC(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // ADC Oper,X (doc lists as 0x70)
            0x7d => Instruction::ADC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // ROR Oper,X
            0x7e => Instruction::ROR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // STA (Oper,X)
            0x81 => Instruction::STA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // STY Oper
            0x84 => Instruction::STY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // STA Oper
            0x85 => Instruction::STA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // STX Oper
            0x86 => Instruction::STX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // DEY
            0x88 => Instruction::DEY,
            // TXA
            0x8a => Instruction::TXA,
            // STY Oper
            0x8c => Instruction::STY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // STA Oper (doc lists as 0x80)
            0x8d => Instruction::STA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // STX Oper
            0x8e => Instruction::STX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BCC
            0x90 => Instruction::BCC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // STA (Oper),Y
            0x91 => Instruction::STA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // STY Oper,X
            0x94 => {
                tick_fn(); // HACK
                Instruction::STY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)))
            }
            // STA Oper,X
            0x95 => {
                tick_fn(); // HACK
                Instruction::STA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)))
            }
            // STX Oper,Y
            0x96 => Instruction::STX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn))),
            // TYA
            0x98 => Instruction::TYA,
            // STA Oper,Y
            0x99 => Instruction::STA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // TXS
            0x9a => Instruction::TXS,
            // STA Oper,X (doc lists as 0x90)
            0x9d => Instruction::STA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // LDY #Oper
            0xa0 => Instruction::LDY(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // LDA (Oper,X)
            0xa1 => Instruction::LDA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // LDX #Oper
            0xa2 => Instruction::LDX(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // LDY Oper
            0xa4 => Instruction::LDY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // LDA Oper
            0xa5 => Instruction::LDA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // LDX Oper
            0xa6 => Instruction::LDX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // LAX Oper
            0xa7 => Instruction::LAX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // TAY
            0xa8 => Instruction::TAY,
            // LDA #Oper
            0xa9 => Instruction::LDA(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // TAX
            0xaa => Instruction::TAX,
            // LDY Oper
            0xac => Instruction::LDY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // LDA Oper
            0xad => Instruction::LDA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // LDX Oper
            0xae => Instruction::LDX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BCS
            0xb0 => Instruction::BCS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // LDA (Oper),Y
            0xb1 => Instruction::LDA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // LAX (Oper),Y
            0xb3 => Instruction::LAX(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // LDY Oper,X
            0xb4 => Instruction::LDY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // LDA Oper,X
            0xb5 => Instruction::LDA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // LDX Oper,Y
            0xb6 => Instruction::LDX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn))),
            // CLV
            0xb8 => Instruction::CLV,
            // LDA Oper,Y
            0xb9 => Instruction::LDA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // TSX
            0xba => Instruction::TSX,
            // LDY Oper,X
            0xbc => Instruction::LDY(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // LDA Oper,X
            0xbd => Instruction::LDA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // LDX Oper,Y
            0xbe => Instruction::LDX(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // CPY *Oper
            0xc0 => Instruction::CPY(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // CMP (Oper,X)
            0xc1 => Instruction::CMP(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // CPY Oper
            0xc4 => Instruction::CPY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // CMP Oper
            0xc5 => Instruction::CMP(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // DEC Oper
            0xc6 => Instruction::DEC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // INY
            0xc8 => Instruction::INY,
            // CMP #Oper
            0xc9 => Instruction::CMP(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // DEX
            0xca => Instruction::DEX,
            // AXS
            0xcb => Instruction::AXS(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // CPY Oper
            0xcc => Instruction::CPY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // CMP Oper
            0xcd => Instruction::CMP(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // DEC Oper
            0xce => Instruction::DEC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BNE
            0xd0 => Instruction::BNE(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // CMP (Oper),Y
            0xd1 => Instruction::CMP(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // CMP Oper,X
            0xd5 => Instruction::CMP(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // DEC Oper,X
            0xd6 => Instruction::DEC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // CLD
            0xd8 => Instruction::CLD,
            // CMP Oper,Y
            0xd9 => Instruction::CMP(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // CMP Oper,X
            0xdd => Instruction::CMP(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // DEC Oper,X
            0xde => Instruction::DEC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // CPX *Oper
            0xe0 => Instruction::CPX(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // SBC (Oper,X)
            0xe1 => Instruction::SBC(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            // CPX Oper
            0xe4 => Instruction::CPX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // SBC Oper
            0xe5 => Instruction::SBC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // INC Oper
            0xe6 => Instruction::INC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            // INX
            0xe8 => Instruction::INX,
            // SBC #Oper
            0xe9 => Instruction::SBC(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            // NOP
            0xea => Instruction::NOP,
            // CPX Oper
            0xec => Instruction::CPX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // SBC Oper
            0xed => Instruction::SBC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // INC Oper
            0xee => Instruction::INC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            // BEQ
            0xf0 => Instruction::BEQ(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            // SBC (Oper),Y
            0xf1 => Instruction::SBC(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            // SBC Oper,X
            0xf5 => Instruction::SBC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // INC Oper,X
            0xf6 => Instruction::INC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            // SED
            0xf8 => Instruction::SED,
            // SBC Oper,Y
            0xf9 => Instruction::SBC(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            // SBC Oper,X
            0xfd => Instruction::SBC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // INC Oper,X
            0xfe => Instruction::INC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            // catch all
            _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, cpu.get_pc()),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Data Movement
            &Instruction::LDA(ref operand) => write!(f, "lda {}", operand),
            &Instruction::LDX(ref operand) => write!(f, "ldx {}", operand),
            &Instruction::LDY(ref operand) => write!(f, "ldy {}", operand),
            &Instruction::PHA => write!(f, "pha"),
            &Instruction::PHP => write!(f, "php"),
            &Instruction::PLA => write!(f, "pla"),
            &Instruction::PLP => write!(f, "plp"),
            &Instruction::STA(ref operand) => write!(f, "sta {}", operand),
            &Instruction::STX(ref operand) => write!(f, "stx {}", operand),
            &Instruction::STY(ref operand) => write!(f, "sty {}", operand),
            &Instruction::TAX => write!(f, "tax"),
            &Instruction::TAY => write!(f, "tay"),
            &Instruction::TSX => write!(f, "tsx"),
            &Instruction::TXA => write!(f, "txa"),
            &Instruction::TXS => write!(f, "txs"),
            &Instruction::TYA => write!(f, "tya"),
            // Arithmetic
            &Instruction::ADC(ref operand) => write!(f, "adc {}", operand),
            &Instruction::SBC(ref operand) => write!(f, "sbc {}", operand),
            &Instruction::CMP(ref operand) => write!(f, "cmp {}", operand),
            &Instruction::CPX(ref operand) => write!(f, "cpx {}", operand),
            &Instruction::CPY(ref operand) => write!(f, "cpy {}", operand),
            &Instruction::DEC(ref operand) => write!(f, "dec {}", operand),
            &Instruction::DEX => write!(f, "dex"),
            &Instruction::DEY => write!(f, "dey"),
            &Instruction::INC(ref operand) => write!(f, "inc {}", operand),
            &Instruction::INX => write!(f, "inx"),
            &Instruction::INY => write!(f, "iny"),
            // Logical
            &Instruction::AND(ref operand) => write!(f, "and {}", operand),
            &Instruction::EOR(ref operand) => write!(f, "eor {}", operand),
            &Instruction::ORA(ref operand) => write!(f, "ora {}", operand),
            // Shift and Rotate
            &Instruction::ASL(ref operand) => write!(f, "asl {}", operand),
            &Instruction::LSR(ref operand) => write!(f, "lsr {}", operand),
            &Instruction::ROL(ref operand) => write!(f, "rol {}", operand),
            &Instruction::ROR(ref operand) => write!(f, "ror {}", operand),
            // Control Flow
            &Instruction::BCC(ref operand) => write!(f, "bcc {}", operand),
            &Instruction::BCS(ref operand) => write!(f, "bcs {}", operand),
            &Instruction::BEQ(ref operand) => write!(f, "beq {}", operand),
            &Instruction::BMI(ref operand) => write!(f, "bmi {}", operand),
            &Instruction::BNE(ref operand) => write!(f, "bne {}", operand),
            &Instruction::BPL(ref operand) => write!(f, "bpl {}", operand),
            &Instruction::BVC(ref operand) => write!(f, "bvc {}", operand),
            &Instruction::BVS(ref operand) => write!(f, "bvs {}", operand),
            &Instruction::JMP(ref operand) => write!(f, "jmp {}", operand),
            &Instruction::JSR(ref operand) => write!(f, "jsr {}", operand),
            &Instruction::RTS => write!(f, "rts"),
            // Misc
            &Instruction::BIT(ref operand) => write!(f, "bit {}", operand),
            &Instruction::BRK => write!(f, "brk"),
            &Instruction::CLC => write!(f, "clc"),
            &Instruction::CLD => write!(f, "cld"),
            &Instruction::CLI => write!(f, "cli"),
            &Instruction::CLV => write!(f, "clv"),
            &Instruction::NOP => write!(f, "nop"),
            &Instruction::SEC => write!(f, "sec"),
            &Instruction::SED => write!(f, "sed"),
            &Instruction::SEI => write!(f, "sei"),
            &Instruction::RTI => write!(f, "rti"),
            // Undocumented
            &Instruction::AXS(ref operand) => write!(f, "axs {}", operand),
            &Instruction::LAX(ref operand) => write!(f, "lax {}", operand),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{IoPort, IrqLine, Mmu, Pin, Ram};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct MockMemory {
        ram: Ram,
    }

    impl MockMemory {
        pub fn new(ram: Ram) -> Self {
            Self { ram }
        }
    }

    impl Mmu for MockMemory {
        fn switch_banks(&mut self, _mode: u8) {}

        fn read(&self, address: u16) -> u8 {
            self.ram.read(address)
        }

        fn write(&mut self, address: u16, value: u8) {
            self.ram.write(address, value);
        }
    }

    fn setup_cpu() -> Cpu6510 {
        let ba_line = Rc::new(RefCell::new(Pin::new_high()));
        let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
        let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
        let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
        let mem = Rc::new(RefCell::new(MockMemory::new(Ram::new(0x10000))));
        Cpu6510::new(ba_line, cpu_io_port, cpu_irq, cpu_nmi, mem)
    }

    #[test]
    fn decode_brk() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0x00, &tick_fn) {
            Instruction::BRK => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }

    #[test]
    fn decode_lda_absolute() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0xad, &tick_fn) {
            Instruction::LDA(Operand::Absolute(_)) => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }
}
