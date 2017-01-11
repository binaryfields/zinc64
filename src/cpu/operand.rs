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

use super::Cpu;

// Spec: INSTRUCTION ADDRESSING MODES AND RELATED EXECUTION TIMES (p. 255)
// Design:
//    Inspired by UAE handling of operands with instr_params, and functions
//    GetEA, GetFromEA, StoreToEA. Use Operand variants to specify addressing mode
//    and applicable parameter

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

impl Operand {
    pub fn ea(&self, cpu: &Cpu) -> u16 {
        match *self {
            Operand::Accumulator => panic!("Illegal op for addressing mode {}", "accumulator"),
            Operand::Immediate(_) => panic!("Illegal op for addressing mode {}", "immediate"),
            Operand::ZeroPage(address) => address as u16,
            Operand::ZeroPageX(address) => address.wrapping_add(cpu.get_x()) as u16,
            Operand::ZeroPageY(address) => address.wrapping_add(cpu.get_y()) as u16,
            Operand::Absolute(address) => address,
            Operand::AbsoluteX(address) => address.wrapping_add(cpu.get_x() as u16),
            Operand::AbsoluteY(address) => address.wrapping_add(cpu.get_y() as u16),
            Operand::IndirectX(address) => cpu.read_word(address.wrapping_add(cpu.get_x()) as u16),
            Operand::IndirectY(address) => cpu.read_word(address as u16).wrapping_add(cpu.get_y() as u16),
            Operand::Indirect(address) => cpu.read_word(address),
            Operand::Relative(offset) if offset < 0 => cpu.get_pc().wrapping_sub((offset as i16).abs() as u16),
            Operand::Relative(offset) => cpu.get_pc().wrapping_add(offset as u16),
        }
    }

    pub fn get(&self, cpu: &Cpu) -> u8 {
        match *self {
            Operand::Accumulator => cpu.get_a(),
            Operand::Immediate(value) => value,
            Operand::Indirect(_) => panic!("illegal op for addressing mode {}", "indirect"),
            Operand::Relative(_) => panic!("illegal op for addressing mode {}", "relative"),
            _ => {
                let address = self.ea(cpu);
                cpu.read(address)
            },
        }
    }

    pub fn set(&self, cpu: &mut Cpu, value: u8) {
        match *self {
            Operand::Accumulator => cpu.set_a(value),
            Operand::Immediate(_) => panic!("illegal op for addressing mode {}", "immediate"),
            Operand::Indirect(_) => panic!("illegal op for addressing mode {}", "indirect"),
            Operand::Relative(_) => panic!("illegal op for addressing mode {}", "relative"),
            _ => {
                let address = self.ea(cpu);
                cpu.write(address, value)
            },
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cpu::Cpu;
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

    #[test]
    fn address_zeropage() {
        let cpu = setup_cpu().unwrap();
        let op = Operand::ZeroPage(0x10);
        assert_eq!(0x0010, op.ea(&cpu));
    }

    #[test]
    fn address_zeropagex() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_x(0x01);
        let op = Operand::ZeroPageX(0x10);
        assert_eq!(0x0011, op.ea(&cpu));
    }

    #[test]
    fn address_zeropagex_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_x(0x03);
        let op = Operand::ZeroPageX(0xff);
        assert_eq!(0x0002, op.ea(&cpu));
    }

    #[test]
    fn address_zeropagey() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_y(0x01);
        let op = Operand::ZeroPageY(0x10);
        assert_eq!(0x0011, op.ea(&cpu));
    }

    #[test]
    fn address_zeropagey_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_y(0x03);
        let op = Operand::ZeroPageY(0xff);
        assert_eq!(0x0002, op.ea(&cpu));
    }

    #[test]
    fn address_absolute() {
        let cpu = setup_cpu().unwrap();
        let op = Operand::Absolute(0x0100);
        assert_eq!(0x0100, op.ea(&cpu));
    }

    #[test]
    fn address_absolutex() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_x(0x01);
        let op = Operand::AbsoluteX(0x0100);
        assert_eq!(0x0101, op.ea(&cpu));
    }

    #[test]
    fn address_absolutex_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_x(0x03);
        let op = Operand::AbsoluteX(0xffff);
        assert_eq!(0x0002, op.ea(&cpu));
    }

    #[test]
    fn address_absolutey() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_y(0x01);
        let op = Operand::AbsoluteY(0x0100);
        assert_eq!(0x0101, op.ea(&cpu));
    }

    #[test]
    fn address_absolutey_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_y(0x03);
        let op = Operand::AbsoluteY(0xffff);
        assert_eq!(0x0002, op.ea(&cpu));
    }

    #[test]
    fn address_indirectx() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0006, 0x00);
        cpu.write(0x0007, 0x16);
        cpu.set_x(0x05);
        let op = Operand::IndirectX(0x01);
        assert_eq!(0x1600, op.ea(&cpu));
    }

    #[test]
    fn address_indirectx_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0006, 0x00);
        cpu.write(0x0007, 0x16);
        cpu.set_x(0x07);
        let op = Operand::IndirectX(0xff);
        assert_eq!(0x1600, op.ea(&cpu));
    }

    #[test]
    fn address_indirecty() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0006, 0x00);
        cpu.write(0x0007, 0x16);
        cpu.set_y(0x05);
        let op = Operand::IndirectY(0x06);
        assert_eq!(0x1605, op.ea(&cpu));
    }

    #[test]
    fn address_indirecty_wrapping() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0006, 0xff);
        cpu.write(0x0007, 0xff);
        cpu.set_y(0x06);
        let op = Operand::IndirectY(0x06);
        assert_eq!(0x0005, op.ea(&cpu));
    }

    #[test]
    fn address_relative_pos() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_pc(0x0100);
        let op = Operand::Relative(0x01);
        assert_eq!(0x0101, op.ea(&cpu));
    }

    #[test]
    fn address_relative_neg() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_pc(0x0100);
        let op = Operand::Relative(-0x01);
        assert_eq!(0x00ff, op.ea(&cpu));
    }

    #[test]
    fn address_relative_neg_max() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_pc(0x0505);
        let op = Operand::Relative(-128);
        assert_eq!(0x0485, op.ea(&cpu));
    }

    #[test]
    fn get_accumulator() {
        let mut cpu = setup_cpu().unwrap();
        cpu.set_a(0xab);
        let op = Operand::Accumulator;
        assert_eq!(0xab, op.get(&cpu));
    }

    #[test]
    fn get_immediate() {
        let cpu = setup_cpu().unwrap();
        let op = Operand::Immediate(0xab);
        assert_eq!(0xab, op.get(&cpu));
    }

    #[test]
    fn get_zeropage() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0010, 0xab);
        let op = Operand::ZeroPage(0x10);
        assert_eq!(0xab, op.get(&cpu));
    }

    #[test]
    fn get_absolute() {
        let mut cpu = setup_cpu().unwrap();
        cpu.write(0x0100, 0xab);
        let op = Operand::Absolute(0x0100);
        assert_eq!(0xab, op.get(&cpu));
    }

    #[test]
    fn set_zeropage() {
        let mut cpu = setup_cpu().unwrap();
        let op = Operand::ZeroPage(0x10);
        op.set(&mut cpu, 0xab);
        assert_eq!(0xab, cpu.read(0x0010));
    }

    #[test]
    fn set_absolute() {
        let mut cpu = setup_cpu().unwrap();
        let op = Operand::Absolute(0x0100);
        op.set(&mut cpu, 0xab);
        assert_eq!(0xab, cpu.read(0x0100));
    }
}
