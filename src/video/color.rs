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
            _ => panic!("invalid color {}", color)
        }
    }

    pub fn rgb(&self) -> u32 {
        *self as u32
    }
}
