// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(dead_code)]

use core::fmt;

pub enum Operand {
    Accumulator,
    Immediate(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    IndirectX(u8),
    IndirectY(u8),
    Indirect(u16),
    Relative(i8),
}

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
    ANE(Operand),
    ANX(Operand),
    ALR(Operand),
    AXS(Operand),
    LAX(Operand),
    LSE(Operand),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Operand::Accumulator => write!(f, "acc"),
            Operand::Immediate(value) => write!(f, "#{:02x}", value),
            Operand::ZeroPage(address) => write!(f, "${:02x}", address),
            Operand::ZeroPageX(address) => write!(f, "${:02x},x", address),
            Operand::ZeroPageY(address) => write!(f, "${:02x},y", address),
            Operand::Absolute(address) => write!(f, "${:04x}", address),
            Operand::AbsoluteX(address) => write!(f, "${:04x},x", address),
            Operand::AbsoluteY(address) => write!(f, "${:04x},y", address),
            Operand::IndirectX(address) => write!(f, "$({:02x},x)", address),
            Operand::IndirectY(address) => write!(f, "$({:02x},y)", address),
            Operand::Indirect(address) => write!(f, "$({:04x})", address),
            Operand::Relative(offset) => write!(f, "${:02x}", offset),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            // Data Movement
            Instruction::LDA(ref operand) => write!(f, "lda {}", operand),
            Instruction::LDX(ref operand) => write!(f, "ldx {}", operand),
            Instruction::LDY(ref operand) => write!(f, "ldy {}", operand),
            Instruction::PHA => write!(f, "pha"),
            Instruction::PHP => write!(f, "php"),
            Instruction::PLA => write!(f, "pla"),
            Instruction::PLP => write!(f, "plp"),
            Instruction::STA(ref operand) => write!(f, "sta {}", operand),
            Instruction::STX(ref operand) => write!(f, "stx {}", operand),
            Instruction::STY(ref operand) => write!(f, "sty {}", operand),
            Instruction::TAX => write!(f, "tax"),
            Instruction::TAY => write!(f, "tay"),
            Instruction::TSX => write!(f, "tsx"),
            Instruction::TXA => write!(f, "txa"),
            Instruction::TXS => write!(f, "txs"),
            Instruction::TYA => write!(f, "tya"),
            // Arithmetic
            Instruction::ADC(ref operand) => write!(f, "adc {}", operand),
            Instruction::SBC(ref operand) => write!(f, "sbc {}", operand),
            Instruction::CMP(ref operand) => write!(f, "cmp {}", operand),
            Instruction::CPX(ref operand) => write!(f, "cpx {}", operand),
            Instruction::CPY(ref operand) => write!(f, "cpy {}", operand),
            Instruction::DEC(ref operand) => write!(f, "dec {}", operand),
            Instruction::DEX => write!(f, "dex"),
            Instruction::DEY => write!(f, "dey"),
            Instruction::INC(ref operand) => write!(f, "inc {}", operand),
            Instruction::INX => write!(f, "inx"),
            Instruction::INY => write!(f, "iny"),
            // Logical
            Instruction::AND(ref operand) => write!(f, "and {}", operand),
            Instruction::EOR(ref operand) => write!(f, "eor {}", operand),
            Instruction::ORA(ref operand) => write!(f, "ora {}", operand),
            // Shift and Rotate
            Instruction::ASL(ref operand) => write!(f, "asl {}", operand),
            Instruction::LSR(ref operand) => write!(f, "lsr {}", operand),
            Instruction::ROL(ref operand) => write!(f, "rol {}", operand),
            Instruction::ROR(ref operand) => write!(f, "ror {}", operand),
            // Control Flow
            Instruction::BCC(ref operand) => write!(f, "bcc {}", operand),
            Instruction::BCS(ref operand) => write!(f, "bcs {}", operand),
            Instruction::BEQ(ref operand) => write!(f, "beq {}", operand),
            Instruction::BMI(ref operand) => write!(f, "bmi {}", operand),
            Instruction::BNE(ref operand) => write!(f, "bne {}", operand),
            Instruction::BPL(ref operand) => write!(f, "bpl {}", operand),
            Instruction::BVC(ref operand) => write!(f, "bvc {}", operand),
            Instruction::BVS(ref operand) => write!(f, "bvs {}", operand),
            Instruction::JMP(ref operand) => write!(f, "jmp {}", operand),
            Instruction::JSR(ref operand) => write!(f, "jsr {}", operand),
            Instruction::RTS => write!(f, "rts"),
            // Misc
            Instruction::BIT(ref operand) => write!(f, "bit {}", operand),
            Instruction::BRK => write!(f, "brk"),
            Instruction::CLC => write!(f, "clc"),
            Instruction::CLD => write!(f, "cld"),
            Instruction::CLI => write!(f, "cli"),
            Instruction::CLV => write!(f, "clv"),
            Instruction::NOP => write!(f, "nop"),
            Instruction::SEC => write!(f, "sec"),
            Instruction::SED => write!(f, "sed"),
            Instruction::SEI => write!(f, "sei"),
            Instruction::RTI => write!(f, "rti"),
            // Undocumented
            Instruction::ANE(ref operand) => write!(f, "ane {}", operand),
            Instruction::ANX(ref operand) => write!(f, "anx {}", operand),
            Instruction::ALR(ref operand) => write!(f, "alr {}", operand),
            Instruction::AXS(ref operand) => write!(f, "axs {}", operand),
            Instruction::LAX(ref operand) => write!(f, "lax {}", operand),
            Instruction::LSE(ref operand) => write!(f, "lse {}", operand),
        }
    }
}
