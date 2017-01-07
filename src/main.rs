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
use loader::Loaders;
use mem::BaseAddr;
use std::path::Path;
use ui::{AppWindow, Options};

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

fn build_options() -> getopts::Options {
    let mut opts = getopts::Options::new();
    opts.optopt("p", "program", "program to load", "path")
        .optflag("f", "fullscreen", "fullscreen window")
        .optmulti("", "bp", "set breakpoint at this address", "address")
        .optflag("", "console", "start in console mode")
        .optopt("", "model", "specify NTSC or PAL variants", "model")
        .optopt("", "offset", "offset at which to load binary", "address")
        .optopt("", "width", "window width", "width")
        .optopt("", "height", "window height", "height")
        .optflag("h", "help", "display this help")
        .optflag("V", "version", "display this version");
    opts
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

fn run(args: Vec<String>) -> Result<i32, String> {
    let opts = build_options();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => return Err(format!("Invalid options\n{}", f)),
    };
    if matches.opt_present("help") {
        print_help(&opts);
        Ok(0)
    } else if matches.opt_present("version") {
        print_version();
        Ok(0)
    } else {
        let model = matches.opt_str("model")
            .unwrap_or(String::from("pal"));
        let config = Config::new(&model);
        let mut c64 = C64::new(config).unwrap();
        let bps_strs = matches.opt_strs("bp");
        let bps = bps_strs.iter()
            .map(|s| s.parse::<u16>().unwrap());
        for bp in bps {
            c64.add_breakpoint(bp);
        }
        let offset = matches.opt_str("offset")
            .map(|s| s.parse::<u16>().unwrap())
            .unwrap_or(0);
        match matches.opt_str("program") {
            Some(program) => {
                let path = Path::new(&program);
                let ext = path.extension()
                    .map(|s| s.to_str().unwrap_or(""));
                let loader = Loaders::new(ext);
                loader.load(&mut c64, path, offset)
                    .map_err(|err| format!("{}", err))?;
                // FIXME let cpu = c64.get_cpu();
                //cpu.borrow_mut().set_pc(offset);
                //cpu.borrow_mut().write(BaseAddr::IoPort.addr(), 0);
                c64.reset();
            },
            None => {
                c64.reset();
            },
        }
        if matches.opt_present("console") {
            loop {
                let running = c64.run_frame();
                if !running {
                    break;
                }
            }
        } else {
            let options = Options {
                fullscreen: matches.opt_present("fullscreen"),
                width: matches.opt_str("width")
                    .map(|s| s.parse::<u32>().unwrap())
                    .unwrap_or(800),
                height: matches.opt_str("height")
                    .map(|s| s.parse::<u32>().unwrap())
                    .unwrap_or(600),
            };
            let mut app_window = AppWindow::new(c64, options)?;
            app_window.run();
        }
        Ok(0)
    }
}
