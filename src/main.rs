/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

mod ui;

use std::env;
use std::path::Path;
use std::process;
use std::result::Result;

use zinc64::config::Config;
use zinc64::device;
use zinc64::loader::{BinLoader, Loader, Loaders};
use zinc64::system::C64;
use zinc64::util::Logger;

use ui::app;

static NAME: &'static str = "zinc64";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    match run(env::args().collect()) {
        Ok(_) => process::exit(0),
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1)
        }
    };
}

fn build_cli_options() -> getopts::Options {
    let mut opts = getopts::Options::new();
    opts.optopt("", "model", "set NTSC or PAL variants", "[ntsc|pal]")
        // Autostart
        .optopt("", "autostart", "attach and autostart image", "image")
        .optopt("", "binary", "load binary into memory", "path")
        .optopt("", "offset", "offset at which to load binary", "address")
        // Devices
        .optopt("", "joydev1", "set device for joystick 1", "numpad")
        .optopt("", "joydev2", "set device for joystick 2", "none")
        // Logging
        .optopt("", "loglevel", "set log level", "[error|warn|info|debug|trace]")
        .optmulti("", "log", "set log level for a target", "target=level")
        // Ui
        .optflag("", "console", "start in console mode")
        .optflag("f", "fullscreen", "enable fullscreen")
        .optopt("", "width", "window width", "width")
        .optopt("", "height", "window height", "height")
        // Debug
        .optmulti("", "bp", "set breakpoint at this address", "address")
        .optopt("", "jamaction", "set cpu jam handling", "[continue|quit|reset]")
        .optopt("", "speed", "set speed of the emulator", "number")
        // Help
        .optflag("h", "help", "display this help")
        .optflag("V", "version", "display this version");
    opts
}

fn build_app_options(matches: &getopts::Matches) -> Result<app::Options, String> {
    let options = app::Options {
        fullscreen: matches.opt_present("fullscreen"),
        jam_action: matches
            .opt_str("jamaction")
            .map(|s| app::JamAction::from(&s))
            .unwrap_or(app::JamAction::Continue),
        height: matches
            .opt_str("height")
            .map(|s| s.parse::<u32>().unwrap())
            .unwrap_or(600),
        width: matches
            .opt_str("width")
            .map(|s| s.parse::<u32>().unwrap())
            .unwrap_or(800),
    };
    Ok(options)
}

fn build_sys_config(matches: &getopts::Matches) -> Result<Config, String> {
    let model = matches.opt_str("model").unwrap_or(String::from("pal"));
    let mut config = Config::new(&model);
    if let Some(joydev) = matches.opt_str("joydev1") {
        config.joystick1 = device::joystick::Mode::from(&joydev);
    }
    if let Some(joydev) = matches.opt_str("joydev2") {
        config.joystick2 = device::joystick::Mode::from(&joydev);
    }
    Ok(config)
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

fn print_help(opts: &getopts::Options) {
    println!("{} {}", NAME, VERSION);
    println!("");
    println!("Usage:");
    print!("{}", opts.usage("C64 rustified emulator"));
}

fn print_version() {
    println!("{} {}", NAME, VERSION);
}

fn process_autostart_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
    match matches.opt_str("autostart") {
        Some(image_path) => {
            let path = Path::new(&image_path);
            let loader = Loaders::from_path(path);
            let mut autostart = loader.autostart(path).map_err(|err| format!("{}", err))?;
            autostart.execute(c64);
        }
        None => match matches.opt_str("binary") {
            Some(binary_path) => {
                let offset = matches
                    .opt_str("offset")
                    .map(|s| s.parse::<u16>().unwrap())
                    .unwrap_or(0);
                let path = Path::new(&binary_path);
                let loader = BinLoader::new(offset);
                let mut image = loader.load(path).map_err(|err| format!("{}", err))?;
                image.mount(c64);
            }
            None => {}
        },
    }
    Ok(())
}

fn process_debug_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
    let bps_strs = matches.opt_strs("bp");
    let bps = bps_strs.iter().map(|s| s.parse::<u16>().unwrap());
    for bp in bps {
        c64.add_breakpoint(bp);
    }
    let speed = matches
        .opt_str("speed")
        .map(|s| s.parse::<u8>().unwrap())
        .unwrap_or(100);
    c64.set_speed(speed);
    Ok(())
}

fn run(args: Vec<String>) -> Result<(), String> {
    let opts = build_cli_options();
    let matches = opts.parse(&args[1..])
        .map_err(|f| format!("Invalid options\n{}", f))?;
    if matches.opt_present("help") {
        print_help(&opts);
    } else if matches.opt_present("version") {
        print_version();
    } else {
        init_logging(&matches)?;
        info!("Staring {}", NAME);
        let config = build_sys_config(&matches)?;
        let mut c64 = C64::new(config).unwrap();
        c64.reset(true);
        process_debug_options(&mut c64, &matches)?;
        process_autostart_options(&mut c64, &matches)?;
        if matches.opt_present("console") {
            let mut overflow_cycles = 0i32;
            loop {
                overflow_cycles = c64.run_frame(overflow_cycles);
                if c64.is_cpu_jam() {
                    let cpu = c64.get_cpu();
                    warn!(target: "main", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
                    break;
                }
            }
        } else {
            let options = build_app_options(&matches)?;
            let mut app_window = ui::AppWindow::new(c64, options)?;
            app_window.run()?;
        }
    }
    Ok(())
}
