// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate log;

pub mod cpu;
pub mod device;
pub mod factory;
pub mod io;
pub mod mem;
pub mod sound;
pub mod util;
pub mod video;
