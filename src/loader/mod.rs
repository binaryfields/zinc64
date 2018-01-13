/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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

mod bin;
mod crt;
//mod hex;
mod loaders;
mod prg;
mod tap;

use std::io;
use std::path::Path;

use system::{AutostartMethod, Image};

pub use self::bin::BinLoader;
pub use self::loaders::Loaders;

pub trait Loader {
    fn autostart(&self, path: &Path) -> Result<AutostartMethod, io::Error>;
    fn load(&self, path: &Path) -> Result<Box<Image>, io::Error>;
}
