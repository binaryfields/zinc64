// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[macro_use]
extern crate log;

mod app;
mod audio;
mod cli;
mod cmd;
mod console;
mod debug;
mod framework;
mod gfx;
mod input;
mod palette;
mod platform;
mod time;
mod ui;
mod util;
mod video;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process;
use std::rc::Rc;
use std::sync::Arc;

use structopt::StructOpt;
use zinc64_core::util::new_shared;
use zinc64_loader::Loaders;
use zinc64_system::{C64Factory, C64};

use crate::app::App;
use crate::audio::SoundBuffer;
use crate::cli::Opt;
use crate::palette::Palette;
use crate::util::{FileReader, Logger};
use crate::video::VideoBuffer;

static NAME: &str = "zinc64";

fn main() {
    let opt = Opt::from_args();
    match run(&opt) {
        Ok(_) => process::exit(0),
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1)
        }
    };
}

fn load_image(c64: &mut C64, path: &Path) -> Result<(), String> {
    let ext = path.extension().map(|s| s.to_str().unwrap());
    let loader = Loaders::from_ext(ext)?;
    let file = File::open(path).map_err(|err| format!("{}", err))?;
    let mut reader = FileReader(BufReader::new(file));
    let mut autostart = loader.autostart(&mut reader)?;
    autostart.execute(c64);
    Ok(())
}

fn run(opt: &Opt) -> Result<(), String> {
    let logger = Logger::build(opt.log_level.as_str(), &opt.log_target_level)?;
    Logger::enable(logger)?;
    info!("Starting {}", NAME);
    let config = Rc::new(cli::build_emu_config(opt)?);
    let sound_buffer = Arc::new(SoundBuffer::new(config.sound.buffer_size << 2));
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
        sound_buffer.clone(),
    );
    cli::set_c64_options(&mut c64, opt)?;
    c64.reset(true);
    if let Some(image_path) = &opt.image {
        load_image(&mut c64, Path::new(image_path))?;
    }
    if opt.console {
        run_console(&mut c64);
    } else {
        let app_options = cli::build_app_options(opt)?;
        let fx_options = framework::Options {
            title: NAME.to_owned(),
            window_size: (opt.width, opt.height),
            fullscreen: opt.fullscreen,
        };
        framework::run(fx_options, |ctx| {
            App::build(
                ctx,
                c64,
                sound_buffer.clone(),
                video_buffer.clone(),
                app_options,
            )
        })?;
    }
    Ok(())
}

fn run_console(c64: &mut C64) {
    loop {
        c64.run_frame();
        c64.reset_vsync();
        if c64.is_cpu_jam() {
            warn!(target: "main", "CPU JAM detected at 0x{:x}", c64.get_cpu().get_pc());
            break;
        }
    }
}
