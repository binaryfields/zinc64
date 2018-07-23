// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::SystemModel;
use device::joystick;

pub struct Config {
    pub model: SystemModel,
    pub joystick: JoystickConfig,
    pub sound: SoundConfig,
}

impl Config {
    pub fn new(model: SystemModel) -> Config {
        Config {
            model,
            joystick: JoystickConfig::default(),
            sound: SoundConfig::default(),
        }
    }
}

pub struct JoystickConfig {
    pub axis_motion_threshold: i16,
    pub joystick_1: joystick::Mode,
    pub joystick_2: joystick::Mode,
}

impl JoystickConfig {
    pub fn default() -> JoystickConfig {
        JoystickConfig {
            axis_motion_threshold: 3200,
            joystick_1: joystick::Mode::Numpad,
            joystick_2: joystick::Mode::None,
        }
    }
}

pub struct SoundConfig {
    pub enable: bool,
    pub buffer_size: usize,
    pub sample_rate: u32,
    pub sid_filters: bool,
}

impl SoundConfig {
    pub fn default() -> SoundConfig {
        SoundConfig {
            enable: true,
            buffer_size: 4096,
            sample_rate: 44100,
            sid_filters: true,
        }
    }
}
