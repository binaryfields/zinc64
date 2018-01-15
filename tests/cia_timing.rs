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

extern crate zinc64;

use std::cell::RefCell;
use std::rc::Rc;

use zinc64::core::{Chip, IoPort, IrqLine, Pin};
use zinc64::io::Cia;
use zinc64::io::cia::{Mode, Reg};

fn setup_cia() -> Cia {
    let cia_flag = Rc::new(RefCell::new(Pin::new_low()));
    let cia_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cia_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
    let keyboard_matrix = Rc::new(RefCell::new([0xff; 8]));
    let mut cia = Cia::new(
        Mode::Cia1,
        cia_flag,
        cia_port_a,
        cia_port_b,
        cpu_irq,
        None,
        None,
        keyboard_matrix,
    );
    cia.reset();
    cia
}

#[test]
fn cia1_tb123_00_01() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x01);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x08, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x07, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

#[test]
fn cia1_tb123_00_10() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    cia.write(Reg::TBLO.addr(), 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x10);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

#[test]
fn cia1_tb123_00_11() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    cia.write(Reg::TBLO.addr(), 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x11);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

#[test]
fn cia1_tb123_01_11() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    cia.write(Reg::TBLO.addr(), 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x11);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x09, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

#[test]
fn cia1_tb123_01_10() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    cia.write(Reg::TBLO.addr(), 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x10);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x0a, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

#[test]
fn cia1_tb123_01_00() {
    let mut cia = setup_cia();
    cia.write(Reg::TBLO.addr(), 0x09);
    cia.write(Reg::TBHI.addr(), 0x00);
    cia.clock();
    cia.clock();
    cia.write(Reg::TBLO.addr(), 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(Reg::CRB.addr(), 0x00);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x05, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x05, cia.read(Reg::TBLO.addr()));
    cia.clock();
    assert_eq!(0x05, cia.read(Reg::TBLO.addr()));
    cia.clock();
}

