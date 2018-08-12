// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

mod cli;

use std::net::SocketAddr;

pub use self::cli::Cli;

pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

impl JamAction {
    pub fn from(action: &str) -> JamAction {
        match action {
            "continue" => JamAction::Continue,
            "quit" => JamAction::Quit,
            "reset" => JamAction::Reset,
            _ => panic!("invalid jam action {}", action),
        }
    }
}

pub struct Options {
    pub fullscreen: bool,
    pub window_size: (u32, u32),
    pub speed: u8,
    pub warp_mode: bool,
    // Debug
    pub debug: bool,
    pub dbg_address: Option<SocketAddr>,
    pub jam_action: JamAction,
    pub rap_address: Option<SocketAddr>,
}
