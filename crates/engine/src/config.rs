// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::model::SystemModel;

pub struct RomData {
    pub basic: Vec<u8>,
    pub charset: Vec<u8>,
    pub kernal: Vec<u8>,
}

impl RomData {
    pub fn new(basic: &[u8], charset: &[u8], kernal: &[u8]) -> Self {
        RomData {
            basic: basic.to_vec(),
            charset: charset.to_vec(),
            kernal: kernal.to_vec(),
        }
    }
}

impl Default for RomData {
    fn default() -> Self {
        RomData {
            basic: vec![0x00; 0x2000],
            charset: vec![0x00; 0x1000],
            kernal: vec![0x00; 0x2000],
        }
    }
}

pub struct SoundConfig {
    pub sample_rate: u32,
    pub sid_filters: bool,
}

impl Default for SoundConfig {
    fn default() -> SoundConfig {
        SoundConfig {
            sample_rate: 44100,
            sid_filters: true,
        }
    }
}

pub struct Config {
    pub model: SystemModel,
    pub sound: SoundConfig,
    pub roms: RomData,
}

impl Config {
    pub fn new(model: SystemModel) -> Config {
        Config {
            model,
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
            sound: SoundConfig::default(),
            roms: RomData::new(basic, charset, kernal),
        }
    }
}
