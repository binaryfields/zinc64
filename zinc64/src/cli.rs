// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use structopt::StructOpt;
use zinc64_core::SystemModel;
use zinc64_emu::device::joystick;
use zinc64_emu::system::{Config, C64};

use crate::app::{self, JamAction};

#[derive(StructOpt, Debug)]
#[structopt(name = "zinc64")]
pub struct Opt {
    /// attach and autostart image
    #[structopt(parse(from_os_str))]
    pub image: Option<PathBuf>,

    /// set NTSC or PAL variants
    #[structopt(long, default_value = "pal")]
    pub model: String,
    /// start in console mode
    #[structopt(long)]
    pub console: bool,
    /// set speed of the emulator
    #[structopt(long)]
    pub speed: Option<u8>,
    /// enable wrap mode
    #[structopt(long = "warp")]
    pub warp_mode: bool,
    /// set cpu jam handling
    #[structopt(
        long = "jamaction",
        default_value = "continue",
        parse(try_from_str = "parse_jam_action"),
        group = "debug"
    )]
    pub jam_action: JamAction,

    // -- Ui
    /// window width
    #[structopt(long, default_value = "800")]
    pub width: u32,
    /// window height
    #[structopt(long, default_value = "600")]
    pub height: u32,
    /// enable fullscreen
    #[structopt(short, long)]
    pub fullscreen: bool,

    // -- Devices
    /// set device for joystick 1
    #[structopt(
        long = "joydev1",
        default_value = "none",
        parse(try_from_str = "parse_joy_mode"),
        group = "devices"
    )]
    pub joydev_1: joystick::Mode,
    /// set device for joystick 2
    #[structopt(
        long = "joydev2",
        default_value = "numpad",
        parse(try_from_str = "parse_joy_mode"),
        group = "devices"
    )]
    pub joydev_2: joystick::Mode,

    // -- Roms
    /// filename of the basic ROM
    #[structopt(long, parse(from_os_str), group = "rom")]
    pub basic: Option<PathBuf>,
    /// filename of the character generator ROM
    #[structopt(long, parse(from_os_str), group = "rom")]
    pub charset: Option<PathBuf>,
    /// filename of the kernal ROM
    #[structopt(long, parse(from_os_str), group = "rom")]
    pub kernal: Option<PathBuf>,
    // -- Sound
    /// disable sound playback
    #[structopt(long = "nosound")]
    pub no_sound: bool,
    /// disable SID filters
    #[structopt(long = "nosidfilters")]
    pub no_sid_filters: bool,
    /// set sound sample rate in Hz
    #[structopt(long = "sound-rate", default_value = "44100")]
    pub sound_rate: u32,
    /// set sound buffer size in samples
    #[structopt(long = "sound-samples", default_value = "2048")]
    pub sound_samples: u32,

    // -- Debug
    /// set breakpoint at this address
    #[structopt(long)]
    pub bp: Vec<u16>,
    /// start debugger
    #[structopt(long)]
    pub debug: bool,
    /// start debugger bound to the specified address
    #[structopt(
        long = "dbg-address",
        default_value = "127.0.0.1:9999",
        parse(try_from_str = "parse_socket_addr"),
        group = "debug"
    )]
    pub dbg_address: SocketAddr,
    /// start rap server bound to the specified address
    #[structopt(
        long = "rap-address",
        default_value = "127.0.0.1:9999",
        parse(try_from_str = "parse_socket_addr"),
        group = "debug"
    )]
    pub rap_address: SocketAddr,

    // -- Logging
    /// set log level
    #[structopt(long = "loglevel", default_value = "info")]
    pub log_level: String,
    /// set log level for a target
    #[structopt(long = "log", parse(try_from_str = "parse_key_val"), group = "logging")]
    pub log_target_level: Vec<(String, String)>,
}

pub fn build_app_options(opt: &Opt) -> Result<app::Options, String> {
    Ok(app::Options {
        fullscreen: opt.fullscreen,
        window_size: (opt.width, opt.height),
        speed: opt.speed.unwrap_or(100),
        warp_mode: opt.warp_mode,
        debug: opt.debug,
        dbg_address: opt.dbg_address,
        rap_address: SocketAddr::from(([127, 0, 0, 1], 9999)), // opt.rap_address,
        jam_action: opt.jam_action,
    })
}

pub fn build_emu_config(opt: &Opt) -> Result<Config, String> {
    let model = SystemModel::from(opt.model.as_str());
    let mut config = Config::new(model);
    config.joystick.joystick_1 = opt.joydev_1;
    config.joystick.joystick_2 = opt.joydev_2;
    let basic_path = Path::new(
        opt.basic
            .as_ref()
            .map(|path| Path::new(path))
            .unwrap_or(Path::new("res/rom/basic.rom")),
    );
    let charset_path = Path::new(
        opt.charset
            .as_ref()
            .map(|path| Path::new(path))
            .unwrap_or(Path::new("res/rom/characters.rom")),
    );
    let kernal_path = Path::new(
        opt.kernal
            .as_ref()
            .map(|path| Path::new(path))
            .unwrap_or(Path::new("res/rom/kernal.rom")),
    );
    config.roms.basic = load_file(basic_path).map_err(|_| format!("Invalid rom: basic"))?;
    config.roms.charset = load_file(charset_path).map_err(|_| format!("Invalid rom: charset"))?;
    config.roms.kernal = load_file(kernal_path).map_err(|_| format!("Invalid rom: kernal"))?;
    config.sound.enable = !opt.no_sound;
    config.sound.buffer_size = opt.sound_samples as usize;
    config.sound.sample_rate = opt.sound_rate;
    config.sound.sid_filters = !opt.no_sid_filters;
    Ok(config)
}

pub fn set_c64_options(c64: &mut C64, opt: &Opt) -> Result<(), String> {
    set_c64_debug_options(c64, opt)?;
    Ok(())
}

fn set_c64_debug_options(c64: &mut C64, opt: &Opt) -> Result<(), String> {
    for bp in &opt.bp {
        c64.get_bpm_mut().set(*bp, false);
    }
    Ok(())
}

fn load_file(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut data = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut data)?;
    Ok(data)
}

fn parse_jam_action(s: &str) -> Result<JamAction, Box<Error>> {
    match s {
        "continue" => Ok(JamAction::Continue),
        "quit" => Ok(JamAction::Quit),
        "reset" => Ok(JamAction::Reset),
        _ => Err(Box::<Error>::from("invalid jamaction".to_string())),
    }
}

fn parse_joy_mode(mode: &str) -> Result<joystick::Mode, Box<Error>> {
    match mode {
        "none" => Ok(joystick::Mode::None),
        "numpad" => Ok(joystick::Mode::Numpad),
        "joy0" => Ok(joystick::Mode::Joy0),
        "joy1" => Ok(joystick::Mode::Joy1),
        _ => Err(Box::<Error>::from("invalid joystick".to_string())),
    }
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<Error>>
where
    T: std::str::FromStr,
    T::Err: Error + 'static,
    U: std::str::FromStr,
    U::Err: Error + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

fn parse_socket_addr(s: &str) -> Result<SocketAddr, Box<Error>> {
    s.parse::<SocketAddr>()
        .map_err(|_| Box::<Error>::from("invalid address".to_string()))
}
