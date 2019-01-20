// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::result::Result;
use zinc64_core::{new_shared, Shared, SystemModel};
use zinc64_emu::system::{C64, C64Factory, Config};

// use crate::debug;
use crate::geo::Rect;
use crate::hal::frame_buffer::FrameBuffer;
use crate::hal::mbox::Mbox;
use crate::null_output::NullSound;
use crate::palette::Palette;
use crate::video_buffer::VideoBuffer;

static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");

const FB_SIZE: (u32, u32) = (640, 480);
const FB_BPP: u32 = 32;

pub struct App {
    c64: C64,
    frame_buffer: FrameBuffer,
    #[allow(unused)]
    mbox: Mbox,
    video_buffer: Shared<VideoBuffer>,
    viewport_rect: Rect,
}

impl App {
    pub fn build(mut mbox: Mbox) -> Result<App, &'static str> {
        let frame_buffer = FrameBuffer::build_with_size(
            &mut mbox,
            FB_SIZE,
            FB_BPP,
        )?;
        info!("Allocated frame buffer at 0x{:08x}", frame_buffer.as_ptr() as usize);
        let config = Rc::new(Config::new_with_roms(
            SystemModel::from("pal"),
            RES_BASIC_ROM,
            RES_CHARSET_ROM,
            RES_KERNAL_ROM,
        ));
        let chip_factory = Box::new(C64Factory::new(config.clone()));
        let video_buffer = new_shared(VideoBuffer::new(
            config.model.frame_buffer_size.0,
            config.model.frame_buffer_size.1,
            Palette::default(),
        ));
        let sound_buffer = Arc::new(NullSound {});
        let mut c64 = C64::build(
            config.clone(),
            &*chip_factory,
            video_buffer.clone(),
            sound_buffer.clone());
        c64.reset(false);
        let viewport_rect = Rect::new_with_origin(
            config.model.viewport_offset,
            config.model.viewport_size,
        );
        Ok(App {
            c64,
            frame_buffer,
            mbox,
            video_buffer,
            viewport_rect,
        })
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        loop {
            self.c64.run_frame();
            if self.c64.is_cpu_jam() {
                info!("CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
                break;
            }
            if self.c64.get_frame_count() % (self.c64.get_config().model.refresh_rate as u32) == 0 {
                info!("Frame {}", self.c64.get_frame_count());
                // debug::dump_screen(&c64);
            }
            self.frame_buffer.wait_for_vsync(&mut self.mbox)?;
            self.frame_buffer.blit(
                self.video_buffer.borrow().get_data(),
                &self.viewport_rect,
                self.video_buffer.borrow().get_pitch() as u32,
            );
            self.c64.reset_vsync();
        }
        Ok(())
    }
}