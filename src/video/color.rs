/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Spec: C64 Programmer's Reference Guide p.455

#[derive(Copy, Clone)]
 pub enum Color {
    Black = 0x000000,
    White = 0xffffff,
    Red = 0x880000,
    Cyan = 0xaaffee,
    Purple = 0xcc44cc,
    Green = 0x00cc55,
    Blue = 0x0000aa,
    Yellow = 0xeeee77,
    Orange = 0xdd8855,
    Brown = 0x664400,
    LightRed = 0xff7777,
    DarkGray = 0x333333,
    MediumGray = 0x777777,
    LightGreen = 0xaaff66,
    LightBlue = 0x0088ff,
    LightGray = 0xbbbbbb,
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
            _ => panic!("invalid color {}", color)
        }
    }

    pub fn rgb(&self) -> u32 {
        *self as u32
    }
}
