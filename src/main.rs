/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern crate byteorder;
extern crate getopts;
extern crate sdl2;
extern crate time;

mod c64;
mod config;
mod cpu;
mod device;
mod io;
mod loader;
mod mem;
mod ui;
mod util;
mod video;

use std::env;
use std::ffi::OsStr;
use std::process;
use std::result::Result;

use c64::C64;
use config::Config;
use loader::{BinLoader, Loader, Loaders};
use mem::BaseAddr;
use std::path::Path;

static NAME: &'static str = "zinc64";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let result = run(env::args().collect());
    match result {
        Ok(rc) => process::exit(rc),
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1)
        },
    };
}

fn build_cli_options() -> getopts::Options {
    let mut opts = getopts::Options::new();
    opts.optopt("", "model", "set NTSC or PAL variants", "model")
        // Autostart
        .optopt("", "autostart", "attach and autostart image", "image")
        .optopt("", "binary", "load binary into memory", "path")
        .optopt("", "offset", "offset at which to load binary", "address")
        // Devices
        .optopt("", "joydev1", "set device for joystick 1", "numpad")
        .optopt("", "joydev2", "set device for joystick 2", "none")
        // Ui
        .optflag("", "console", "start in console mode")
        .optflag("f", "fullscreen", "enable fullscreen")
        .optopt("", "width", "window width", "width")
        .optopt("", "height", "window height", "height")
        // Debug
        .optmulti("", "bp", "set breakpoint at this address", "address")
        // Help
        .optflag("h", "help", "display this help")
        .optflag("V", "version", "display this version");
    opts
}

fn build_sys_config(matches: &getopts::Matches) -> Result<Config, String> {
    let model = matches.opt_str("model")
        .unwrap_or(String::from("pal"));
    let mut config = Config::new(&model);
    if let Some(joydev) = matches.opt_str("joydev1") {
        config.joystick1 = device::joystick::Mode::from(&joydev);
    }
    if let Some(joydev) = matches.opt_str("joydev2") {
        config.joystick2 = device::joystick::Mode::from(&joydev);
    }
    Ok(config)
}

fn build_ui_options(matches: &getopts::Matches) -> Result<ui::Options, String> {
    let options = ui::Options {
        fullscreen: matches.opt_present("fullscreen"),
        height: matches.opt_str("height")
            .map(|s| s.parse::<u32>().unwrap())
            .unwrap_or(600),
        width: matches.opt_str("width")
            .map(|s| s.parse::<u32>().unwrap())
            .unwrap_or(800),
    };
    Ok(options)
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
            let mut autostart = loader.autostart(path)
                .map_err(|err| format!("{}", err))?;
            autostart.execute(c64);
        },
        None => {
            match matches.opt_str("binary") {
                Some(binary_path) => {
                    let offset = matches.opt_str("offset")
                        .map(|s| s.parse::<u16>().unwrap())
                        .unwrap_or(0);
                    let path = Path::new(&binary_path);
                    let loader = BinLoader::new(offset);
                    let mut image = loader.load(path)
                        .map_err(|err| format!("{}", err))?;
                    image.mount(c64);
                },
                None => c64.reset(),
            }
        },
    }
    Ok(())
}

fn process_debug_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
    let bps_strs = matches.opt_strs("bp");
    let bps = bps_strs.iter()
        .map(|s| s.parse::<u16>().unwrap());
    for bp in bps {
        c64.add_breakpoint(bp);
    }
    Ok(())
}

fn run(args: Vec<String>) -> Result<i32, String> {
    let opts = build_cli_options();
    let matches = opts.parse(&args[1..]).map_err(|f| {
        format!("Invalid options\n{}", f)
    })?;
    if matches.opt_present("help") {
        print_help(&opts);
        Ok(0)
    } else if matches.opt_present("version") {
        print_version();
        Ok(0)
    } else {
        let config = build_sys_config(&matches)?;
        let mut c64 = C64::new(config).unwrap();
        process_debug_options(&mut c64, &matches)?;
        process_autostart_options(&mut c64, &matches)?;
        if matches.opt_present("console") {
            loop {
                let running = c64.run_frame();
                if !running {
                    break;
                }
            }
        } else {
            let options = build_ui_options(&matches)?;
            let mut app_window = ui::AppWindow::new(c64, options)?;
            app_window.run();
        }
        Ok(0)
    }
}
