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

use std::collections::VecDeque;

use sdl2::keyboard::Keycode;
use util::bit;

// SPEC: https://www.c64-wiki.com/index.php/Keyboard#Hardware

pub struct Keyboard {
    matrix: [u8; 8],
    buffer: VecDeque<(Keycode, bool)>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            matrix: [0; 8],
            buffer: VecDeque::new(),
        }
    }

    pub fn get_row(&self, row: u8) -> u8 {
        self.matrix[row as usize]
    }

    #[allow(dead_code)]
    pub fn set_row(&mut self, row: u8, value: u8) {
        self.matrix[row as usize] = value;
    }

    pub fn drain_event(&mut self) {
        if let Some((keycode, pressed)) = self.buffer.pop_front() {
            match pressed {
                true => self.on_key_down(keycode),
                false => self.on_key_up(keycode),
            }
        }
    }

    pub fn enqueue(&mut self, str: &str) {
        for c in str.to_string().chars() {
            let keycode = self.map_char(c);
            self.buffer.push_back((keycode, true));
            self.buffer.push_back((keycode, false));
        }
    }

    pub fn has_events(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn reset(&mut self) {
        self.matrix = [0xff; 8];
        self.buffer.clear();
    }

    // -- Event Handlers

    pub fn on_key_down(&mut self, keycode: Keycode) {
        let mapping = self.map_keycode(keycode);
        if mapping.0 != 0xff {
            self.matrix[mapping.0 as usize] = bit::bit_update(self.matrix[mapping.0 as usize],
                                                              mapping.1,
                                                              false);
        }
    }

    pub fn on_key_up(&mut self, keycode: Keycode) {
        let mapping = self.map_keycode(keycode);
        if mapping.0 != 0xff {
            self.matrix[mapping.0 as usize] = bit::bit_update(self.matrix[mapping.0 as usize],
                                                              mapping.1,
                                                              true);
        }
    }

    // -- Internal Ops

    fn map_char(&self, c: char) -> Keycode {
        match c {
            '\n' => Keycode::Return,
            ' ' => Keycode::Space,
            '$' => Keycode::Dollar,
            '*' => Keycode::Asterisk,
            '+' => Keycode::Plus,
            ',' => Keycode::Comma,
            '-' => Keycode::Minus,
            '.' => Keycode::Period,
            '/' => Keycode::Slash,
            '0' => Keycode::Num0,
            '1' => Keycode::Num1,
            '2' => Keycode::Num2,
            '3' => Keycode::Num3,
            '4' => Keycode::Num4,
            '5' => Keycode::Num5,
            '6' => Keycode::Num6,
            '7' => Keycode::Num7,
            '8' => Keycode::Num8,
            '9' => Keycode::Num9,
            ':' => Keycode::Colon,
            ';' => Keycode::Semicolon,
            '=' => Keycode::Equals,
            '@' => Keycode::At,
            'A' => Keycode::A,
            'B' => Keycode::B,
            'C' => Keycode::C,
            'D' => Keycode::D,
            'E' => Keycode::E,
            'F' => Keycode::F,
            'G' => Keycode::G,
            'H' => Keycode::H,
            'I' => Keycode::I,
            'J' => Keycode::J,
            'K' => Keycode::K,
            'L' => Keycode::L,
            'M' => Keycode::M,
            'N' => Keycode::N,
            'O' => Keycode::O,
            'P' => Keycode::P,
            'Q' => Keycode::Q,
            'R' => Keycode::R,
            'S' => Keycode::S,
            'T' => Keycode::T,
            'U' => Keycode::U,
            'V' => Keycode::V,
            'W' => Keycode::W,
            'X' => Keycode::X,
            'Y' => Keycode::Y,
            'Z' => Keycode::Z,
            _ => panic!("unsupported char {}", c),
        }
    }

    fn map_keycode(&self, keycode: Keycode) -> (u8, u8) {
        match keycode {
            // Row 0
            Keycode::Backspace => (0, 0),
            Keycode::Return => (0, 1),
            Keycode::Right => (0, 2),
            Keycode::F7 => (0, 3),
            Keycode::F1 => (0, 4),
            Keycode::F3 => (0, 5),
            Keycode::F5 => (0, 6),
            Keycode::Down => (0, 7),
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
            Keycode::Backslash => (5, 0),   // Plus
            Keycode::P => (5, 1),
            Keycode::L => (5, 2),
            Keycode::Minus => (5, 3),
            Keycode::Period => (5, 4),
            Keycode::Quote => (5, 5),       // Colon
            Keycode::LeftBracket => (5, 6), // At
            Keycode::Comma => (5, 7),
            // Row 6
            Keycode::Dollar => (6, 0),
            Keycode::RightBracket => (6, 1), // Asterisk
            Keycode::Semicolon => (6, 2),
            Keycode::Home => (6, 3),
            Keycode::RShift => (6, 4),
            Keycode::Equals => (6, 5),
            Keycode::Caret => (6, 6), // FIXME
            Keycode::Slash => (6, 7),
            // Row 7
            Keycode::Num1 => (7, 0),
            Keycode::Left => (7, 1),
            Keycode::LCtrl => (7, 2),
            Keycode::RCtrl => (7, 2),
            Keycode::Num2 => (7, 3),
            Keycode::Space => (7, 4),
            Keycode::LGui => (7, 5),
            Keycode::Q => (7, 6),
            Keycode::Pause => (7, 7),
            _ => (0xff, 0xff),
        }
    }
}
