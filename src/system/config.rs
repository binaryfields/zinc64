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

use core::Model;
use device::joystick;

pub struct Config {
    pub model: Model,
    pub joystick: JoystickConfig,
    pub sound: SoundConfig,
}

impl Config {
    pub fn new(model: Model) -> Config {
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
