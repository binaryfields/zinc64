// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// Spec: http://unusedino.de/ec64/technical/misc/vic656x/colors/

#![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]

use crate::gfx::Color;

pub struct Palette;

impl Palette {
    pub fn default() -> [u32; 16] {
        [
            Color::from_rgb(0x00, 0x00, 0x00).rgba(), // Black
            Color::from_rgb(0xff, 0xff, 0xff).rgba(), // White
            Color::from_rgb(0x68, 0x37, 0x2b).rgba(), // Red
            Color::from_rgb(0x70, 0xa4, 0xb2).rgba(), // Cyan
            Color::from_rgb(0x6f, 0x3d, 0x86).rgba(), // Purple
            Color::from_rgb(0x58, 0x8d, 0x43).rgba(), // Green
            Color::from_rgb(0x35, 0x28, 0x79).rgba(), // Blue
            Color::from_rgb(0xb8, 0xc7, 0x6f).rgba(), // Yellow
            Color::from_rgb(0x6f, 0x4f, 0x25).rgba(), // Orange
            Color::from_rgb(0x43, 0x39, 0x00).rgba(), // Brown
            Color::from_rgb(0x9a, 0x67, 0x59).rgba(), // LightRed
            Color::from_rgb(0x44, 0x44, 0x44).rgba(), // DarkGray
            Color::from_rgb(0x6c, 0x6c, 0x6c).rgba(), // MediumGray
            Color::from_rgb(0x9a, 0xd2, 0x84).rgba(), // LightGreen
            Color::from_rgb(0x6c, 0x5e, 0xb5).rgba(), // LightBlue
            Color::from_rgb(0x95, 0x95, 0x95).rgba(), // LightGray
        ]
    }
}
