// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::fmt;
use zinc64_core::{Cpu, TickFn};

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
    ANE(Operand),
    AXS(Operand),
    LAX(Operand),
}

impl Instruction {
    pub fn decode(cpu: &mut Cpu6510, opcode: u8, tick_fn: &TickFn) -> Instruction {
        match opcode {
            0x00 => Instruction::BRK,
            0x01 => Instruction::ORA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0x05 => Instruction::ORA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x06 => Instruction::ASL(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x08 => Instruction::PHP,
            0x09 => Instruction::ORA(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0x0a => Instruction::ASL(Operand::Accumulator),
            0x0d => Instruction::ORA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x0e => Instruction::ASL(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x10 => Instruction::BPL(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0x11 => Instruction::ORA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0x15 => Instruction::ORA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x16 => Instruction::ASL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x18 => Instruction::CLC,
            0x19 => Instruction::ORA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0x1d => Instruction::ORA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x1e => Instruction::ASL(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x20 => Instruction::JSR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x21 => Instruction::AND(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0x24 => Instruction::BIT(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x25 => Instruction::AND(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x26 => Instruction::ROL(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x28 => Instruction::PLP,
            0x29 => Instruction::AND(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0x2a => Instruction::ROL(Operand::Accumulator),
            0x2c => Instruction::BIT(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x2d => Instruction::AND(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x2e => Instruction::ROL(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x30 => Instruction::BMI(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0x31 => Instruction::AND(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0x35 => Instruction::AND(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x36 => Instruction::ROL(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x38 => Instruction::SEC,
            0x39 => Instruction::AND(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0x3a => Instruction::NOP,
            0x3d => Instruction::AND(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x3e => Instruction::ROL(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x40 => Instruction::RTI,
            0x41 => Instruction::EOR(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0x45 => Instruction::EOR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x46 => Instruction::LSR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x48 => Instruction::PHA,
            0x49 => Instruction::EOR(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0x4a => Instruction::LSR(Operand::Accumulator),
            0x4c => Instruction::JMP(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x4d => Instruction::EOR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x4e => Instruction::LSR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x50 => Instruction::BVC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0x51 => Instruction::EOR(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0x55 => Instruction::EOR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x56 => Instruction::LSR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x58 => Instruction::CLI,
            0x59 => Instruction::EOR(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0x5d => Instruction::EOR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x5e => Instruction::LSR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x60 => Instruction::RTS,
            0x61 => Instruction::ADC(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0x65 => Instruction::ADC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x66 => Instruction::ROR(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x68 => Instruction::PLA,
            0x69 => Instruction::ADC(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0x6a => Instruction::ROR(Operand::Accumulator),
            0x6c => Instruction::JMP(Operand::Indirect(cpu.fetch_word(tick_fn))),
            0x6d => Instruction::ADC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x6e => Instruction::ROR(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x70 => Instruction::BVS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0x71 => Instruction::ADC(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0x75 => Instruction::ADC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x76 => Instruction::ROR(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0x78 => Instruction::SEI,
            0x79 => Instruction::ADC(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0x7d => Instruction::ADC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x7e => Instruction::ROR(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0x81 => Instruction::STA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0x84 => Instruction::STY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x85 => Instruction::STA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x86 => Instruction::STX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0x88 => Instruction::DEY,
            0x8a => Instruction::TXA,
            0x8b => Instruction::ANE(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0x8c => Instruction::STY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x8d => Instruction::STA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x8e => Instruction::STX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0x90 => Instruction::BCC(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0x91 => Instruction::STA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0x94 => {
                tick_fn(); // HACK
                Instruction::STY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)))
            }
            0x95 => {
                tick_fn(); // HACK
                Instruction::STA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn)))
            }
            0x96 => Instruction::STX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn))),
            0x98 => Instruction::TYA,
            0x99 => Instruction::STA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0x9a => Instruction::TXS,
            0x9d => Instruction::STA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xa0 => Instruction::LDY(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xa1 => Instruction::LDA(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0xa2 => Instruction::LDX(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xa4 => Instruction::LDY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xa5 => Instruction::LDA(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xa6 => Instruction::LDX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xa7 => Instruction::LAX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xa8 => Instruction::TAY,
            0xa9 => Instruction::LDA(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xaa => Instruction::TAX,
            0xac => Instruction::LDY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xad => Instruction::LDA(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xae => Instruction::LDX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xb0 => Instruction::BCS(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0xb1 => Instruction::LDA(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0xb3 => Instruction::LAX(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0xb4 => Instruction::LDY(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xb5 => Instruction::LDA(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xb6 => Instruction::LDX(Operand::ZeroPageY(cpu.fetch_byte(tick_fn))),
            0xb8 => Instruction::CLV,
            0xb9 => Instruction::LDA(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0xba => Instruction::TSX,
            0xbc => Instruction::LDY(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xbd => Instruction::LDA(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xbe => Instruction::LDX(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0xc0 => Instruction::CPY(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xc1 => Instruction::CMP(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0xc4 => Instruction::CPY(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xc5 => Instruction::CMP(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xc6 => Instruction::DEC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xc8 => Instruction::INY,
            0xc9 => Instruction::CMP(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xca => Instruction::DEX,
            0xcb => Instruction::AXS(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xcc => Instruction::CPY(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xcd => Instruction::CMP(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xce => Instruction::DEC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xd0 => Instruction::BNE(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0xd1 => Instruction::CMP(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0xd5 => Instruction::CMP(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xd6 => Instruction::DEC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xd8 => Instruction::CLD,
            0xd9 => Instruction::CMP(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0xdd => Instruction::CMP(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xde => Instruction::DEC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xe0 => Instruction::CPX(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xe1 => Instruction::SBC(Operand::IndirectX(cpu.fetch_byte(tick_fn))),
            0xe4 => Instruction::CPX(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xe5 => Instruction::SBC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xe6 => Instruction::INC(Operand::ZeroPage(cpu.fetch_byte(tick_fn))),
            0xe8 => Instruction::INX,
            0xe9 => Instruction::SBC(Operand::Immediate(cpu.fetch_byte(tick_fn))),
            0xea => Instruction::NOP,
            0xec => Instruction::CPX(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xed => Instruction::SBC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xee => Instruction::INC(Operand::Absolute(cpu.fetch_word(tick_fn))),
            0xf0 => Instruction::BEQ(Operand::Relative(cpu.fetch_byte(tick_fn) as i8)),
            0xf1 => Instruction::SBC(Operand::IndirectY(cpu.fetch_byte(tick_fn))),
            0xf5 => Instruction::SBC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xf6 => Instruction::INC(Operand::ZeroPageX(cpu.fetch_byte(tick_fn))),
            0xf8 => Instruction::SED,
            0xf9 => Instruction::SBC(Operand::AbsoluteY(cpu.fetch_word(tick_fn))),
            0xfc => Instruction::NOP,
            0xfd => Instruction::SBC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            0xfe => Instruction::INC(Operand::AbsoluteX(cpu.fetch_word(tick_fn))),
            _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, cpu.get_pc()),
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
            Instruction::AXS(ref operand) => write!(f, "axs {}", operand),
            Instruction::LAX(ref operand) => write!(f, "lax {}", operand),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zinc64_core::{make_noop, new_shared, Addressable, IoPort, IrqLine, Mmu, Pin, Ram};

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
    fn decode_brk() {
        //let tick_fn: TickFn = Rc::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0x00, &make_noop()) {
            Instruction::BRK => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }

    #[test]
    fn decode_lda_absolute() {
        //let tick_fn: TickFn = Rc::new(move || {});
        let mut cpu = setup_cpu();
        let valid = match Instruction::decode(&mut cpu, 0xad, &make_noop()) {
            Instruction::LDA(Operand::Absolute(_)) => true,
            _ => false,
        };
        assert_eq!(true, valid);
    }
}
