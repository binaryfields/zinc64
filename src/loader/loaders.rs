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

use std::path::Path;

use loader::Loader;
use loader::crt;
use loader::prg;
use loader::tap;

pub struct Loaders;

impl Loaders {
    pub fn from_ext(ext: Option<&str>) -> Box<Loader> {
        match ext {
            Some("crt") => Box::new(crt::CrtLoader::new()),
            //Some("hex") => Box::new(hex::HexLoader::new()),
            Some("prg") => Box::new(prg::PrgLoader::new()),
            Some("tap") => Box::new(tap::TapLoader::new()),
            _ => panic!("invalid loader {}", ext.unwrap_or("")),
        }
    }

    pub fn from_path(path: &Path) -> Box<Loader> {
        let ext = path.extension().map(|s| s.to_str().unwrap_or(""));
        Loaders::from_ext(ext)
    }
}
