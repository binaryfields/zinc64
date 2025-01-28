// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// Spec: http://unusedino.de/ec64/technical/misc/vic656x/colors/

#![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]

pub struct Palette;

impl Palette {
    pub fn default() -> [u32; 16] {
        [
            0x000000, // Black
            0xffffff, // White
            0x68372b, // Red
            0x70a4b2, // Cyan
            0x6f3d86, // Purple
            0x588d43, // Green
            0x352879, // Blue
            0xb8c76f, // Yellow
            0x6f4f25, // Orange
            0x433900, // Brown
            0x9a6759, // LightRed
            0x444444, // DarkGray
            0x6c6c6c, // MediumGray
            0x9ad284, // LightGreen
            0x6c5eb5, // LightBlue
            0x959595, // LightGray
        ]
    }
}
