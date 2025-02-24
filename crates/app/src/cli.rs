// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use structopt::StructOpt;
use zinc64_engine::Config;
use zinc64_engine::SystemModel;

use crate::input::JoystickDevice;
use crate::session::JamAction;
use crate::settings::Settings;

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
    /// set speed of the emulator in percent of real time
    #[structopt(long)]
    pub speed: Option<u8>,
    /// enable wrap mode
    #[structopt(long = "warp")]
    pub warp_mode: bool,
    /// set cpu jam handling: continue, quit or reset (default: quit in
    /// console mode, continue otherwise)
    #[structopt(
        long = "jamaction",
        parse(try_from_str = parse_jam_action)
    )]
    pub jam_action: Option<JamAction>,

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
    /// set device for joystick 1: none or numpad
    #[structopt(
        long = "joydev1",
        default_value = "none",
        parse(try_from_str = parse_joystick_device)
    )]
    pub joydev_1: JoystickDevice,
    /// set device for joystick 2: none or numpad
    #[structopt(
        long = "joydev2",
        default_value = "numpad",
        parse(try_from_str = parse_joystick_device)
    )]
    pub joydev_2: JoystickDevice,

    // -- Roms
    /// filename of the basic ROM
    #[structopt(long, parse(from_os_str))]
    pub basic: Option<PathBuf>,
    /// filename of the character generator ROM
    #[structopt(long, parse(from_os_str))]
    pub charset: Option<PathBuf>,
    /// filename of the kernal ROM
    #[structopt(long, parse(from_os_str))]
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
        parse(try_from_str = parse_socket_addr)
    )]
    pub dbg_address: SocketAddr,
    /// start a radare2 server bound to the specified address
    #[structopt(long = "rap-address", parse(try_from_str = parse_socket_addr))]
    pub rap_address: Option<SocketAddr>,

    // -- Logging
    /// set log level
    #[structopt(long = "loglevel", default_value = "info")]
    pub log_level: String,
    /// set log level for a target
    #[structopt(long = "log", parse(try_from_str = parse_key_val))]
    pub log_target_level: Vec<(String, String)>,
}

// ---- argument parsing ----

fn parse_jam_action(s: &str) -> Result<JamAction, Box<dyn Error>> {
    match s {
        "continue" => Ok(JamAction::Continue),
        "quit" => Ok(JamAction::Quit),
        "reset" => Ok(JamAction::Reset),
        _ => Err(Box::<dyn Error>::from("invalid jamaction".to_string())),
    }
}

fn parse_joystick_device(device: &str) -> Result<JoystickDevice, Box<dyn Error>> {
    match device {
        "none" => Ok(JoystickDevice::None),
        "numpad" => Ok(JoystickDevice::Numpad),
        _ => Err(Box::<dyn Error>::from(
            "invalid joystick device, expected none or numpad".to_string(),
        )),
    }
}

fn parse_socket_addr(s: &str) -> Result<SocketAddr, Box<dyn Error>> {
    s.parse::<SocketAddr>()
        .map_err(|_| Box::<dyn Error>::from("invalid address".to_string()))
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error>>
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

// ---- emulator config ----

pub fn build_emu_config(opt: &Opt) -> Result<Config, String> {
    let model = SystemModel::from(opt.model.as_str());
    let mut config = Config::new(model);
    let basic_path = Path::new(
        opt.basic
            .as_ref()
            .map(Path::new)
            .unwrap_or(Path::new("res/rom/basic.rom")),
    );
    let charset_path = Path::new(
        opt.charset
            .as_ref()
            .map(Path::new)
            .unwrap_or(Path::new("res/rom/characters.rom")),
    );
    let kernal_path = Path::new(
        opt.kernal
            .as_ref()
            .map(Path::new)
            .unwrap_or(Path::new("res/rom/kernal.rom")),
    );
    config.roms.basic = load_file(basic_path).map_err(|_| "Invalid rom: basic".to_string())?;
    config.roms.charset =
        load_file(charset_path).map_err(|_| "Invalid rom: charset".to_string())?;
    config.roms.kernal = load_file(kernal_path).map_err(|_| "Invalid rom: kernal".to_string())?;
    config.sound.sample_rate = opt.sound_rate;
    config.sound.sid_filters = !opt.no_sid_filters;
    Ok(config)
}

fn load_file(path: &Path) -> Result<Vec<u8>, io::Error> {
    let mut data = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut data)?;
    Ok(data)
}

// ---- host settings ----

pub fn build_settings(opt: &Opt) -> Settings {
    Settings {
        window_size: (opt.width, opt.height),
        fullscreen: opt.fullscreen,
        speed: opt.speed.unwrap_or(100),
        warp_mode: opt.warp_mode,
        audio: !opt.no_sound,
        audio_buffer: opt.sound_samples as usize,
        joystick_1: opt.joydev_1,
        joystick_2: opt.joydev_2,
        breakpoints: opt.bp.clone(),
        monitor_address: opt.debug.then_some(opt.dbg_address),
        rap_address: opt.rap_address,
    }
}
