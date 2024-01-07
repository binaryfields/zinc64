// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(feature = "std")]
extern crate core;
#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;

pub mod autostart;
mod breakpoint;
pub mod c64;
mod c64_factory;
mod condition;
pub mod config;

pub use self::autostart::{Autostart, AutostartMethod, Image};
pub use self::breakpoint::Breakpoint;
pub use self::c64::C64;
pub use self::c64_factory::C64Factory;
pub use self::condition::Condition;
pub use self::config::Config;
