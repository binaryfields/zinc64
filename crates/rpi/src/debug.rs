// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]

use alloc::prelude::*;
use zinc64_system::system::C64;

pub fn dump_screen(c64: &C64) {
    let vm_base = sys_screen_ptr(c64);
    let data = mem_read(c64, vm_base, vm_base.wrapping_add(1000));
    let mut counter = 0;
    info!(target: "screen", "Displaying 40x25 screen at ${:04x}", vm_base);
    let mut line = String::new();
    for value in data {
        let ascii = match screen_code_to_ascii(value) {
            0 => 46,
            v => v,
        };
        line.push(char::from(ascii));
        counter += 1;
        if counter % 40 == 0 {
            info!(target: "screen", "{}", line);
            line.clear();
        }
    }
    if counter % 40 != 0 {
        info!(target: "screen", "{}", line);
        line.clear();
    }
}

fn mem_read(c64: &C64, start: u16, end: u16) -> Vec<u8> {
    let cpu = c64.get_cpu();
    let mut buffer = Vec::new();
    let mut address = start;
    while address < end {
        buffer.push(cpu.read(address));
        address = address.wrapping_add(1);
    }
    buffer
}

fn screen_code_to_ascii(code: u8) -> u8 {
    match code {
        0 => 64,
        1...31 => 96 + code,
        32...90 => code,
        _ => 0,
    }
}

fn sys_screen_ptr(c64: &C64) -> u16 {
    let cia2 = c64.get_cia_2();
    let vic = c64.get_vic();
    let cia2_port_a = cia2.borrow_mut().read(0x00);
    let vm = (((vic.borrow_mut().read(0x18) & 0xf0) >> 4) as u16) << 10;
    let vm_base = ((!cia2_port_a & 0x03) as u16) << 14 | vm;
    vm_base
}
