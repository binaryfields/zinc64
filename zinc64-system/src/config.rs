// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
#[cfg(not(feature = "std"))]
use alloc::vec;
use zinc64_core::device::joystick;
use zinc64_core::factory::SystemModel;

pub struct Config {
    pub model: SystemModel,
    pub joystick: JoystickConfig,
    pub sound: SoundConfig,
    pub roms: RomData,
}

impl Config {
    pub fn new(model: SystemModel) -> Config {
        Config {
            model,
            joystick: JoystickConfig::default(),
            sound: SoundConfig::default(),
            roms: RomData::default(),
        }
    }

    pub fn new_with_roms(
        model: SystemModel,
        basic: &[u8],
        charset: &[u8],
        kernal: &[u8],
    ) -> Config {
        Config {
            model,
            joystick: JoystickConfig::default(),
            sound: SoundConfig::default(),
            roms: RomData::new(basic, charset, kernal),
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

pub struct RomData {
    pub basic: Vec<u8>,
    pub charset: Vec<u8>,
    pub kernal: Vec<u8>,
}

impl RomData {
    pub fn default() -> Self {
        RomData {
            basic: vec![0x00; 0x2000],
            charset: vec![0x00; 0x1000],
            kernal: vec![0x00; 0x2000],
        }
    }

    pub fn new(basic: &[u8], charset: &[u8], kernal: &[u8]) -> Self {
        RomData {
            basic: basic.to_vec(),
            charset: charset.to_vec(),
            kernal: kernal.to_vec(),
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
