// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs;
use std::sync::{Arc, Mutex};

use zinc64_engine::chip::{Register, SoundOutput};
use zinc64_engine::media::{Autostart, AutostartMethod, Media, Mode};
use zinc64_engine::peripheral::Tape;
use zinc64_engine::peripheral::cartridge::{Cartridge, Chip, ChipType, HwType};
use zinc64_engine::peripheral::joystick::{self, Button};
use zinc64_engine::{C64, Config, SystemModel};

const KEY_EVENT_CYCLES: u64 = 20_000;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

const CHIP_TOUR: &[u8] = &[
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

const MAGIC_DESK_BANK_0: &[u8] = &[
    0x09, 0x80, 0x09, 0x80, 0xc3, 0xc2, 0xcd, 0x38, 0x30, // vectors, CBM80
    0xa2, 0x00, 0xbd, 0x20, 0x80, 0x9d, 0x00, 0xc0, // ldx #0, lda $8020,x, sta $c000,x
    0xe8, 0xe0, 0x40, 0xd0, 0xf5, // inx, cpx #$40, bne
    0x4c, 0x00, 0xc0, // jmp $c000
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // pad to $8020
    0xa2, 0x00, // ldx #0
    0x8a, 0x29, 0x03, 0x8d, 0x00, 0xde, // txa, and #3, sta $de00
    0xad, 0x00, 0x90, 0x8d, 0x20, 0xd0, // lda $9000, sta $d020
    0x9d, 0x00, 0x04, // sta $0400,x
    0xe8, 0xe0, 0x80, 0xd0, 0xec, // inx, cpx #$80, bne
    0xa9, 0x80, 0x8d, 0x00, 0xde, // disable the cartridge
    0xad, 0x00, 0x90, 0x8d, 0x21, 0xd0, // lda $9000, sta $d021
    0xee, 0x20, 0xd0, 0x4c, 0x21, 0xc0, // inc $d020, jmp *-3
];

struct Sound {
    state: Mutex<(u64, u64)>,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            state: Mutex::new((FNV_OFFSET, 0)),
        }
    }
}

impl SoundOutput for Sound {
    fn reset(&self) {}

    fn write(&self, samples: &[i16]) {
        let mut state = self.state.lock().unwrap();
        let mut fnv = Fnv(state.0);
        for sample in samples {
            fnv.write(&sample.to_le_bytes());
        }
        *state = (fnv.0, state.1 + samples.len() as u64);
    }
}

struct Replay {
    c64: C64,
    sound: Arc<Sound>,
    next_key_event: u64,
}

impl Replay {
    fn new(config: Config) -> Self {
        let sound = Arc::new(Sound::default());
        let mut c64 = C64::new(config, sound.clone());
        c64.reset(true);
        Self {
            c64,
            sound,
            next_key_event: 0,
        }
    }

    fn run(&mut self, frames: u32) {
        for _ in 0..frames {
            if self.c64.get_keyboard().has_events() && self.c64.get_cycles() >= self.next_key_event
            {
                self.c64.get_keyboard().drain_event();
                self.next_key_event = self.c64.get_cycles().wrapping_add(KEY_EVENT_CYCLES);
            }
            self.c64.run_frame();
            self.c64.reset_vsync();
        }
    }

    fn fingerprint(&mut self) -> u64 {
        let mut fnv = Fnv::new();
        let visible = (0x0000..=0xcfff_u16)
            .chain(0xd800..=0xdbff)
            .chain(0xe000..=0xffff);
        for address in visible {
            fnv.write(&[self.c64.read(address)]);
        }
        let cpu = self.c64.get_cpu();
        fnv.write(self.c64.frame());
        let (sound_hash, sample_count) = *self.sound.state.lock().unwrap();
        fnv.write(&sound_hash.to_le_bytes());
        fnv.write(&sample_count.to_le_bytes());
        for reg in [
            Register::A,
            Register::X,
            Register::Y,
            Register::SP,
            Register::P,
        ] {
            fnv.write(&[cpu.get_register(reg)]);
        }
        fnv.write(&cpu.get_pc().to_le_bytes());
        fnv.write(&self.c64.get_cycles().to_le_bytes());
        fnv.write(&self.c64.get_frame_count().to_le_bytes());
        fnv.0
    }
}

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(FNV_OFFSET)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(FNV_PRIME);
        }
    }
}

struct NoiseTape {
    seed: u32,
}

impl NoiseTape {
    const SEED: u32 = 0x1234_5678;

    fn new() -> Self {
        Self { seed: Self::SEED }
    }
}

impl Tape for NoiseTape {
    fn read_pulse(&mut self) -> Option<u32> {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        Some([384, 528, 688][(self.seed >> 16) as usize % 3])
    }

    fn seek(&mut self, pos: usize) -> bool {
        if pos == 0 {
            self.seed = Self::SEED;
        }
        true
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
    assert_eq!(replay.fingerprint(), 0x92f2730e68b1d5a5);
}

#[test]
fn chip_tour_drives_every_chip() {
    let mut config = config();
    config.joystick.joystick_2 = joystick::Mode::Numpad;
    let mut replay = Replay::new(config);
    autostart(&mut replay.c64, CHIP_TOUR, 0x0801);
    replay.run(150);
    let joystick = replay.c64.get_joystick2_mut().as_mut().unwrap();
    joystick.on_key_down(Button::Up);
    joystick.on_key_down(Button::Fire);
    replay.run(150);
    assert_eq!(replay.fingerprint(), 0x2644f39ba9fe0b61);
}

#[test]
fn magic_desk_switches_banks_then_detaches() {
    let mut replay = Replay::new(config());
    let mut cartridge = Cartridge::new(0x0100, HwType::MagicDesk, false, true);
    for bank in 0..4_u8 {
        let mut data = vec![0; 0x2000];
        if bank == 0 {
            data[..MAGIC_DESK_BANK_0.len()].copy_from_slice(MAGIC_DESK_BANK_0);
        }
        data[0x1000] = 0x11 * (bank + 1);
        cartridge.add(Chip {
            chip_type: ChipType::Rom,
            bank_number: bank,
            offset: 0x8000,
            size: 0x2000,
            data,
        });
    }
    replay.c64.attach_cartridge(cartridge);
    replay.c64.reset(false);
    replay.run(100);
    replay.c64.detach_cartridge();
    replay.run(150);
    assert_eq!(replay.fingerprint(), 0xa29e4ff1e42a01dc);
}

#[test]
fn tape_load_reads_pulses() {
    let mut replay = Replay::new(config());
    replay.c64.attach_tape(Box::new(NoiseTape::new()));
    replay.run(120);
    replay.c64.get_keyboard().enqueue("LOAD\n");
    replay.run(20);
    replay.c64.play_tape();
    replay.run(300);
    assert_eq!(replay.fingerprint(), 0x45dd8975a42754c4);
}

#[test]
fn functional_test_runs_from_binary() {
    let mut replay = Replay::new(config());
    let data = read("bin/6502_functional_test.bin");
    replay.c64.mount(Media::Binary {
        data,
        offset: 0x0400,
    });
    replay.run(300);
    assert_eq!(replay.fingerprint(), 0x8f76a5f310d37fcb);
}
