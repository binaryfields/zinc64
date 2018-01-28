/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate getopts;
#[macro_use]
extern crate log;
extern crate sdl2;
extern crate time;
extern crate zinc64;

mod app;

use std::env;
use std::process;
use std::rc::Rc;

use zinc64::system::{C64, ChipFactory};

use self::app::{App, Cli, ConsoleApp, Logger};

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
        let factory = Box::new(ChipFactory::new(config.clone()));
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
