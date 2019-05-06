// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::RefCell;
use std::rc::Rc;

use zinc64_core::{Chip, IoPort, IrqLine, Pin};
use zinc64_emu::io::cia::{reg, Mode};
use zinc64_emu::io::Cia;

fn setup_cia() -> Cia {
    let cia_flag = Rc::new(RefCell::new(Pin::new_low()));
    let cia_port_a = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cia_port_b = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
    let keyboard_matrix = Rc::new(RefCell::new([0xff; 16]));
    let mut cia = Cia::new(
        Mode::Cia1,
        None,
        None,
        Some(keyboard_matrix),
        cia_port_a,
        cia_port_b,
        cia_flag,
        cpu_irq,
    );
    cia.reset();
    cia
}

#[test]
fn cia1_tb123_00_01() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x01);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x08, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x07, cia.read(reg::TBLO));
    cia.clock();
}

#[test]
fn cia1_tb123_00_10() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    cia.write(reg::TBLO, 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x10);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
}

#[test]
fn cia1_tb123_00_11() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    cia.write(reg::TBLO, 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x00);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x11);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
}

#[test]
fn cia1_tb123_01_11() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    cia.write(reg::TBLO, 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x11);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x09, cia.read(reg::TBLO));
    cia.clock();
}

#[test]
fn cia1_tb123_01_10() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    cia.write(reg::TBLO, 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x10);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x0a, cia.read(reg::TBLO));
    cia.clock();
}

#[test]
fn cia1_tb123_01_00() {
    let mut cia = setup_cia();
    cia.write(reg::TBLO, 0x09);
    cia.write(reg::TBHI, 0x00);
    cia.clock();
    cia.clock();
    cia.write(reg::TBLO, 0x0a);
    // STA $dd0f #1 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x01);
    cia.clock();
    // STA $dd0f #2 - 4 cycles
    for _i in 0..3 {
        cia.clock();
    }
    cia.write(reg::CRB, 0x00);
    cia.clock();
    // DD06 sequence
    assert_eq!(0x06, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x05, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x05, cia.read(reg::TBLO));
    cia.clock();
    assert_eq!(0x05, cia.read(reg::TBLO));
    cia.clock();
}
