// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::net::SocketAddr;

use crate::input::JoystickDevice;

pub struct Settings {
    // Window
    pub window_size: (u32, u32),
    pub fullscreen: bool,
    // Speed
    pub speed: u8,
    pub warp_mode: bool,
    // Audio
    pub audio: bool,
    pub audio_buffer: usize,
    // Input
    pub joystick_1: JoystickDevice,
    pub joystick_2: JoystickDevice,
    // Debug
    pub breakpoints: Vec<u16>,
    pub monitor_address: Option<SocketAddr>,
    pub rap_address: Option<SocketAddr>,
}

impl Settings {
    pub fn frame_rate(&self, refresh_rate: f32) -> Option<f64> {
        if self.warp_mode {
            None
        } else {
            Some(refresh_rate as f64 * self.speed as f64 / 100.0)
        }
    }
}
