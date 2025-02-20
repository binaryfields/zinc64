// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::process;
use std::sync::{Arc, mpsc};
use std::thread;

use structopt::StructOpt;
use zinc64_debug::{Command, Executor, MonitorServer};
use zinc64_engine::C64;

use crate::app::{App, Options};
use crate::audio::SoundBuffer;
use crate::cli::Opt;
use crate::logger::Logger;
use crate::session::{JamAction, Session};

#[macro_use]
extern crate log;

mod app;
mod audio;
mod circular_buffer;
mod cli;
mod console;
mod framework;
mod headless;
mod input;
mod logger;
mod session;
mod ui;
mod video;

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
    let config = cli::build_emu_config(opt)?;
    let sound_buffer = Arc::new(SoundBuffer::new(config.sound.buffer_size << 2));
    let mut c64 = C64::new(config, sound_buffer.clone());
    cli::set_c64_options(&mut c64, opt)?;
    c64.reset(true);
    let app_options = cli::build_app_options(opt)?;
    // Nobody watches a console run, so a jam ends it unless asked otherwise.
    let jam_action = opt.jam_action.unwrap_or(if opt.console {
        JamAction::Quit
    } else {
        JamAction::Continue
    });
    let mut session = Session::new(c64, start_debugger(&app_options), jam_action);
    if let Some(image_path) = &opt.image {
        session.open_image(image_path)?;
    }
    if opt.console {
        headless::run(&mut session)?;
    } else {
        let fx_options = framework::Options {
            title: NAME.to_owned(),
            window_size: (opt.width, opt.height),
            fullscreen: opt.fullscreen,
        };
        framework::run(fx_options, |ctx| {
            App::build(ctx, session, sound_buffer, app_options)
        })?;
    }
    Ok(())
}

fn start_debugger(options: &Options) -> Executor {
    let (command_tx, command_rx) = mpsc::channel::<Command>();
    if options.debug {
        let address = options.dbg_address;
        info!("Starting debugger at {}", address);
        let command_tx = command_tx.clone();
        thread::spawn(move || {
            let debugger = MonitorServer::new(command_tx);
            debugger.start(address).expect("Failed to start debugger");
        });
    }
    Executor::new(command_rx)
}
