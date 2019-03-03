// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::result::Result;
use zinc64_core::{new_shared, SystemModel};
use zinc64_emu::system::{C64, C64Factory, Config};
use zinc64_loader::Loaders;
use zorio::cursor::Cursor;

// use crate::debug;
use crate::audio::AudioEngine;
use crate::device::board;
use crate::device::delay;
use crate::device::gpio::GPIO;
use crate::device::mbox::Mbox;
use crate::device::fat32::{self, Fat32};
use crate::memory;
use crate::palette::Palette;
use crate::video_buffer::VideoBuffer;
use crate::util::reader::ImageReader;
use crate::sound_buffer::SoundBuffer;
use crate::video_renderer::VideoRenderer;

// static RES_BASIC_ROM: &[u8] = include_bytes!("../../res/rom/basic.rom");
// static RES_CHARSET_ROM: &[u8] = include_bytes!("../../res/rom/characters.rom");
// static RES_KERNAL_ROM: &[u8] = include_bytes!("../../res/rom/kernal.rom");
// static RES_APP_IMAGE: &[u8] = include_bytes!("../../bin/SineAndGraphics.prg");

// TODO impl app state/audio state

#[allow(unused)]
pub struct App<'a> {
    // Components
    c64: C64,
    pub audio_engine: AudioEngine<'a>,
    video_renderer: VideoRenderer,
    // Resources
    mbox: Mbox<'a>,
    // Runtime State
    frame_duration: u64,
    idle_counter: u64,
    next_frame_ts: u64,
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
    pub fn build(gpio: &GPIO, fat32: &Fat32) -> Result<App<'a>, &'static str> {
        // Initialize emulator
        let config = Rc::new(Config::new_with_roms(
            SystemModel::from("pal"),
            read_res(fat32, "res/rom/basic.rom")?.as_ref(),
            read_res(fat32, "res/rom/charac~1.rom")?.as_ref(),
            read_res(fat32, "res/rom/kernal.rom")?.as_ref(),
        ));
        let sound_buffer = Arc::new(
            SoundBuffer::new(config.sound.buffer_size << 4)
        );
        let video_buffer = new_shared(VideoBuffer::new(
            config.model.frame_buffer_size.0,
            config.model.frame_buffer_size.1,
            Palette::default(),
        ));
        let chip_factory = Box::new(C64Factory::new(config.clone()));
        let mut c64 = C64::build(
            config.clone(),
            &*chip_factory,
            video_buffer.clone(),
            sound_buffer.clone());
        c64.reset(false);
        // Initialize audio
        let audio_engine = AudioEngine::build(
            gpio,
            config.sound.sample_rate,
            config.sound.buffer_size << 3,
            sound_buffer.clone(),
        )?;
        // Initialize video
        let mut mbox = Mbox::build(memory::map::MBOX_BASE)?;
        let video_renderer = VideoRenderer::build(
            &mut mbox,
            video_buffer.clone(),
            config.model.viewport_offset,
            config.model.viewport_size,
        )?;
        let frame_duration = delay::get_counter_freq() as u64
                * config.model.cycles_per_frame as u64 / config.model.cpu_freq as u64;
        Ok(App {
            c64,
            audio_engine,
            video_renderer,
            mbox,
            frame_duration,
            idle_counter: 0,
            next_frame_ts: 0,
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
            }
            Err(_) => debug!("autorun not found"),
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        info!("Running main loop");
        self.audio_engine.start();
        self.next_frame_ts = delay::get_counter() + self.frame_duration;
        loop {
            self.c64.run_frame();
            // if self.c64.is_cpu_jam() {
            //     info!("CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
            //     break;
            // }
            self.process_vsync()?;
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

    fn process_vsync(&mut self) -> Result<(), &'static str> {
        let refresh_rate = self.c64.get_config().model.refresh_rate as u32;
        self.idle_counter += self.next_frame_ts - delay::get_counter();
        self.sync_frame();
        self.video_renderer.render()?;
        self.c64.reset_vsync();
        if self.c64.get_frame_count() % refresh_rate == 0 {
            let idle_pct = self.idle_counter * 100 / delay::get_counter_freq() as u64;
            self.idle_counter = 0;
            info!("Frame {}, idle {}", self.c64.get_frame_count(), idle_pct);
        }
        Ok(())
    }

    fn sync_frame(&mut self) {
        delay::wait_counter(self.next_frame_ts);
        self.next_frame_ts += self.frame_duration;
    }

    #[allow(unused)]
    fn wait_for_vsync(&mut self) -> Result<(), &'static str> {
        board::wait_for_vsync(&mut self.mbox)
    }
}


