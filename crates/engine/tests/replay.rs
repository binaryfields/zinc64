// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs;
use std::hash::{DefaultHasher, Hasher};

use zinc64_engine::media::{Autostart, AutostartMethod, Media, Mode};
use zinc64_engine::peripheral::joystick::Button;
use zinc64_engine::{C64, Config, SystemModel};

const ALL_CHIPS_PRG: &[u8] = &[
    0x0b, 0x08, 0x0a, 0x00, 0x9e, 0x32, 0x30, 0x36, 0x34, 0x00, 0x00, 0x00, // 10 SYS2064
    0x00, 0x00, 0x00, // pad to $0810
    0x78, // sei
    0xa9, 0x8f, 0x8d, 0x14, 0x03, // lda #<irq, sta $0314
    0xa9, 0x08, 0x8d, 0x15, 0x03, // lda #>irq, sta $0315
    0xa9, 0x80, 0x8d, 0x12, 0xd0, // lda #$80, sta $d012
    0xad, 0x11, 0xd0, 0x29, 0x7f, 0x8d, 0x11, 0xd0, // clear raster bit 8
    0xa9, 0x01, 0x8d, 0x1a, 0xd0, // enable the raster irq
    0x8d, 0x15, 0xd0, // enable sprite 0
    0xa9, 0x0d, 0x8d, 0xf8, 0x07, // sprite 0 data at $0340
    0xa9, 0x64, 0x8d, 0x00, 0xd0, 0x8d, 0x01, 0xd0, // sprite 0 at (100, 100)
    0xa2, 0x3f, 0xa9, 0xff, 0x9d, 0x40, 0x03, 0xca, 0x10, 0xfa, // fill the sprite
    0xa9, 0x0f, 0x8d, 0x18, 0xd4, // volume
    0xa9, 0x19, 0x8d, 0x01, 0xd4, // voice 1 frequency
    0xa9, 0x09, 0x8d, 0x05, 0xd4, // attack/decay
    0xa9, 0x00, 0x8d, 0x06, 0xd4, // sustain/release
    0xa9, 0x11, 0x8d, 0x04, 0xd4, // triangle, gate on
    0xa9, 0xff, 0x8d, 0x04, 0xdd, 0xa9, 0x10, 0x8d, 0x05, 0xdd, // cia 2 timer a latch
    0xa9, 0x81, 0x8d, 0x0d, 0xdd, // enable the timer a nmi
    0xa9, 0x11, 0x8d, 0x0e, 0xdd, // start timer a
    0xa9, 0x36, 0x85, 0x01, // bank basic out
    0xa9, 0x5a, 0x8d, 0x00, 0xa0, // write the ram under basic
    0xad, 0x00, 0xa0, 0x8d, 0x00, 0xc1, // read it back into $c100
    0xa9, 0x37, 0x85, 0x01, // bank basic in
    0xad, 0x00, 0xa0, 0x8d, 0x01, 0xc1, // read the rom into $c101
    0x58, 0x60, // cli, rts
    0xad, 0x19, 0xd0, 0x29, 0x01, 0xf0, 0x1d, // irq: not a raster irq?
    0x8d, 0x19, 0xd0, // acknowledge it
    0xee, 0x21, 0xd0, // next background color
    0xad, 0x00, 0xdd, 0x49, 0x01, 0x8d, 0x00, 0xdd, // flip the vic bank
    0xad, 0x00, 0xdc, 0x8d, 0x02, 0xc1, // sample cia 1 port a
    0xad, 0x01, 0xdc, 0x8d, 0x03, 0xc1, // sample cia 1 port b
    0x4c, 0x81, 0xea, // jmp $ea81
    0x4c, 0x31, 0xea, // jmp $ea31
];

struct Replay {
    c64: C64,
    sample_hash: DefaultHasher,
}

impl Replay {
    fn new(config: Config) -> Self {
        let mut c64 = C64::new(config);
        c64.reset(true);
        Self {
            c64,
            sample_hash: DefaultHasher::new(),
        }
    }

    fn run(&mut self, frames: u32) {
        for _ in 0..frames {
            self.c64.run_frame();
            for sample in self.c64.samples() {
                self.sample_hash.write_i16(*sample);
            }
        }
    }

    fn state_hash(&mut self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let visible = (0x0000..=0xcfff_u16)
            .chain(0xd800..=0xdbff)
            .chain(0xe000..=0xffff);
        for address in visible {
            hasher.write_u8(self.c64.read(address));
        }
        hasher.write(self.c64.frame());
        hasher.write_u64(self.sample_hash.finish());
        hasher.write_u16(self.c64.get_cpu().get_pc());
        hasher.finish()
    }
}

fn config() -> Config {
    Config::new_with_roms(
        SystemModel::from("pal"),
        &read("res/rom/basic.rom"),
        &read("res/rom/characters.rom"),
        &read("res/rom/kernal.rom"),
    )
}

fn read(path: &str) -> Vec<u8> {
    fs::read(format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap()
}

fn autostart(c64: &mut C64, data: &[u8], offset: u16) {
    let media = Media::Program {
        data: data.to_vec(),
        offset,
    };
    AutostartMethod::Run(Autostart::new(Mode::Run, media)).execute(c64);
}

#[test]
fn music_irq_plays() {
    let mut replay = Replay::new(config());
    let prg = read("bin/MusicIrq.prg");
    autostart(&mut replay.c64, &prg[2..], 0x0801);
    replay.run(400);
    assert_eq!(replay.state_hash(), 0x46059fd562774526);
}

#[test]
fn all_chips_program_drives_every_chip() {
    let mut replay = Replay::new(config());
    autostart(&mut replay.c64, ALL_CHIPS_PRG, 0x0801);
    replay.run(150);
    let joystick = replay.c64.get_joystick2_mut();
    joystick.press(Button::Up);
    joystick.press(Button::Fire);
    replay.run(150);
    assert_eq!(replay.state_hash(), 0xad7bcc50769d486a);
}
