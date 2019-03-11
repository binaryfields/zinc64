// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[macro_use]
extern crate log;

mod app;
mod audio;
mod cli;
mod console;
mod execution;
mod input;
mod palette;
mod sound_buffer;
mod util;
mod video_buffer;
mod video_renderer;

use std::process;
use std::rc::Rc;
use std::sync::Arc;

use structopt::StructOpt;
use zinc64_core::new_shared;
use zinc64_emu::system::{C64Factory, C64};

use crate::app::App;
use crate::cli::Opt;
use crate::console::ConsoleApp;
use crate::palette::Palette;
use crate::sound_buffer::SoundBuffer;
use crate::util::Logger;
use crate::video_buffer::VideoBuffer;

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
    c64.reset(true);
    cli::set_c64_options(&mut c64, opt)?;
    if opt.console {
        let mut app = ConsoleApp::new(c64);
        app.run();
    } else {
        let options = cli::build_app_options(opt)?;
        let mut app = App::build(c64, video_buffer.clone(), sound_buffer.clone(), options)?;
        app.run()?;
    }
    Ok(())
}
