// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64_engine::chip::cia::{Cia, CiaPins, Mode, PortInput, reg};
use zinc64_engine::signal::{IrqLine, Pin};

struct Rig {
    cia: Cia,
    flag: Pin,
    irq: IrqLine,
}

impl Rig {
    fn clock(&mut self) {
        self.cia.clock(&mut CiaPins {
            flag: &mut self.flag,
            irq: &mut self.irq,
        });
    }

    fn read(&mut self, reg: u8) -> u8 {
        let input = PortInput { a: 0xff, b: 0xff };
        self.cia.read(
            reg,
            input,
            &mut CiaPins {
                flag: &mut self.flag,
                irq: &mut self.irq,
            },
        )
    }

    fn write(&mut self, reg: u8, value: u8) {
        self.cia.write(reg, value);
    }
}

fn setup_cia() -> Rig {
    let mut rig = Rig {
        cia: Cia::new(Mode::Cia1),
        flag: Pin::new_low(),
        irq: IrqLine::new("irq"),
    };
    rig.cia.reset(&mut CiaPins {
        flag: &mut rig.flag,
        irq: &mut rig.irq,
    });
    rig
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
