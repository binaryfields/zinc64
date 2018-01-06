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

// Spec: http://unusedino.de/ec64/technical/misc/vic656x/colors/

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
