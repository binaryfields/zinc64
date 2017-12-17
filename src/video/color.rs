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

#[derive(Copy, Clone)]
pub enum Color {
    Black = 0x000000,
    White = 0xffffff,
    Red = 0x68372b,
    Cyan = 0x70a4b2,
    Purple = 0x6f3d86,
    Green = 0x588d43,
    Blue = 0x352879,
    Yellow = 0xb8c76f,
    Orange = 0x6f4f25,
    Brown = 0x433900,
    LightRed = 0x9a6759,
    DarkGray = 0x444444,
    MediumGray = 0x6c6c6c,
    LightGreen = 0x9ad284,
    LightBlue = 0x6c5eb5,
    LightGray = 0x959595,
}

impl Color {
    pub fn from(color: u8) -> Color {
        match color {
            0 => Color::Black,
            1 => Color::White,
            2 => Color::Red,
            3 => Color::Cyan,
            4 => Color::Purple,
            5 => Color::Green,
            6 => Color::Blue,
            7 => Color::Yellow,
            8 => Color::Orange,
            9 => Color::Brown,
            10 => Color::LightRed,
            11 => Color::DarkGray,
            12 => Color::MediumGray,
            13 => Color::LightGreen,
            14 => Color::LightBlue,
            15 => Color::LightGray,
            _ => panic!("invalid color {}", color),
        }
    }

    pub fn rgb(&self) -> u32 {
        *self as u32
    }
}
