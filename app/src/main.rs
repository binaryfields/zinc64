// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

extern crate bit_field;
extern crate byteorder;
extern crate getopts;
#[macro_use]
extern crate log;
extern crate sdl2;
extern crate time;
extern crate zinc64;

mod app;
mod audio;
mod charset;
mod cli;
mod command;
mod console;
mod debugger;
mod disassembler;
mod execution;
mod io;
mod keymap;
mod logger;
mod rap_server;
mod renderer;

use std::env;
use std::process;
use std::rc::Rc;

use zinc64::system::{C64, C64Factory};

use self::app::App;
use self::cli::Cli;
use self::console::ConsoleApp;
use self::logger::Logger;

static NAME: &'static str = "zinc64";

fn main() {
    match run(env::args().collect()) {
        Ok(_) => process::exit(0),
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1)
        }
    };
}

fn init_logging(matches: &getopts::Matches) -> Result<(), String> {
    let loglevel = matches.opt_str("loglevel").unwrap_or("info".to_string());
    let mut logger = Logger::new(&loglevel)?;
    for target_level in matches.opt_strs("log") {
        if let Some(equals) = target_level.find('=') {
            let (target, level) = target_level.split_at(equals);
            logger.add_target(target.to_string(), level[1..].to_string())?;
        } else {
            return Err(format!("invalid log target pair {}", target_level));
        }
    }
    Logger::enable(logger)?;
    Ok(())
}

fn run(args: Vec<String>) -> Result<(), String> {
    let matches = Cli::parse_args(&args)?;
    if matches.opt_present("help") {
        Cli::print_help();
    } else if matches.opt_present("version") {
        Cli::print_version();
    } else {
        init_logging(&matches)?;
        info!("Starting {}", NAME);
        let config = Rc::new(Cli::parse_system_config(&matches)?);
        let factory = Box::new(C64Factory::new(config.clone()));
        let mut c64 = C64::new(config.clone(), factory).unwrap();
        c64.reset(true);
        Cli::set_c64_options(&mut c64, &matches)?;
        if matches.opt_present("console") {
            let mut app = ConsoleApp::new(c64);
            app.run();
        } else {
            let options = Cli::parse_app_options(&matches)?;
            let mut app = App::new(c64, options)?;
            app.run()?;
        }
    }
    Ok(())
}
