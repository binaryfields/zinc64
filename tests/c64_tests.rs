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

use std::rc::Rc;

use zinc64::core::{SystemModel, TickFn};
use zinc64::system::{C64, ChipFactory, Config};

#[test]
fn exec_keyboard_read() {
    /*
    .c000  78         sei
    .c001  a9 ff      lda #$ff
    .c003  8d 02 dc   sta $dc02
    .c006  a9 00      lda #$00
    .c008  8d 03 dc   sta $dc03
    .c00b  a9 fd      lda #$fd
    .c00d  8d 00 dc   sta $dc00
    .c010  ad 01 dc   lda $dc01
    .c013  29 20      and #$20
    .c015  d0 f9      bne $c010
    .c017  58         cli
    */
    let code = [
        0x78u8, 0xa9, 0xff, 0x8d, 0x02, 0xdc, 0xa9, 0x00, 0x8d, 0x03, 0xdc, 0xa9, 0xfd, 0x8d,
        0x00, 0xdc, 0xad, 0x01, 0xdc, 0x29, 0x20, 0xd0, 0xf9, 0x58,
    ];
    let tick_fn: TickFn = Box::new(move || {});
    let config = Rc::new(Config::new(SystemModel::from("pal")));
    let factory = Box::new(ChipFactory::new(config.clone()));
    let mut c64 = C64::new(config.clone(), factory).unwrap();
    c64.load(&code.to_vec(), 0xc000);
    let keyboard = c64.get_keyboard();
    keyboard.borrow_mut().set_row(1, !(1 << 5));
    let cpu = c64.get_cpu();
    cpu.borrow_mut().write_debug(0x0001, 0x06);
    cpu.borrow_mut().set_pc(0xc000);
    let mut branch_count = 0;
    loop {
        c64.step(&tick_fn);
        if cpu.borrow().get_pc() == 0xc018 {
            break;
        }
        if cpu.borrow().get_pc() == 0xc015 {
            branch_count += 1;
            if branch_count > 1 {
                panic!("trap at 0x{:x}", cpu.borrow_mut().get_pc());
            }
        }
    }
}
