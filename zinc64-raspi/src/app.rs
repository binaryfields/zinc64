// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use zinc64_core::{new_shared, SoundOutput, SystemModel, VideoOutput};
use zinc64_emu::system::{C64, C64Factory, Config};

static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");

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

pub fn run() {
    let config = Rc::new(Config::new_with_roms(
        SystemModel::from("pal"),
        RES_BASIC_ROM,
        RES_CHARSET_ROM,
        RES_KERNAL_ROM,
    ));
    let factory = Box::new(C64Factory::new(config.clone()));
    let video_output = new_shared(NullVideo {});
    let sound_output = Arc::new(NullSound {});
    let mut c64 = C64::build(config.clone(), &*factory, video_output, sound_output);
    c64.reset(false);
    loop {
        c64.run_frame();
        c64.reset_vsync();
        if c64.is_cpu_jam() {
            info!(target: "app", "CPU JAM detected at 0x{:x}", c64.get_cpu().get_pc());
            break;
        }
        if c64.get_frame_count() % 100 == 0 {
            info!(target: "app", "Frame {}", c64.get_frame_count());
        }
    }
}
