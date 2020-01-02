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

pub use crate::bin::BinLoader;
pub use crate::io::{Reader, Result};

pub enum Format {
    Bin,
    Crt,
    P00,
    Prg,
    Tap,
}

impl Format {
    pub fn from_ext(ext: Option<&str>) -> Option<Format> {
        match ext {
            Some("bin") => Some(Format::Bin),
            Some("crt") => Some(Format::Crt),
            Some("p00") => Some(Format::P00),
            Some("P00") => Some(Format::P00),
            Some("prg") => Some(Format::Prg),
            Some("tap") => Some(Format::Tap),
            _ => None,
        }
    }
}

pub trait Loader {
    fn autostart(&self, path: &mut dyn Reader) -> io::Result<AutostartMethod>;
    fn load(&self, path: &mut dyn Reader) -> io::Result<Box<dyn Image>>;
}

pub struct Loaders;

impl Loaders {
    pub fn from(kind: Format) -> Box<dyn Loader> {
        match kind {
            Format::Bin => Box::new(bin::BinLoader::new(1024)),
            Format::Crt => Box::new(crt::CrtLoader::new()),
            Format::P00 => Box::new(p00::P00Loader::new()),
            Format::Prg => Box::new(prg::PrgLoader::new()),
            Format::Tap => Box::new(tap::TapLoader::new()),
        }
    }

    pub fn from_ext(ext: Option<&str>) -> Result<Box<dyn Loader>> {
        if let Some(kind) = Format::from_ext(ext) {
            Ok(Loaders::from(kind))
        } else {
            Err(format!("Unknown image extension {}", ext.unwrap_or("")))
        }
    }
}
