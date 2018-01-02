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

use std::result::Result;
use std::path::Path;

use getopts;
use zinc64::device;
use zinc64::loader::{BinLoader, Loader, Loaders};
use zinc64::system::{C64, Config, Model};

use super::app;

static NAME: &'static str = "zinc64";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct Cli;

impl Cli {

    pub fn parse_args(args: &Vec<String>) -> Result<getopts::Matches, String> {
        let opts = Cli::build_options();
        let matches = opts
            .parse(&args[1..])
            .map_err(|f| format!("Invalid options\n{}", f))?;
        Ok(matches)
    }

    pub fn parse_app_options(matches: &getopts::Matches) -> Result<app::Options, String> {
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
            speed: matches
                .opt_str("speed")
                .map(|s| s.parse::<u8>().unwrap())
                .unwrap_or(100),
            warp_mode: matches.opt_present("warp"),
        };
        Ok(options)
    }

    pub fn parse_system_config(matches: &getopts::Matches) -> Result<Config, String> {
        let model = Model::from(
            &matches
                .opt_str("model")
                .unwrap_or(String::from("pal"))
        );
        let mut config = Config::new(model);
        Cli::parse_device_config(&mut config, matches)?;
        Cli::parse_sound_config(&mut config, matches)?;
        Ok(config)
    }

    pub fn print_help() {
        let opts = Cli::build_options();
        println!("{} {}", NAME, VERSION);
        println!();
        println!("Usage:");
        print!("{}", opts.usage("C64 rustified emulator"));
    }

    pub fn print_version() {
        println!("{} {}", NAME, VERSION);
    }

    pub fn set_c64_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
        Cli::set_debug_options(c64, matches)?;
        Cli::set_autostart_options(c64, matches)?;
        Ok(())
    }

    fn build_options() -> getopts::Options {
        let mut opts = getopts::Options::new();
        opts.optopt("", "model", "set NTSC or PAL variants", "[ntsc|pal]")
            // Autostart
            .optopt("", "autostart", "attach and autostart image", "path")
            .optopt("", "binary", "load binary into memory", "path")
            .optopt("", "offset", "offset at which to load binary", "address")
            // App
            .optflag("", "console", "start in console mode")
            .optflag("f", "fullscreen", "enable fullscreen")
            .optopt("", "width", "window width", "800")
            .optopt("", "height", "window height", "600")
            .optopt("", "speed", "set speed of the emulator", "number")
            .optflag("", "warp", "enable wrap mode")
            // Device
            .optopt("", "joydev1", "set device for joystick 1", "none")
            .optopt("", "joydev2", "set device for joystick 2", "numpad")
            // Sound
            .optflag("", "nosound", "disable sound playback")
            .optflag("", "nosidfilters", "disable SID filters")
            .optopt("", "soundbufsize", "set sound buffer size in samples", "4096")
            .optopt("", "soundrate", "set sound sample rate in Hz", "44100")
            // Debug
            .optmulti("", "bp", "set breakpoint at this address", "address")
            .optopt("", "jamaction", "set cpu jam handling", "[continue|quit|reset]")
            // Logging
            .optopt("", "loglevel", "set log level", "[error|warn|info|debug|trace]")
            .optmulti("", "log", "set log level for a target", "target=level")
            // Help
            .optflag("h", "help", "display this help")
            .optflag("V", "version", "display this version");
        opts
    }

    fn parse_device_config(config: &mut Config, matches: &getopts::Matches) -> Result<(), String> {
        if let Some(joydev) = matches.opt_str("joydev1") {
            config.joystick.joystick_1 = device::joystick::Mode::from(&joydev);
        }
        if let Some(joydev) = matches.opt_str("joydev2") {
            config.joystick.joystick_2 = device::joystick::Mode::from(&joydev);
        }
        Ok(())
    }

    fn parse_sound_config(config: &mut Config, matches: &getopts::Matches) -> Result<(), String> {
        config.sound.enable = !matches.opt_present("nosound");
        config.sound.buffer_size = matches
            .opt_str("soundbufsize")
            .map(|s| s.parse::<usize>().unwrap())
            .unwrap_or(4096);
        config.sound.sample_rate = matches
            .opt_str("soundrate")
            .map(|s| s.parse::<u32>().unwrap())
            .unwrap_or(44100);
        config.sound.sid_filters = !matches.opt_present("nosidfilters");
        Ok(())
    }

    fn set_autostart_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
        match matches.opt_str("autostart") {
            Some(image_path) => {
                let path = Path::new(&image_path);
                let loader = Loaders::from_path(path);
                let mut autostart = loader
                    .autostart(path)
                    .map_err(|err| format!("{}", err))?;
                autostart.execute(c64);
            }
            None => {
                match matches.opt_str("binary") {
                    Some(binary_path) => {
                        let offset = matches
                            .opt_str("offset")
                            .map(|s| s.parse::<u16>().unwrap())
                            .unwrap_or(0);
                        let path = Path::new(&binary_path);
                        let loader = BinLoader::new(offset);
                        let mut image = loader
                            .load(path)
                            .map_err(|err| format!("{}", err))?;
                        image.mount(c64);
                    }
                    None => {}
                }
            },
        }
        Ok(())
    }

    fn set_debug_options(c64: &mut C64, matches: &getopts::Matches) -> Result<(), String> {
        let bps_strs = matches.opt_strs("bp");
        let bps = bps_strs
            .iter()
            .map(|s| s.parse::<u16>().unwrap());
        for bp in bps {
            c64.add_breakpoint(bp);
        }
        Ok(())
    }
}