// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::result::Result;
use zinc64_core::{new_shared, Shared, SystemModel};
use zinc64_emu::system::{C64, C64Factory, Config};
use zinc64_loader::{LoaderKind, Loaders};
use zorio::cursor::Cursor;

// use crate::debug;
use crate::geo::Rect;
use crate::hal::frame_buffer::FrameBuffer;
use crate::hal::mbox::Mbox;
use crate::reader::ImageReader;
use crate::null_output::NullSound;
use crate::palette::Palette;
use crate::video_buffer::VideoBuffer;

static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");
static RES_APP_IMAGE: &[u8] = include_bytes!("../../bin/SineAndGraphics.prg");

const FB_SIZE: (u32, u32) = (640, 480);
const FB_BPP: u32 = 32;

pub struct App<'a> {
    // Dependencies
    mbox: &'a mut Mbox<'a>,
    // Components
    c64: C64,
    video_buffer: Shared<VideoBuffer>,
    viewport_rect: Rect,
    // Runtime State
    frame_buffer: FrameBuffer,
    next_keyboard_event: u64,
}

impl<'a> App<'a> {
    pub fn build(mbox: &'a mut Mbox<'a>) -> Result<App<'a>, &'static str> {
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
        let frame_buffer = FrameBuffer::build(
            mbox,
            FB_SIZE,
            (config.model.viewport_size.0, config.model.viewport_size.1),
            (0, 0),
            FB_BPP,
        )?;
        info!("Allocated frame buffer at 0x{:08x}", frame_buffer.as_ptr() as usize);
        Ok(App {
            mbox,
            c64,
            video_buffer,
            viewport_rect,
            frame_buffer,
            next_keyboard_event: 0,
        })
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        let mut image = ImageReader(Cursor::new(RES_APP_IMAGE));
        let loader = Loaders::from(LoaderKind::Prg);
        let mut autostart = loader.autostart(&mut image)
            .map_err(|_| "failed to load image")?;
        autostart.execute(&mut self.c64);
        info!("Running main loop");
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
            self.handle_events();
        }
        Ok(())
    }

    fn handle_events(&mut self) {
        let keyboard = self.c64.get_keyboard();
        if keyboard.borrow().has_events()
            && self.c64.get_cycles() >= self.next_keyboard_event
        {
            keyboard.borrow_mut().drain_event();
            self.next_keyboard_event = self.c64.get_cycles().wrapping_add(20000);
        }
    }
}