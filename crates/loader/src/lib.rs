// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::{self, Read};

use zinc64_engine::{AutostartMethod, Media};

pub use crate::bin::BinLoader;

#[macro_use]
extern crate log;

mod bin;
mod crt;
mod p00;
mod prg;
mod tap;

pub trait Loader {
    fn autostart(&self, reader: &mut dyn Read) -> io::Result<AutostartMethod>;
    fn load(&self, reader: &mut dyn Read) -> io::Result<Media>;
}

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

pub struct Loaders;

impl Loaders {
    pub fn from_ext(ext: Option<&str>) -> io::Result<Box<dyn Loader>> {
        if let Some(kind) = Format::from_ext(ext) {
            Ok(Loaders::from(kind))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("Unknown image extension {}", ext.unwrap_or("")),
            ))
        }
    }

    pub fn from(kind: Format) -> Box<dyn Loader> {
        match kind {
            Format::Bin => Box::new(bin::BinLoader::new(1024)),
            Format::Crt => Box::new(crt::CrtLoader::new()),
            Format::P00 => Box::new(p00::P00Loader::new()),
            Format::Prg => Box::new(prg::PrgLoader::new()),
            Format::Tap => Box::new(tap::TapLoader::new()),
        }
    }
}

fn invalid_data<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}
