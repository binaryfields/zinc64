// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::fmt;
use log::LogLevel;

use crate::factory::{Addressable, Cpu, Register, TickFn};
use crate::util::{IoPort, IrqLine, Pin, Shared};

use super::uops::{decode_opcode, load_program, MicroOp, MicroOpPair, ProgramId};

pub enum Flag {
    Carry = 1,
    Zero = 1 << 1,
    IntDisable = 1 << 2,
    Decimal = 1 << 3,
    Break = 1 << 4,
    Reserved = 1 << 5,
    Overflow = 1 << 6,
    Negative = 1 << 7,
}

pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub p: u8,
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
    opcode: u8,
    uops: &'static [MicroOpPair],
    cycle: u8,
    address_lo: u8,
    address_hi: u8,
    data: u8,
    page_cross: bool,
    last_nmi: bool,
    last_pc: u16,
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
            opcode: 0,
            uops: load_program(ProgramId::Start),
            cycle: 0,
            address_lo: 0,
            address_hi: 0,
            data: 0,
            page_cross: false,
            last_nmi: false,
            last_pc: 0,
            ba_line,
            io_port,
            irq_line,
            nmi_line,
        }
    }

    pub fn clock(&mut self) {
        if self.ba_line.borrow().is_low() {
            return;
        } 
        let pair = self.uops[self.cycle as usize];
        self.execute(pair.0);
        if let Some(op1) = pair.1 {
            self.execute(op1);
        }
        self.cycle += 1;
    }

    fn execute(&mut self, op: MicroOp) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::uop", "0x{:04x}: {:02x} #{} {:<18}; {}", self.regs.pc, self.opcode, self.cycle, format!("{:?}", op), &self);
        }
        match op {
            MicroOp::FetchOpcode => self.load_next_program(),
            MicroOp::FetchOpcodeDiscard => self.fetch_opcode_discard(),
            MicroOp::FetchOperand => self.fetch_operand(),
            MicroOp::FetchAdl => self.fetch_adl(),
            MicroOp::FetchAdh => self.fetch_adh(),
            MicroOp::IncrementAdlX => self.increment_adl_x(),
            MicroOp::IncrementAdlY => self.increment_adl_y(),
            MicroOp::IndirectFetchAdl => self.indirect_fetch_adl(),
            MicroOp::IndirectFetchAdh => self.indirect_fetch_adh(),
            MicroOp::ReadData => self.read_data(),
            MicroOp::ReadDataOrFixAdh => self.read_data_or_fix_adh(),
            MicroOp::WriteData => self.write_data(),
            MicroOp::OpLDA => self.lda(),
            MicroOp::OpLDX => self.ldx(),
            MicroOp::OpLDY => self.ldy(),
            MicroOp::OpSTA => self.sda(),
            MicroOp::OpSTX => self.sdx(),
            MicroOp::OpSTY => self.sdy(),
            MicroOp::OpTAX => self.tax(),
            MicroOp::OpTXA => self.txa(),
            MicroOp::OpTAY => self.tay(),
            MicroOp::OpTYA => self.tya(),
            MicroOp::OpTSX => self.tsx(),
            MicroOp::OpTXS => self.txs(),
            MicroOp::OpPLA => self.pla(),
            MicroOp::OpPLP => self.plp(),
            MicroOp::OpPHA => self.pha(),
            MicroOp::OpPHP => self.php(),
            MicroOp::OpAND => self.and(),
            MicroOp::OpEOR => self.eor(),
            MicroOp::OpORA => self.ora(),
            MicroOp::OpADC => self.adc(),
            MicroOp::OpSBC => self.sbc(),
            MicroOp::OpBIT => self.bit(),
            MicroOp::OpCMP => self.cmp(),
            MicroOp::OpCPX => self.cpx(),
            MicroOp::OpCPY => self.cpy(),
            MicroOp::OpDEC => self.dec(),
            MicroOp::OpDEX => self.dex(),
            MicroOp::OpDEY => self.dey(),
            MicroOp::OpINC => self.inc(),
            MicroOp::OpINX => self.inx(),
            MicroOp::OpINY => self.iny(),
            MicroOp::OpASL => self.asl(),
            MicroOp::OpASLImplied => self.asl_implied(),
            MicroOp::OpLSR => self.lsr(),
            MicroOp::OpLSRImplied => self.lsr_implied(),
            MicroOp::OpROL => self.rol(),
            MicroOp::OpROLImplied => self.rol_implied(),
            MicroOp::OpROR => self.ror(),
            MicroOp::OpRORImplied => self.ror_implied(),
            MicroOp::OpJMP => self.jmp(),
            MicroOp::OpJSR => self.jsr(),
            MicroOp::OpRTS => self.rts(),
            MicroOp::OpBRK => self.brk(),
            MicroOp::OpRTI => self.rti(),
            MicroOp::OpBCC => self.branch(Flag::Carry, false),
            MicroOp::OpBCS => self.branch(Flag::Carry, true),
            MicroOp::OpBEQ => self.branch(Flag::Zero, true),
            MicroOp::OpBNE => self.branch(Flag::Zero, false),
            MicroOp::OpBMI => self.branch(Flag::Negative, true),
            MicroOp::OpBPL => self.branch(Flag::Negative, false),
            MicroOp::OpBVC => self.branch(Flag::Overflow, false),
            MicroOp::OpBVS => self.branch(Flag::Overflow, true),
            MicroOp::OpCLC => self.clear_flag(Flag::Carry),
            MicroOp::OpCLD => self.clear_flag(Flag::Decimal),
            MicroOp::OpCLI => self.clear_flag(Flag::IntDisable),
            MicroOp::OpCLV => self.clear_flag(Flag::Overflow),
            MicroOp::OpSEC => self.set_flag(Flag::Carry),
            MicroOp::OpSED => self.set_flag(Flag::Decimal),
            MicroOp::OpSEI => self.set_flag(Flag::IntDisable),
            MicroOp::OpNOP => self.nop(),
            MicroOp::OpANE => self.ane(),
            MicroOp::OpANX => self.anx(),
            MicroOp::OpALR => self.alr(),
            MicroOp::OpAXS => self.axs(),
            MicroOp::OpLAX => self.lax(),
            MicroOp::OpLSE => self.lse(),
            MicroOp::OpIRQ => self.irq(),
            MicroOp::OpNMI => self.nmi(),
            MicroOp::OpRST => self.rst(),
        }
    }

    fn load_next_program(&mut self) {
        if self.nmi_line.borrow().is_low() {
            if log_enabled!(LogLevel::Trace) {
                trace!(target: "cpu::int", "IRQ");
            }
            self.uops = load_program(ProgramId::Nmi);
        } else if self.irq_line.borrow().is_low() && !self.test_flag(Flag::IntDisable) {
            if log_enabled!(LogLevel::Trace) {
                trace!(target: "cpu::int", "NMI");
            }
            self.uops = load_program(ProgramId::Irq);
        } else {
            self.fetch_opcode();
        }
        self.cycle = 0;
    }

    fn fetch_opcode(&mut self) {
        self.opcode = self.read_mem(self.regs.pc);
        self.uops = decode_opcode(self.opcode);
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "cpu::ins", "0x{:04x}: {:02x}; {}", self.regs.pc, self.opcode, &self);
        }
        self.regs.pc = self.regs.pc.wrapping_add(1);
    }

    fn fetch_opcode_discard(&mut self) {
        let _ = self.read_mem(self.regs.pc);
    }

    fn fetch_operand(&mut self) {
        self.data = self.read_mem(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
    }

    fn fetch_adl(&mut self) {
        self.address_hi = 0;
        self.address_lo = self.read_mem(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
    }

    fn fetch_adh(&mut self) {
        self.address_hi = self.read_mem(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
    }

    fn increment_adl_x(&mut self) {
        self.page_cross = self.address_lo.checked_add(self.regs.x).is_none();
        self.address_lo = self.address_lo.wrapping_add(self.regs.x);
    }

    fn increment_adl_y(&mut self) {
        self.page_cross = self.address_lo.checked_add(self.regs.y).is_none();
        self.address_lo = self.address_lo.wrapping_add(self.regs.y);
    }

    fn indirect_fetch_adl(&mut self) {
        let address = make_address(self.address_hi, self.address_lo);
        let adl = self.read_mem(address);
        self.data = adl;
        self.address_lo = self.address_lo.wrapping_add(1);
    }

    fn indirect_fetch_adh(&mut self) {
        let address = make_address(self.address_hi, self.address_lo);
        self.address_lo = self.data;
        self.address_hi = self.read_mem(address);
    }

    fn read_data_or_fix_adh(&mut self) {
        if self.page_cross == true {
            self.address_hi = self.address_hi.wrapping_add(1);
        } else {
            let address = make_address(self.address_hi, self.address_lo);
            self.data = self.read_mem(address);
            self.cycle += 1;
        }
    }

    fn read_data(&mut self) {
        let address = make_address(self.address_hi, self.address_lo);
        self.data = self.read_mem(address);
    }

    fn write_data(&mut self) {
        let address = make_address(self.address_hi, self.address_lo);
        self.write_mem(address, self.data);
    }

    fn lda(&mut self) {
        let data = self.data;
        self.regs.a = data;
        self.set_nz(data);
    }

    fn ldx(&mut self) {
        let data = self.data;
        self.regs.x = data;
        self.set_nz(data);
    }

    fn ldy(&mut self) {
        let data = self.data;
        self.regs.y = data;
        self.set_nz(data);
    }

    fn sda(&mut self) {
        self.data = self.regs.a;
    }

    fn sdx(&mut self) {
        self.data = self.regs.x;
    }

    fn sdy(&mut self) {
        self.data = self.regs.y;
    }

    fn tax(&mut self) {
        let data = self.regs.a;
        self.regs.x = data;
        self.set_nz(data);
    }

    fn txa(&mut self) {
        let data = self.regs.x;
        self.regs.a = data;
        self.set_nz(data);
    }

    fn tay(&mut self) {
        let data = self.regs.a;
        self.regs.y = data;
        self.set_nz(data);
    }

    fn tya(&mut self) {
        let data = self.regs.y;
        self.regs.a = data;
        self.set_nz(data);
    }

    fn tsx(&mut self) {
        let data = self.regs.sp;
        self.regs.x = data;
        self.set_nz(data);
    }

    fn txs(&mut self) {
        let data = self.regs.x;
        self.regs.sp = data;
        // NOTE do not set nz
    }

    fn pla(&mut self) {
        match self.cycle {
            2 => {
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => {
                let address = make_address(0x01, self.regs.sp);
                let data = self.read_mem(address);
                self.regs.a = data;
                self.set_nz(data);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn plp(&mut self) {
        match self.cycle {
            2 => {
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => {
                let address = make_address(0x01, self.regs.sp);
                let data = self.read_mem(address);
                self.regs.p = data;
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn pha(&mut self) {
        match self.cycle {
            2 => {
                self.write_stack(self.regs.a);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn php(&mut self) {
        match self.cycle {
            2 => {
                self.write_stack(self.regs.p | (Flag::Break as u8) | (Flag::Reserved as u8));
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn and(&mut self) {
        let result = self.regs.a & self.data;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn eor(&mut self) {
        let result = self.regs.a ^ self.data;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn ora(&mut self) {
        let result = self.regs.a | self.data;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn adc(&mut self) {
        let ac = self.regs.a as u16;
        let value = self.data as u16;
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
        self.update_flag(
            Flag::Overflow,
            (ac ^ value) & 0x80 == 0 && (ac ^ temp) & 0x80 == 0x80,
        );
        self.update_flag(Flag::Carry, temp > 0xff);
        let result = (temp & 0xff) as u8;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn sbc(&mut self) {
        let ac = self.regs.a as u16;
        let value = self.data as u16;
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
        self.update_flag(
            Flag::Overflow,
            (ac ^ temp) & 0x80 != 0 && (ac ^ value) & 0x80 == 0x80,
        );
        self.update_flag(Flag::Carry, temp < 0x100);
        let result = (temp & 0xff) as u8;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn bit(&mut self) {
        let data = self.data;
        let a = self.regs.a;
        self.update_flag(Flag::Negative, data & 0x80 != 0);
        self.update_flag(Flag::Overflow, 0x40 & data != 0);
        self.update_flag(Flag::Zero, data & a == 0);
    }

    fn cmp(&mut self) {
        let result = (u16::from(self.regs.a)).wrapping_sub(u16::from(self.data));
        self.update_flag(Flag::Carry, result < 0x100);
        self.set_nz((result & 0xff) as u8);
    }

    fn cpx(&mut self) {
        let result = (u16::from(self.regs.x)).wrapping_sub(u16::from(self.data));
        self.update_flag(Flag::Carry, result < 0x100);
        self.set_nz((result & 0xff) as u8);
    }

    fn cpy(&mut self) {
        let result = (u16::from(self.regs.y)).wrapping_sub(u16::from(self.data));
        self.update_flag(Flag::Carry, result < 0x100);
        self.set_nz((result & 0xff) as u8);
    }

    fn dec(&mut self) {
        let result = self.data.wrapping_sub(1);
        self.data = result;
        self.set_nz(result);
    }

    fn dex(&mut self) {
        let result = self.regs.x.wrapping_sub(1);
        self.regs.x = result;
        self.set_nz(result);
    }

    fn dey(&mut self) {
        let result = self.regs.y.wrapping_sub(1);
        self.regs.y = result;
        self.set_nz(result);
    }

    fn inc(&mut self) {
        let result = self.data.wrapping_add(1);
        self.data = result;
        self.set_nz(result);
    }

    fn inx(&mut self) {
        let result = self.regs.x.wrapping_add(1);
        self.regs.x = result;
        self.set_nz(result);
    }

    fn iny(&mut self) {
        let result = self.regs.y.wrapping_add(1);
        self.regs.y = result;
        self.set_nz(result);
    }

    fn asl(&mut self) {
        let data = self.data;
        self.update_flag(Flag::Carry, (data & 0x80) != 0);
        let result = data << 1;
        self.data = result;
        self.set_nz(result);
    }

    fn asl_implied(&mut self) {
        let data = self.regs.a;
        self.update_flag(Flag::Carry, (data & 0x80) != 0);
        let result = data << 1;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn lsr(&mut self) {
        let data = self.data;
        self.update_flag(Flag::Carry, (data & 0x01) != 0);
        let result = data >> 1;
        self.data = result;
        self.set_nz(result);
    }

    fn lsr_implied(&mut self) {
        let data = self.regs.a;
        self.update_flag(Flag::Carry, (data & 0x01) != 0);
        let result = data >> 1;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn rol(&mut self) {
        let data = self.data;
        let mut temp = (data as u16) << 1;
        if self.test_flag(Flag::Carry) {
            temp |= 0x01
        };
        self.update_flag(Flag::Carry, temp > 0xff);
        let result = (temp & 0xff) as u8;
        self.data = result;
        self.set_nz(result);
    }

    fn rol_implied(&mut self) {
        let data = self.regs.a;
        let mut temp = (data as u16) << 1;
        if self.test_flag(Flag::Carry) {
            temp |= 0x01
        };
        self.update_flag(Flag::Carry, temp > 0xff);
        let result = (temp & 0xff) as u8;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn ror(&mut self) {
        let data = self.data as u16;
        let mut temp = if self.test_flag(Flag::Carry) {
            data | 0x100
        } else {
            data
        };
        self.update_flag(Flag::Carry, temp & 0x01 != 0);
        temp >>= 1;
        let result = (temp & 0xff) as u8;
        self.data = result;
        self.set_nz(result);
    }

    fn ror_implied(&mut self) {
        let data = self.regs.a as u16;
        let mut temp = if self.test_flag(Flag::Carry) {
            data | 0x100
        } else {
            data
        };
        self.update_flag(Flag::Carry, temp & 0x01 != 0);
        temp >>= 1;
        let result = (temp & 0xff) as u8;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn jmp(&mut self) {
        self.regs.pc = make_address(self.address_hi, self.address_lo);
    }

    fn jsr(&mut self) {
        match self.cycle {
            2 => {
                // SP -> Address Bus
            }
            3 => {
                self.write_stack(hi_byte(self.regs.pc));
            }
            4 => {
                self.write_stack(lo_byte(self.regs.pc));
            }
            5 => {
                self.address_hi = self.read_mem(self.regs.pc);
                // Do not increment pc
            }
            6 => {
                self.regs.pc = make_address(self.address_hi, self.address_lo);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn rts(&mut self) {
        match self.cycle {
            2 => {
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => {
                let address = make_address(0x01, self.regs.sp);
                let pcl = self.read_mem(address);
                self.regs.pc = u16::from(pcl);
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            4 => {
                let address = make_address(0x01, self.regs.sp);
                let pch = self.read_mem(address);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
            }
            5 => {
                self.regs.pc = self.regs.pc.wrapping_add(1);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn brk(&mut self) {
        match self.cycle {
            2 => {
                self.write_stack(hi_byte(self.regs.pc));
            }
            3 => {
                self.write_stack(lo_byte(self.regs.pc));
            }
            4 => {
                self.write_stack(self.regs.p | (Flag::Break as u8) | (Flag::Reserved as u8));
            }
            5 => {
                let pcl = self.read_mem(0xfffe);
                self.regs.pc = u16::from(pcl);
            }
            6 => {
                let pch = self.read_mem(0xffff);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
                self.set_flag(Flag::IntDisable);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn rti(&mut self) {
        match self.cycle {
            2 => {
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => {
                let address = make_address(0x01, self.regs.sp);
                let p = self.read_mem(address);
                self.regs.sp = self.regs.sp.wrapping_add(1);
                self.regs.p = p;
            }
            4 => {
                let address = make_address(0x01, self.regs.sp);
                let pcl = self.read_mem(address);
                self.regs.pc = u16::from(pcl);
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            5 => {
                let address = make_address(0x01, self.regs.sp);
                let pch = self.read_mem(address);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn branch(&mut self, flag: Flag, value: bool) {
        match self.cycle {
            2 => {
                let cond = self.test_flag(flag) == value;
                if cond {
                    let offset = self.data as i8;
                    let ea = if offset < 0 {
                        self.regs.pc.wrapping_sub((offset as i16).abs() as u16)
                    } else {
                        self.regs.pc.wrapping_add(offset as u16)
                    };
                    self.regs.pc = ea;
                } else {
                    self.load_next_program();
                }
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn nop(&mut self) {}

    fn ane(&mut self) {
        let result = self.regs.a & self.regs.x & self.data;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn anx(&mut self) {
        let result = self.regs.a & self.data;
        self.regs.a = result;
        self.regs.x = result;
        self.set_nz(result);
    }

    fn alr(&mut self) {
        let value = self.regs.a & self.data;
        self.update_flag(Flag::Carry, (value & 0x01) != 0);
        let result = value >> 1;
        self.regs.a = result;
        self.set_nz(result);
    }

    fn axs(&mut self) {
        let result = ((self.regs.a & self.regs.x) as u16).wrapping_sub(self.data as u16);
        self.update_flag(Flag::Carry, result < 0x100);
        self.regs.x = (result & 0xff) as u8;
        self.set_nz((result & 0xff) as u8);
    }

    fn lax(&mut self) {
        let data = self.data;
        self.regs.a = data;
        self.regs.x = data;
        self.set_nz(data);
    }

    fn lse(&mut self) {
        let value = self.data;
        self.update_flag(Flag::Carry, (value & 0x01) != 0);
        let result = self.regs.a ^ (value >> 1);
        self.regs.a = result;
        self.set_nz(result);
        // tick_fn();
        // tick_fn();
    }

    // -- Interrupts

    fn irq(&mut self) {
        match self.cycle {
            2 => {
                self.write_stack(hi_byte(self.regs.pc));
            }
            3 => {
                self.write_stack(lo_byte(self.regs.pc));
            }
            4 => {
                self.write_stack(self.regs.p & 0xef);
            }
            5 => {
                let pcl = self.read_mem(0xfffe);
                self.regs.pc = u16::from(pcl);
            }
            6 => {
                let pch = self.read_mem(0xffff);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
                self.set_flag(Flag::IntDisable);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn nmi(&mut self) {
        match self.cycle {
            2 => {
                self.write_stack(hi_byte(self.regs.pc));
            }
            3 => {
                self.write_stack(lo_byte(self.regs.pc));
            }
            4 => {
                self.write_stack(self.regs.p & 0xef);
            }
            5 => {
                let pcl = self.read_mem(0xfffa);
                self.regs.pc = u16::from(pcl);
            }
            6 => {
                let pch = self.read_mem(0xfffb);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
                self.set_flag(Flag::IntDisable);
                self.nmi_line.borrow_mut().reset();
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    fn rst(&mut self) {
        match self.cycle {
            2 => {}
            3 => {}
            4 => {
                let pcl = self.read_mem(0xfffc);
                self.regs.pc = u16::from(pcl);
            }
            5 => {
                let pch = self.read_mem(0xfffd);
                self.regs.pc = make_address(pch, self.regs.pc as u8);
            }
            _ => panic!("invalid cycle {}", self.cycle),
        }
    }

    // -- Flag Ops

    #[inline]
    fn clear_flag(&mut self, flag: Flag) {
        self.regs.p &= !(flag as u8);
    }

    #[inline]
    fn set_flag(&mut self, flag: Flag) {
        self.regs.p |= flag as u8;
    }

    #[inline]
    fn set_nz(&mut self, value: u8) {
        self.update_flag(Flag::Negative, value & 0x80 != 0);
        self.update_flag(Flag::Zero, value == 0);
    }

    #[inline]
    fn update_flag(&mut self, flag: Flag, value: bool) {
        if value {
            self.regs.p |= flag as u8;
        } else {
            self.regs.p &= !(flag as u8);
        }
    }

    #[inline]
    fn test_flag(&self, flag: Flag) -> bool {
        (self.regs.p & (flag as u8)) != 0
    }

    // -- Memory Ops

    pub fn read_mem(&self, address: u16) -> u8 {
        let value = match address {
            0x0000 => self.io_port.borrow().get_direction(),
            0x0001 => self.io_port.borrow().get_value() & 0x3f,
            _ => self.mem.borrow().read(address),
        };
        value
    }

    pub fn write_mem(&mut self, address: u16, value: u8) {
        match address {
            0x0000 => self.io_port.borrow_mut().set_direction(value),
            0x0001 => self.io_port.borrow_mut().set_value(value),
            _ => {}
        }
        self.mem.borrow_mut().write(address, value);
    }

    #[inline]
    pub fn write_stack(&mut self, value: u8) {
        let address = make_address(0x01, self.regs.sp);
        self.write_mem(address, value);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
    }
}

impl Cpu for Cpu6510 {
    fn get_register(&self, reg: Register) -> u8 {
        match reg {
            Register::A => self.regs.a,
            Register::X => self.regs.x,
            Register::Y => self.regs.y,
            Register::SP => self.regs.sp,
            Register::PCL => self.regs.pc as u8,
            Register::PCH => (self.regs.pc >> 8) as u8,
            Register::P => self.regs.p,
        }
    }

    fn set_register(&mut self, reg: Register, value: u8) {
        match reg {
            Register::A => {
                self.regs.a = value;
            }
            Register::X => {
                self.regs.x = value;
            }
            Register::Y => {
                self.regs.y = value;
            }
            Register::SP => {
                self.regs.sp = value;
            }
            Register::PCL => {
                self.regs.pc = (self.regs.pc & 0xff00) | u16::from(value);
            }
            Register::PCH => {
                self.regs.pc = (u16::from(value) << 8) | (self.regs.pc & 0xff);
            }
            Register::P => {
                self.regs.p = value;
            }
        }
    }

    fn get_pc(&self) -> u16 {
        match self.cycle {
            1 => self.regs.pc.wrapping_sub(1),
            _ => self.regs.pc
        }
    }

    fn set_pc(&mut self, value: u16) {
        self.regs.pc = value;
        self.uops = load_program(ProgramId::Start);
        self.cycle = 0;
    }

    fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.get_pc()
    }

    fn step(&mut self, tick_fn: &TickFn) {
        self.last_pc = self.get_pc();
        let mut is_done = false;
        while !is_done {
            self.clock();
            tick_fn();
            is_done = self.cycle == 1;
        }
    }

    fn reset(&mut self) {
        self.regs.reset();
        self.address_hi = 0;
        self.address_lo = 0;
        self.data = 0;
        self.page_cross = false;
        self.last_nmi = false;
        self.last_pc = 0;
        self.io_port.borrow_mut().set_value(0xff);
        self.irq_line.borrow_mut().reset();
        self.nmi_line.borrow_mut().reset();
        self.write(0x0000, 0b_0010_1111);
        self.write(0x0001, 0b_0001_1111);
        self.opcode = 0;
        self.uops = load_program(ProgramId::Reset);
        self.cycle = 0;
    }

    // -- I/O

    fn read(&self, address: u16) -> u8 {
        self.read_mem(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.write_mem(address, value);
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
            if (self.regs.p & Flag::Break as u8) != 0 {
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

#[inline]
fn make_address(hi: u8, lo: u8) -> u16 {
    u16::from(hi) << 8 | u16::from(lo)
}

#[inline]
fn lo_byte(data: u16) -> u8 {
    data as u8
}

#[inline]
fn hi_byte(data: u16) -> u8 {
    (data >> 8) as u8
}
