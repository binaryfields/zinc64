// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::result::Result;
use zinc64_core::{new_shared, Shared, SystemModel};
use zinc64_emu::system::{C64, C64Factory, Config};
use zinc64_loader::Loaders;
use zorio::cursor::Cursor;

// use crate::debug;
use crate::device::frame_buffer::FrameBuffer;
use crate::device::mbox::Mbox;
use crate::device::fat32::{self, Fat32};
use crate::null_output::NullSound;
use crate::palette::Palette;
use crate::video_buffer::VideoBuffer;
use crate::util::geo::Rect;
use crate::util::reader::ImageReader;

// static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
// static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
// static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");
// static RES_APP_IMAGE: &[u8] = include_bytes!("../../bin/SineAndGraphics.prg");

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

fn read_to_end(fat32: &Fat32, file: &mut fat32::File, buf: &mut Vec<u8>) -> Result<usize, &'static str> {
    let mut buffer = [0u8; 512];
    let mut total = 0;
    loop {
        let read = fat32.read(file, &mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in buffer[0..read].iter() {
            buf.push(*byte);
        }
        total += read;
    }
    Ok(total)
}

fn read_res(fat32: &Fat32, path: &str) -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    let mut file = fat32.open(path)?;
    let read = read_to_end(&fat32, &mut file, &mut data)?;
    info!("Read resource {} {} bytes", path, read);
    Ok(data)
}

impl<'a> App<'a> {
    pub fn build(mbox: &'a mut Mbox<'a>, fat32: &Fat32) -> Result<App<'a>, &'static str> {
        let config = Rc::new(Config::new_with_roms(
            SystemModel::from("pal"),
            read_res(fat32, "res/rom/basic.rom")?.as_ref(),
            read_res(fat32, "res/rom/charac~1.rom")?.as_ref(),
            read_res(fat32, "res/rom/kernal.rom")?.as_ref(),
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

    pub fn autostart(&mut self, fat32: &Fat32) -> Result<(), &'static str> {
        // let mut image = ImageReader(Cursor::new(RES_APP_IMAGE));
        // let loader = Loaders::from(LoaderKind::Prg);
        // let mut autostart = loader.autostart(&mut image)
        //     .map_err(|_| "failed to load image")?;
        // autostart.execute(&mut self.c64);
        info!("Searching for autostart image ...");
        match fat32.read_dir("res/autorun") {
            Ok(entries) => {
                for entry in entries.iter() {
                    debug!("Checking {}", entry.name);
                    if let Some(loader) = Loaders::from_ext(Some(entry.ext.to_lowercase().as_ref())) {
                        let mut path = String::new();
                        path.push_str("res/autorun/");
                        path.push_str(entry.name.as_ref());
                        info!("Preparing to autostart {}", path);
                        let image_res = read_res(fat32, &path)?;
                        let mut image = ImageReader(
                            Cursor::new(image_res.as_ref())
                        );
                        let mut autostart = loader.autostart(&mut image)
                            .map_err(|_| "failed to load image")?;
                        autostart.execute(&mut self.c64);
                        break;
                    }
                }
            },
            Err(_) => debug!("autorun not found"),
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
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