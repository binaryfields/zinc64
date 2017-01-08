/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::path::Path;

use loader::Loader;
use loader::crt;
use loader::prg;

pub struct Loaders {}

impl Loaders {
    pub fn from_ext(ext: Option<&str>) -> Box<Loader> {
        match ext {
            Some("crt") => Box::new(crt::CrtLoader::new()),
            //Some("hex") => Box::new(hex::HexLoader::new()),
            Some("prg") => Box::new(prg::PrgLoader::new()),
            _ => panic!("invalid loader {}", ext.unwrap_or(""))
        }
    }

    pub fn from_path(path: &Path) -> Box<Loader> {
        let ext = path.extension()
            .map(|s| s.to_str().unwrap_or(""));
        Loaders::from_ext(ext)
    }
}

