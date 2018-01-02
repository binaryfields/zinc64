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

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use bit_field::BitField;

// SPEC: https://www.c64-wiki.com/index.php/Keyboard#Hardware

#[derive(Copy, Clone, Debug)]
pub enum Key {
    // Numerical
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    // Alpha
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    //
    Asterisk,
    At,
    Backspace,
    Caret,
    Colon,
    Comma,
    Dollar,
    Equals,
    Minus,
    Period,
    Plus,
    Return,
    Semicolon,
    Slash,
    Space,
    //
    CrsrDown,
    Ctrl,
    Home,
    Left,
    LGui,
    LShift,
    Pause,
    CrsrRight,
    RShift,
    // Function
    F1,
    F3,
    F5,
    F7,
}

#[derive(Copy, Clone, Debug)]
pub struct KeyEvent {
    keycode: Key,
    modifier: Option<Key>,
    disable_shift: bool,
}

impl KeyEvent {
    pub fn new(keycode: Key) -> KeyEvent {
        KeyEvent {
            keycode,
            modifier: None,
            disable_shift: false,
        }
    }

    pub fn with_disabled_shift(keycode: Key) -> KeyEvent {
        KeyEvent {
            keycode,
            modifier: None,
            disable_shift: true,
        }
    }

    pub fn with_mod(keycode: Key, modifier: Key) -> KeyEvent {
        KeyEvent {
            keycode,
            modifier: Some(modifier),
            disable_shift: false,
        }
    }
}

pub struct Keyboard {
    matrix: Rc<RefCell<[u8; 8]>>,
    queue: VecDeque<(KeyEvent, bool)>,
    disabled_shift: u8,
}

impl Keyboard {
    pub fn new(matrix: Rc<RefCell<[u8; 8]>>) -> Keyboard {
        Keyboard {
            matrix,
            queue: VecDeque::new(),
            disabled_shift: 0,
        }
    }

    pub fn get_row(&self, row: u8) -> u8 {
        self.matrix.borrow()[row as usize]
    }

    pub fn set_row(&mut self, row: u8, value: u8) {
        self.matrix.borrow_mut()[row as usize] = value;
    }

    pub fn drain_event(&mut self) {
        if let Some((key_event, pressed)) = self.queue.pop_front() {
            match pressed {
                true => self.on_key_down(key_event),
                false => self.on_key_up(key_event),
            }
        }
    }

    pub fn enqueue(&mut self, str: &str) {
        for c in str.to_string().chars() {
            let key_event = self.map_char(c);
            self.queue.push_back((key_event, true));
            self.queue.push_back((key_event, false));
        }
    }

    pub fn has_events(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn reset(&mut self) {
        for i in 0..8 {
            self.set_row(i, 0xff);
        }
        self.queue.clear();
    }

    fn is_pressed(&self, keycode: Key) -> bool {
        let mapping = self.map_keycode(keycode);
        !self.matrix.borrow()[mapping.0].get_bit(mapping.1)
    }

    fn set_key(&mut self, keycode: Key, enabled: bool) {
        let mapping = self.map_keycode(keycode);
        self.matrix.borrow_mut()[mapping.0].set_bit(mapping.1, !enabled);
    }

    // -- Event Handlers

    pub fn on_key_down(&mut self, event: KeyEvent) {
        self.set_key(event.keycode, true);
        if let Some(modifier) = event.modifier {
            self.set_key(modifier, true);
        }
        if event.disable_shift {
            if self.is_pressed(Key::LShift) {
                self.set_key(Key::LShift, false);
                self.disabled_shift.set_bit(0, true);
            }
            if self.is_pressed(Key::RShift) {
                self.set_key(Key::RShift, false);
                self.disabled_shift.set_bit(1, true);
            }
        }
    }

    pub fn on_key_up(&mut self, event: KeyEvent) {
        self.set_key(event.keycode, false);
        if let Some(modifier) = event.modifier {
            self.set_key(modifier, false);
        }
        if event.disable_shift {
            if self.disabled_shift.get_bit(0) {
                self.set_key(Key::LShift, true);
            }
            if self.disabled_shift.get_bit(1) {
                self.set_key(Key::RShift, true);
            }
            self.disabled_shift = 0;
        }
    }

    // -- Mapping Ops

    fn map_char(&self, c: char) -> KeyEvent {
        match c {
            '\n' => KeyEvent::new(Key::Return),
            ' ' => KeyEvent::new(Key::Space),
            '!' => KeyEvent::with_mod(Key::Num1, Key::LShift),
            '"' => KeyEvent::with_mod(Key::Num2, Key::LShift),
            '#' => KeyEvent::with_mod(Key::Num3, Key::LShift),
            '$' => KeyEvent::with_mod(Key::Num4, Key::LShift),
            '%' => KeyEvent::with_mod(Key::Num5, Key::LShift),
            '&' => KeyEvent::with_mod(Key::Num6, Key::LShift),
            '\'' => KeyEvent::with_mod(Key::Num7, Key::LShift),
            '(' => KeyEvent::with_mod(Key::Num8, Key::LShift),
            ')' => KeyEvent::with_mod(Key::Num9, Key::LShift),
            '*' => KeyEvent::new(Key::Asterisk),
            '+' => KeyEvent::new(Key::Plus),
            ',' => KeyEvent::new(Key::Comma),
            '-' => KeyEvent::new(Key::Minus),
            '.' => KeyEvent::new(Key::Period),
            '/' => KeyEvent::new(Key::Slash),
            '0' => KeyEvent::new(Key::Num0),
            '1' => KeyEvent::new(Key::Num1),
            '2' => KeyEvent::new(Key::Num2),
            '3' => KeyEvent::new(Key::Num3),
            '4' => KeyEvent::new(Key::Num4),
            '5' => KeyEvent::new(Key::Num5),
            '6' => KeyEvent::new(Key::Num6),
            '7' => KeyEvent::new(Key::Num7),
            '8' => KeyEvent::new(Key::Num8),
            '9' => KeyEvent::new(Key::Num9),
            ':' => KeyEvent::new(Key::Colon),
            ';' => KeyEvent::new(Key::Semicolon),
            '<' => KeyEvent::with_mod(Key::Comma, Key::LShift),
            '=' => KeyEvent::new(Key::Equals),
            '>' => KeyEvent::with_mod(Key::Period, Key::LShift),
            '?' => KeyEvent::with_mod(Key::Slash, Key::LShift),
            '@' => KeyEvent::new(Key::At),
            'A' => KeyEvent::new(Key::A),
            'B' => KeyEvent::new(Key::B),
            'C' => KeyEvent::new(Key::C),
            'D' => KeyEvent::new(Key::D),
            'E' => KeyEvent::new(Key::E),
            'F' => KeyEvent::new(Key::F),
            'G' => KeyEvent::new(Key::G),
            'H' => KeyEvent::new(Key::H),
            'I' => KeyEvent::new(Key::I),
            'J' => KeyEvent::new(Key::J),
            'K' => KeyEvent::new(Key::K),
            'L' => KeyEvent::new(Key::L),
            'M' => KeyEvent::new(Key::M),
            'N' => KeyEvent::new(Key::N),
            'O' => KeyEvent::new(Key::O),
            'P' => KeyEvent::new(Key::P),
            'Q' => KeyEvent::new(Key::Q),
            'R' => KeyEvent::new(Key::R),
            'S' => KeyEvent::new(Key::S),
            'T' => KeyEvent::new(Key::T),
            'U' => KeyEvent::new(Key::U),
            'V' => KeyEvent::new(Key::V),
            'W' => KeyEvent::new(Key::W),
            'X' => KeyEvent::new(Key::X),
            'Y' => KeyEvent::new(Key::Y),
            'Z' => KeyEvent::new(Key::Z),
            '^' => KeyEvent::new(Key::Caret),
            _ => panic!("unsupported char {}", c),
        }
    }

    fn map_keycode(&self, keycode: Key) -> (usize, usize) {
        match keycode {
            // Row 0
            Key::Backspace => (0, 0),
            Key::Return => (0, 1),
            Key::CrsrRight => (0, 2),
            Key::F7 => (0, 3),
            Key::F1 => (0, 4),
            Key::F3 => (0, 5),
            Key::F5 => (0, 6),
            Key::CrsrDown => (0, 7),
            // Row 1
            Key::Num3 => (1, 0),
            Key::W => (1, 1),
            Key::A => (1, 2),
            Key::Num4 => (1, 3),
            Key::Z => (1, 4),
            Key::S => (1, 5),
            Key::E => (1, 6),
            Key::LShift => (1, 7),
            // Row 2
            Key::Num5 => (2, 0),
            Key::R => (2, 1),
            Key::D => (2, 2),
            Key::Num6 => (2, 3),
            Key::C => (2, 4),
            Key::F => (2, 5),
            Key::T => (2, 6),
            Key::X => (2, 7),
            // Row 3
            Key::Num7 => (3, 0),
            Key::Y => (3, 1),
            Key::G => (3, 2),
            Key::Num8 => (3, 3),
            Key::B => (3, 4),
            Key::H => (3, 5),
            Key::U => (3, 6),
            Key::V => (3, 7),
            // Row 4
            Key::Num9 => (4, 0),
            Key::I => (4, 1),
            Key::J => (4, 2),
            Key::Num0 => (4, 3),
            Key::M => (4, 4),
            Key::K => (4, 5),
            Key::O => (4, 6),
            Key::N => (4, 7),
            // Row 5
            Key::Plus => (5, 0),
            Key::P => (5, 1),
            Key::L => (5, 2),
            Key::Minus => (5, 3),
            Key::Period => (5, 4),
            Key::Colon => (5, 5),
            Key::At => (5, 6),
            Key::Comma => (5, 7),
            // Row 6
            Key::Dollar => (6, 0),
            Key::Asterisk => (6, 1),
            Key::Semicolon => (6, 2),
            Key::Home => (6, 3),
            Key::RShift => (6, 4),
            Key::Equals => (6, 5),
            Key::Caret => (6, 6), // Caret
            Key::Slash => (6, 7),
            // Row 7
            Key::Num1 => (7, 0),
            Key::Left => (7, 1),
            Key::Ctrl => (7, 2),
            Key::Num2 => (7, 3),
            Key::Space => (7, 4),
            Key::LGui => (7, 5),
            Key::Q => (7, 6),
            Key::Pause => (7, 7),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_key_event() {
        let matrix = Rc::new(RefCell::new([0; 8]));
        let mut keyboard = Keyboard::new(matrix);
        keyboard.reset();
        assert_eq!(false, keyboard.has_events());
        keyboard.enqueue("S");
        assert_eq!(true, keyboard.has_events());
    }

    #[test]
    fn drain_key_event() {
        let matrix = Rc::new(RefCell::new([0; 8]));
        let mut keyboard = Keyboard::new(matrix);
        keyboard.reset();
        keyboard.enqueue("S");
        assert_eq!(true, keyboard.has_events());
        keyboard.drain_event();
        keyboard.drain_event();
        assert_eq!(false, keyboard.has_events());
    }

    #[test]
    fn emulate_key_press() {
        let matrix = Rc::new(RefCell::new([0; 8]));
        let mut keyboard = Keyboard::new(matrix);
        keyboard.reset();
        keyboard.enqueue("S");
        keyboard.drain_event();
        assert_eq!(0xdf, keyboard.get_row(1));
    }
}