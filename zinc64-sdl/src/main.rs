// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use getopts;
#[macro_use]
extern crate log;

mod config;
mod console;
mod input;
mod output;
mod ui;
mod util;

use std::env;
use std::process;
use std::rc::Rc;
use std::sync::Arc;

use zinc64::system::{C64Factory, C64};
use zinc64_core::new_shared;

use crate::config::Cli;
use crate::console::ConsoleApp;
use crate::output::{FrameBuffer, Palette, SoundBuffer};
use crate::ui::App;
use crate::util::Logger;

static NAME: &str = "zinc64";

fn main() {
    let args: Vec<String> = env::args().collect();
    match run(&args) {
        Ok(_) => process::exit(0),
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1)
        }
    };
}

fn init_logging(matches: &getopts::Matches) -> Result<(), String> {
    let loglevel = matches
        .opt_str("loglevel")
        .unwrap_or_else(|| "info".to_string());
    let mut logger = Logger::build(&loglevel)?;
    for target_level in matches.opt_strs("log") {
        if let Some(equals) = target_level.find('=') {
            let (target, level) = target_level.split_at(equals);
            logger.add_target(target.to_string(), &level[1..])?;
        } else {
            return Err(format!("invalid log target pair {}", target_level));
        }
    }
    Logger::enable(logger)?;
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let matches = Cli::parse_args(&args)?;
    if matches.opt_present("help") {
        Cli::print_help();
    } else if matches.opt_present("version") {
        Cli::print_version();
    } else {
        init_logging(&matches)?;
        info!("Starting {}", NAME);
        let config = Rc::new(Cli::parse_system_config(&matches)?);
        let chip_factory = Box::new(C64Factory::new(config.clone()));
        let frame_buffer = new_shared(FrameBuffer::new(
            config.model.frame_buffer_size.0,
            config.model.frame_buffer_size.1,
            Palette::default(),
        ));
        let sound_buffer = Arc::new(SoundBuffer::new(config.sound.buffer_size << 2));
        let mut c64 = C64::build(
            config.clone(),
            &*chip_factory,
            frame_buffer.clone(),
            sound_buffer.clone(),
        );
        c64.reset(true);
        Cli::set_c64_options(&mut c64, &matches)?;
        if matches.opt_present("console") {
            let mut app = ConsoleApp::new(c64);
            app.run();
        } else {
            let options = Cli::parse_app_options(&matches)?;
            let mut app = App::build(c64, frame_buffer.clone(), sound_buffer.clone(), options)?;
            app.run()?;
        }
    }
    Ok(())
}
