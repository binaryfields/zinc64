// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::path::Path;

use crate::crt;
use crate::p00;
use crate::prg;
use crate::tap;
use crate::Loader;

pub struct Loaders;

impl Loaders {
    pub fn from_ext(ext: Option<&str>) -> Box<dyn Loader> {
        match ext {
            Some("crt") => Box::new(crt::CrtLoader::new()),
            //Some("hex") => Box::new(hex::HexLoader::new()),
            Some("p00") => Box::new(p00::P00Loader::new()),
            Some("P00") => Box::new(p00::P00Loader::new()),
            Some("prg") => Box::new(prg::PrgLoader::new()),
            Some("tap") => Box::new(tap::TapLoader::new()),
            _ => panic!("invalid loader {}", ext.unwrap_or("")),
        }
    }

    pub fn from_path(path: &Path) -> Box<dyn Loader> {
        let ext = path.extension().map(|s| s.to_str().unwrap_or(""));
        Loaders::from_ext(ext)
    }
}
