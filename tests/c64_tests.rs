// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use zinc64::io::cia;
use zinc64::system::{C64Factory, Config, C64};
use zinc64_core::{new_shared, SoundOutput, SystemModel, TickFn, VideoOutput};

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

static CIA1TAB_TA: [u8; 12] = [01, 02, 02, 01, 02, 02, 01, 02, 02, 01, 02, 02];

static CIA1TAB_TB: [u8; 12] = [02, 02, 02, 01, 01, 01, 00, 00, 02, 02, 02, 02];

static CIA1TAB_PB: [u8; 12] = [
    0x80, 0xC0, 0x80, 0x80, 0xC0, 0x80, 0x80, 0xC0, 0x00, 0x00, 0x40, 0x00,
];

struct NullSound;
impl SoundOutput for NullSound {
    fn reset(&self) {}
    fn write(&self, _samples: &[i16]) {}
}

struct NullVideo;
impl VideoOutput for NullVideo {
    fn get_dimension(&self) -> (usize, usize) {
        (0, 0)
    }
    fn reset(&mut self) {}
    fn write(&mut self, _index: usize, _color: u8) {}
}

#[test]
fn program_cia1tab() {
    let config = Rc::new(Config::new(SystemModel::from("pal")));
    let factory = Box::new(C64Factory::new(config.clone()));
    let video_output = new_shared(NullVideo {});
    let sound_output = Arc::new(NullSound {});
    let mut c64 = C64::build(config.clone(), &*factory, video_output, sound_output);
    c64.reset(false);
    let cia1_clone = c64.get_cia_1();
    let cia2_clone = c64.get_cia_2();
    let clock_clone = c64.get_clock();
    let test_flag = Rc::new(Cell::new(false));
    let test_flag_clone = test_flag.clone();
    let test_cycle = Rc::new(Cell::new(0));
    let test_cycle_clone = test_cycle.clone();
    let tick_fn: TickFn = Rc::new(move || {
        cia1_clone.borrow_mut().clock();
        cia2_clone.borrow_mut().clock();
        clock_clone.tick();
        if test_flag_clone.get() {
            if test_cycle_clone.get() >= 1 && test_cycle_clone.get() < 13 {
                let i = test_cycle_clone.get() - 1;
                assert_eq!(
                    cia1_clone.borrow_mut().read(cia::Reg::TALO.addr()),
                    CIA1TAB_TA[i]
                );
                assert_eq!(
                    cia1_clone.borrow_mut().read(cia::Reg::TBLO.addr()),
                    CIA1TAB_TB[i]
                );
                assert_eq!(
                    cia1_clone.borrow_mut().read(cia::Reg::PRB.addr()),
                    CIA1TAB_PB[i]
                );
            }
            test_cycle_clone.set(test_cycle_clone.get() + 1);
        }
    });
    c64.load(&CIA1TAB_PRG.to_vec()[2..].to_vec(), 0x4000);
    c64.get_cpu_mut().set_pc(0x4000);
    while test_cycle.get() < 13 {
        c64.step_internal(&tick_fn);
        if c64.get_cpu().get_pc() == 0x402d {
            test_flag.set(true);
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
    let config = Rc::new(Config::new(SystemModel::from("pal")));
    let factory = Box::new(C64Factory::new(config.clone()));
    let video_output = new_shared(NullVideo {});
    let sound_output = Arc::new(NullSound {});
    let mut c64 = C64::build(config.clone(), &*factory, video_output, sound_output);
    c64.load(&code.to_vec(), 0xc000);
    let keyboard = c64.get_keyboard();
    keyboard.borrow_mut().set_row(1, !(1 << 5));
    c64.get_cpu_mut().write(0x0001, 0x06);
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
#[test]
fn read_keyboard_s() {
    let keyboard_matrix = Rc::new(RefCell::new([0xff; 8]));
    let keyboard = Rc::new(RefCell::new(Keyboard::new(keyboard_matrix.clone())));
    keyboard.borrow_mut().reset();
    let mut cia = setup_cia_with_keyboard(keyboard_matrix.clone());
    keyboard.borrow_mut().enqueue("S");
    keyboard.borrow_mut().drain_event();
    cia.write(Reg::DDRA.addr(), 0xff);
    cia.write(Reg::DDRB.addr(), 0x00);
    cia.write(Reg::PRA.addr(), 0xfd);
    assert_eq!(!(1 << 5), cia.read(Reg::PRB.addr()));
}
*/

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
