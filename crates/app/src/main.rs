// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::process;
use std::sync::{Arc, mpsc};
use std::thread;

use structopt::StructOpt;
use zinc64_debug::{Command, Executor, MonitorServer, RapServer};
use zinc64_engine::C64;

use crate::app::App;
use crate::audio::SoundBuffer;
use crate::cli::Opt;
use crate::logger::Logger;
use crate::session::{JamAction, Session};
use crate::settings::Settings;

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
mod settings;
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
    let settings = cli::build_settings(opt);
    let config = cli::build_emu_config(opt)?;
    let mut c64 = C64::new(config);
    c64.reset(true);
    // Nobody watches a console run, so a jam ends it unless asked otherwise.
    let jam_action = opt.jam_action.unwrap_or(if opt.console {
        JamAction::Quit
    } else {
        JamAction::Continue
    });
    let mut session = Session::new(c64, start_debuggers(&settings), jam_action);
    if let Some(image_path) = &opt.image {
        session.open_image(image_path)?;
    }
    if opt.console {
        headless::run(&mut session)?;
    } else {
        let sound_buffer = Arc::new(SoundBuffer::new(settings.audio_buffer << 2));
        let fx_options = framework::Options {
            title: NAME.to_owned(),
            window_size: settings.window_size,
            fullscreen: settings.fullscreen,
        };
        framework::run(fx_options, |ctx| {
            App::build(ctx, session, sound_buffer, settings)
        })?;
    }
    Ok(())
}

fn start_debuggers(settings: &Settings) -> Executor {
    let (command_tx, command_rx) = mpsc::channel::<Command>();
    if let Some(address) = settings.monitor_address {
        info!("Starting debugger at {}", address);
        let command_tx = command_tx.clone();
        thread::spawn(move || {
            let debugger = MonitorServer::new(command_tx);
            debugger.start(address).expect("Failed to start debugger");
        });
    }
    if let Some(address) = settings.rap_address {
        info!("Starting radare2 server at {}", address);
        let command_tx = command_tx.clone();
        thread::spawn(move || {
            let server = RapServer::new(command_tx);
            server
                .start(address)
                .expect("Failed to start radare2 server");
        });
    }
    let mut executor = Executor::new(command_rx);
    for address in &settings.breakpoints {
        executor.set_breakpoint(*address);
    }
    executor
}
