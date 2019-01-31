// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate log;

mod bin;
mod crt;
mod io;
mod p00;
mod prg;
mod tap;

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use zinc64_emu::system::{AutostartMethod, Image};

pub use crate::io::{Reader, Result};
pub use crate::bin::BinLoader;

pub trait Loader {
    fn autostart(&self, path: &mut dyn Reader) -> io::Result<AutostartMethod>;
    fn load(&self, path: &mut dyn Reader) -> io::Result<Box<dyn Image>>;
}

pub enum LoaderKind {
    Crt,
    P00,
    Prg,
    Tap,
}

impl LoaderKind {
    pub fn from_ext(ext: Option<&str>) -> Option<LoaderKind> {
        match ext {
            Some("crt") => Some(LoaderKind::Crt),
            Some("p00") => Some(LoaderKind::P00),
            Some("P00") => Some(LoaderKind::P00),
            Some("prg") => Some(LoaderKind::Prg),
            Some("tap") => Some(LoaderKind::Tap),
            _ => None,
        }
    }
}

pub struct Loaders;

impl Loaders {
    pub fn from(kind: LoaderKind) -> Box<Loader> {
        match kind {
            LoaderKind::Crt => Box::new(crt::CrtLoader::new()),
            LoaderKind::P00 => Box::new(p00::P00Loader::new()),
            LoaderKind::Prg => Box::new(prg::PrgLoader::new()),
            LoaderKind::Tap => Box::new(tap::TapLoader::new()),
        }
    }
}
