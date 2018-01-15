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

use super::Cpu6510;

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
    pub fn ea(&self, cpu: &Cpu6510, rmw: bool, tick_fn: &TickFn) -> u16 {
        match *self {
            Operand::Accumulator => panic!("Illegal op for addressing mode {}", "accumulator"),
            Operand::Immediate(_) => panic!("Illegal op for addressing mode {}", "immediate"),
            Operand::ZeroPage(address) => address as u16,
            Operand::ZeroPageX(address) => {
                if !rmw {
                    // FIXME cpu: rmw
                    tick_fn();
                }
                address.wrapping_add(cpu.get_x()) as u16
            }
            Operand::ZeroPageY(address) => {
                tick_fn();
                address.wrapping_add(cpu.get_y()) as u16
            }
            Operand::Absolute(address) => address,
            Operand::AbsoluteX(address) => {
                if rmw {
                    tick_fn();
                }
                address.wrapping_add(cpu.get_x() as u16)
            }
            Operand::AbsoluteY(address) => {
                if rmw {
                    tick_fn();
                }
                address.wrapping_add(cpu.get_y() as u16)
            }
            Operand::IndirectX(address) => {
                let calc_address = address.wrapping_add(cpu.get_x()) as u16;
                tick_fn();
                cpu.read_word(calc_address, tick_fn)
            }
            Operand::IndirectY(address) => {
                if rmw {
                    tick_fn();
                }
                cpu.read_word(address as u16, tick_fn)
                    .wrapping_add(cpu.get_y() as u16)
            }
            Operand::Indirect(address) => cpu.read_word(address, tick_fn),
            Operand::Relative(offset) => {
                let ea = if offset < 0 {
                    cpu.get_pc().wrapping_sub((offset as i16).abs() as u16)
                } else {
                    cpu.get_pc().wrapping_add(offset as u16)
                };
                if cpu.get_pc() & 0xff00 != ea & 0xff00 {
                    tick_fn();
                }
                ea
            },
        }
    }

    #[inline]
    pub fn get(&self, cpu: &Cpu6510, tick_fn: &TickFn) -> u8 {
        match *self {
            Operand::Accumulator => cpu.get_a(),
            Operand::Immediate(value) => value,
            Operand::Indirect(_) => panic!("illegal op for addressing mode {}", "indirect"),
            Operand::Relative(_) => panic!("illegal op for addressing mode {}", "relative"),
            _ => {
                let address = self.ea(cpu, false, tick_fn);
                cpu.read(address, tick_fn)
            }
        }
    }

    #[inline]
    pub fn set(&self, cpu: &mut Cpu6510, value: u8, rmw: bool, tick_fn: &TickFn) {
        match *self {
            Operand::Accumulator => cpu.set_a(value),
            Operand::Immediate(_) => panic!("illegal op for addressing mode {}", "immediate"),
            Operand::Indirect(_) => panic!("illegal op for addressing mode {}", "indirect"),
            Operand::Relative(_) => panic!("illegal op for addressing mode {}", "relative"),
            _ => {
                let address = self.ea(cpu, rmw, tick_fn);
                cpu.write(address, value, tick_fn);
            }
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
    fn ea_zeropage() {
        let tick_fn: TickFn = Box::new(move || {});
        let cpu = setup_cpu();
        let op = Operand::ZeroPage(0x10);
        assert_eq!(0x0010, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_zeropage_x() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_x(0x01);
        let op = Operand::ZeroPageX(0x10);
        assert_eq!(0x0011, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_zeropage_x_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_x(0x03);
        let op = Operand::ZeroPageX(0xff);
        assert_eq!(0x0002, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_zeropage_y() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_y(0x01);
        let op = Operand::ZeroPageY(0x10);
        assert_eq!(0x0011, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_zeropage_y_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_y(0x03);
        let op = Operand::ZeroPageY(0xff);
        assert_eq!(0x0002, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_absolute() {
        let tick_fn: TickFn = Box::new(move || {});
        let cpu = setup_cpu();
        let op = Operand::Absolute(0x0100);
        assert_eq!(0x0100, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_absolute_x() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_x(0x01);
        let op = Operand::AbsoluteX(0x0100);
        assert_eq!(0x0101, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_absolute_x_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_x(0x03);
        let op = Operand::AbsoluteX(0xffff);
        assert_eq!(0x0002, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_absolute_y() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_y(0x01);
        let op = Operand::AbsoluteY(0x0100);
        assert_eq!(0x0101, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_absolute_y_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_y(0x03);
        let op = Operand::AbsoluteY(0xffff);
        assert_eq!(0x0002, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_indirect_x() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0006, 0x00, &tick_fn);
        cpu.write(0x0007, 0x16, &tick_fn);
        cpu.set_x(0x05);
        let op = Operand::IndirectX(0x01);
        assert_eq!(0x1600, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_indirect_x_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0006, 0x00, &tick_fn);
        cpu.write(0x0007, 0x16, &tick_fn);
        cpu.set_x(0x07);
        let op = Operand::IndirectX(0xff);
        assert_eq!(0x1600, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_indirect_y() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0006, 0x00, &tick_fn);
        cpu.write(0x0007, 0x16, &tick_fn);
        cpu.set_y(0x05);
        let op = Operand::IndirectY(0x06);
        assert_eq!(0x1605, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_indirect_y_wrapping() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0006, 0xff, &tick_fn);
        cpu.write(0x0007, 0xff, &tick_fn);
        cpu.set_y(0x06);
        let op = Operand::IndirectY(0x06);
        assert_eq!(0x0005, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_relative_pos() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_pc(0x0100);
        let op = Operand::Relative(0x01);
        assert_eq!(0x0101, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_relative_neg() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_pc(0x0100);
        let op = Operand::Relative(-0x01);
        assert_eq!(0x00ff, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn ea_relative_neg_max() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_pc(0x0505);
        let op = Operand::Relative(-128);
        assert_eq!(0x0485, op.ea(&cpu, false, &tick_fn));
    }

    #[test]
    fn get_accumulator() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.set_a(0xab);
        let op = Operand::Accumulator;
        assert_eq!(0xab, op.get(&cpu, &tick_fn));
    }

    #[test]
    fn get_immediate() {
        let tick_fn: TickFn = Box::new(move || {});
        let cpu = setup_cpu();
        let op = Operand::Immediate(0xab);
        assert_eq!(0xab, op.get(&cpu, &tick_fn));
    }

    #[test]
    fn get_zeropage() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0010, 0xab, &tick_fn);
        let op = Operand::ZeroPage(0x10);
        assert_eq!(0xab, op.get(&cpu, &tick_fn));
    }

    #[test]
    fn get_absolute() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        cpu.write(0x0100, 0xab, &tick_fn);
        let op = Operand::Absolute(0x0100);
        assert_eq!(0xab, op.get(&cpu, &tick_fn));
    }

    #[test]
    fn set_zeropage() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let op = Operand::ZeroPage(0x10);
        op.set(&mut cpu, 0xab, false, &tick_fn);
        assert_eq!(0xab, cpu.read(0x0010, &tick_fn));
    }

    #[test]
    fn set_absolute() {
        let tick_fn: TickFn = Box::new(move || {});
        let mut cpu = setup_cpu();
        let op = Operand::Absolute(0x0100);
        op.set(&mut cpu, 0xab, false, &tick_fn);
        assert_eq!(0xab, cpu.read(0x0100, &tick_fn));
    }
}
