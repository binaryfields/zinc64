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

use sdl2::keyboard::Keycode;

// SPEC: https://www.c64-wiki.com/index.php/Keyboard#Hardware

// TODO keyboard: add test cases

pub struct Keyboard {
    matrix: [u8; 8],
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            matrix: [0xff; 8]
        }
    }

    pub fn get_row(&self, row: u8) -> u8 {
        self.matrix[row as usize]
    }

    pub fn set_row(&mut self, row: u8, value: u8) {
        self.matrix[row as usize] = value;
    }

    pub fn on_key_down(&mut self, keycode: Keycode) {
        let mapping = self.map_keycode(keycode);
        if mapping.0 != 0xff {
            self.matrix[mapping.0 as usize] = self.bit_clear(self.matrix[mapping.0 as usize], mapping.1);
        }
    }

    pub fn on_key_up(&mut self, keycode: Keycode) {
        let mapping = self.map_keycode(keycode);
        if mapping.0 != 0xff {
            self.matrix[mapping.0 as usize] = self.bit_set(self.matrix[mapping.0 as usize], mapping.1);
        }
    }

    fn bit_clear(&self, value: u8, bit: u8) -> u8 {
        let mask = 1 << bit;
        value & !mask
    }

    fn bit_set(&self, value: u8, bit: u8) -> u8 {
        let mask = 1 << bit;
        value | mask
    }

    fn map_keycode(&self, keycode: Keycode) -> (u8, u8) {
        match keycode {
            // Row 0
            Keycode::Delete => (0, 0),
            Keycode::Return => (0, 1),
            // Keycode::F7 => (0, 2),
            Keycode::F7 => (0, 3),
            Keycode::F1 => (0, 4),
            Keycode::F3 => (0, 5),
            Keycode::F5 => (0, 6),
            // Keycode::F7 => (0, 7),
            // Row 1
            Keycode::Num3 => (1, 0),
            Keycode::W => (1, 1),
            Keycode::A => (1, 2),
            Keycode::Num4 => (1, 3),
            Keycode::Z => (1, 4),
            Keycode::S => (1, 5),
            Keycode::E => (1, 6),
            Keycode::LShift => (1, 7),
            // Row 2
            Keycode::Num5 => (2, 0),
            Keycode::R => (2, 1),
            Keycode::D => (2, 2),
            Keycode::Num6 => (2, 3),
            Keycode::C => (2, 4),
            Keycode::F => (2, 5),
            Keycode::T => (2, 6),
            Keycode::X => (2, 7),
            // Row 3
            Keycode::Num7 => (3, 0),
            Keycode::Y => (3, 1),
            Keycode::G => (3, 2),
            Keycode::Num8 => (3, 3),
            Keycode::B => (3, 4),
            Keycode::H => (3, 5),
            Keycode::U => (3, 6),
            Keycode::V => (3, 7),
            // Row 4
            Keycode::Num9 => (4, 0),
            Keycode::I => (4, 1),
            Keycode::J => (4, 2),
            Keycode::Num0 => (4, 3),
            Keycode::M => (4, 4),
            Keycode::K => (4, 5),
            Keycode::O => (4, 6),
            Keycode::N => (4, 7),
            // Row 5
            Keycode::Plus => (5, 0),
            Keycode::P => (5, 1),
            Keycode::L => (5, 2),
            Keycode::Minus => (5, 3),
            Keycode::Period => (5, 4),
            Keycode::Colon => (5, 5),
            Keycode::At => (5, 6),
            Keycode::Comma => (5, 7),
            // Row 6
            Keycode::Dollar => (6, 0),
            Keycode::Asterisk => (6, 1),
            Keycode::Semicolon => (6, 2),
            Keycode::Home => (6, 3),
            Keycode::RShift => (6, 4),
            Keycode::Equals => (6, 5),
            Keycode::Up => (6, 6),
            Keycode::Slash => (6, 7),
            // Row 7
            Keycode::Num1 => (7, 0),
            Keycode::Left => (7, 1),
            Keycode::LCtrl => (7, 2),
            Keycode::Num2 => (7, 3),
            Keycode::Space => (7, 4),
            Keycode::LGui => (7, 5),
            Keycode::Q => (7, 6),
            Keycode::Pause => (7, 7),
            _ => (0xff, 0xff),
        }
    }
}
