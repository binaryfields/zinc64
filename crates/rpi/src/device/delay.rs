// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use cortex_a::{asm, regs::*};

pub fn get_counter() -> u64 {
    CNTPCT_EL0.get()
}

pub fn get_counter_freq() -> u32 {
    CNTFRQ_EL0.get()
}

pub fn wait_cycles(cycles: u32) {
    for _ in 0..cycles {
        asm::nop();
    }
}

pub fn wait_msec(ms: u32) {
    let frq = CNTFRQ_EL0.get();
    let tval = (u64::from(frq) * u64::from(ms * 1000) / 1_000_000) as u32;
    CNTP_TVAL_EL0.set(tval);
    CNTP_CTL_EL0.modify(CNTP_CTL_EL0::ENABLE::SET + CNTP_CTL_EL0::IMASK::SET);
    loop {
        if CNTP_CTL_EL0.is_set(CNTP_CTL_EL0::ISTATUS) {
            break;
        }
    }
    CNTP_CTL_EL0.modify(CNTP_CTL_EL0::ENABLE::CLEAR);
}

pub fn wait_counter(target: u64) {
    loop {
        if CNTPCT_EL0.get() >= target {
            break;
        }
    }
}
