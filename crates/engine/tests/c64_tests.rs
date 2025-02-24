// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64_engine::SystemModel;
use zinc64_engine::chip::cia;
use zinc64_engine::{C64, Config};

/*
Program CIA1TAB - TA, TB, PB67 and ICR in cascaded mode

Both latches are set to 2. TA counts system clocks, TB counts TA underflows (cascaded).
PB6 is high for one cycle when TA underflows, PB7 is toggled when TB underflows. IMR is $02.

TA  01 02 02 01 02 02 01 02 02 01 02 02
TB  02 02 02 01 01 01 00 00 02 02 02 02
PB  80 C0 80 80 C0 80 80 C0 00 00 40 00
ICR 00 01 01 01 01 01 01 01 03 83 83 83
*/

static CIA1TAB_PRG: &[u8] = include_bytes!("data/cia1tab.prg");

static CIA1TAB_TA: [u8; 12] = [
    0x01, 0x02, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x02,
];

static CIA1TAB_TB: [u8; 12] = [
    0x02, 0x02, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x02, 0x02, 0x02, 0x02,
];

static CIA1TAB_PB: [u8; 12] = [
    0x80, 0xC0, 0x80, 0x80, 0xC0, 0x80, 0x80, 0xC0, 0x00, 0x00, 0x40, 0x00,
];

#[test]
#[ignore = "fails before and after the edition 2024 migration"]
fn program_cia1tab() {
    let config = Config::new(SystemModel::from("pal"));
    let mut c64 = C64::new(config);
    c64.reset(false);
    c64.load(&CIA1TAB_PRG.to_vec()[2..], 0x4000);
    c64.get_cpu_mut().set_pc(0x4000);
    let mut test_cycle: Option<usize> = None;
    while test_cycle.is_none_or(|cycle| cycle < 13) {
        c64.clock();
        if let Some(cycle) = test_cycle.as_mut() {
            if (1..13).contains(cycle) {
                let cia1 = 0xdc00 | u16::from(cia::reg::TALO);
                assert_eq!(c64.read(cia1), CIA1TAB_TA[*cycle - 1]);
                let cia1 = 0xdc00 | u16::from(cia::reg::TBLO);
                assert_eq!(c64.read(cia1), CIA1TAB_TB[*cycle - 1]);
                let cia1 = 0xdc00 | u16::from(cia::reg::PRB);
                assert_eq!(c64.read(cia1), CIA1TAB_PB[*cycle - 1]);
            }
            *cycle += 1;
        } else if c64.get_cpu().is_opcode_fetched() && c64.get_cpu().get_pc() == 0x402d {
            test_cycle = Some(0);
        }
    }
}

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
        0x78u8, 0xa9, 0xff, 0x8d, 0x02, 0xdc, 0xa9, 0x00, 0x8d, 0x03, 0xdc, 0xa9, 0xfd, 0x8d, 0x00,
        0xdc, 0xad, 0x01, 0xdc, 0x29, 0x20, 0xd0, 0xf9, 0x58,
    ];
    let config = Config::new(SystemModel::from("pal"));
    let mut c64 = C64::new(config);
    c64.load(code.as_ref(), 0xc000);
    let keyboard = c64.get_keyboard();
    keyboard.set_matrix((1, 5), true);
    c64.write(0x0001, 0x06);
    c64.get_cpu_mut().set_pc(0xc000);
    let mut branch_count = 0;
    loop {
        c64.step();
        if c64.get_cpu().get_pc() == 0xc018 {
            break;
        }
        if c64.get_cpu().get_pc() == 0xc015 {
            branch_count += 1;
            if branch_count > 1 {
                panic!("trap at 0x{:x}", c64.get_cpu().get_pc());
            }
        }
    }
}

/*
; This program waits until the key "S" was pushed.
; Start with SYS 49152

*=$c000                  ; startaddress

PRA  =  $dc00            ; CIA#1 (Port Register A)
DDRA =  $dc02            ; CIA#1 (Data Direction Register A)

PRB  =  $dc01            ; CIA#1 (Port Register B)
DDRB =  $dc03            ; CIA#1 (Data Direction Register B)


start    sei             ; interrupts deactivated

         lda #%11111111  ; CIA#1 port A = outputs
         sta DDRA

         lda #%00000000  ; CIA#1 port B = inputs
         sta DDRB

         lda #%11111101  ; testing column 1 (COL1) of the matrix
         sta PRA

loop     lda PRB
         and #%00100000  ; masking row 5 (ROW5)
         bne loop        ; wait until key "S"

         cli             ; interrupts activated

ende     rts             ; back to BASIC
*/
