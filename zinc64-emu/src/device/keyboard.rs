// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::prelude::*;
use bit_field::BitField;
use zinc64_core::Shared;

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
    // Symbols
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
    // Control
    CrsrDown,
    Ctrl,
    Home,
    Left,
    LGui,
    LShift,
    RunStop,
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
    pub fn new(keycode: Key) -> Self {
        Self {
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
    matrix: Shared<[u8; 16]>,
    queue: Vec<(KeyEvent, bool)>,
    disabled_shift: u8,
}

impl Keyboard {
    pub fn new(matrix: Shared<[u8; 16]>) -> Self {
        Self {
            matrix,
            queue: Vec::new(),
            disabled_shift: 0,
        }
    }

    pub fn get_col(&self, col: u8) -> u8 {
        self.matrix.borrow()[8 + col as usize]
    }

    pub fn get_row(&self, row: u8) -> u8 {
        self.matrix.borrow()[row as usize]
    }

    pub fn drain_event(&mut self) {
        if !self.queue.is_empty() {
            let (key_event, pressed) = self.queue.remove(0);
            if pressed {
                self.on_key_down(key_event)
            } else {
                self.on_key_up(key_event)
            }
        }
    }

    pub fn enqueue(&mut self, str: &str) {
        for c in str.to_string().chars() {
            let key_event = self.map_char(c);
            self.queue.push((key_event, true));
            self.queue.push((key_event, false));
        }
    }

    pub fn has_events(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn reset(&mut self) {
        let mut matrix = self.matrix.borrow_mut();
        for i in 0..16 {
            matrix[i] = 0xff;
        }
        self.queue.clear();
    }

    pub fn set_key(&mut self, keycode: Key, enabled: bool) {
        let mapping = self.map_keycode(keycode);
        self.matrix.borrow_mut()[mapping.0].set_bit(mapping.1, !enabled);
        self.matrix.borrow_mut()[8 + mapping.1].set_bit(mapping.0, !enabled);
    }

    pub fn set_matrix(&mut self, mapping: (usize, usize), enabled: bool) {
        self.matrix.borrow_mut()[mapping.0].set_bit(mapping.1, !enabled);
        self.matrix.borrow_mut()[8 + mapping.1].set_bit(mapping.0, !enabled);
    }

    fn is_pressed(&self, keycode: Key) -> bool {
        let mapping = self.map_keycode(keycode);
        !self.matrix.borrow()[mapping.0].get_bit(mapping.1)
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
            Key::RunStop => (7, 7),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zinc64_core::new_shared;

    #[test]
    fn enqueue_key_event() {
        let matrix = new_shared([0; 16]);
        let mut keyboard = Keyboard::new(matrix);
        keyboard.reset();
        assert_eq!(false, keyboard.has_events());
        keyboard.enqueue("S");
        assert_eq!(true, keyboard.has_events());
    }

    #[test]
    fn drain_key_event() {
        let matrix = new_shared([0; 16]);
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
        let matrix = new_shared([0; 16]);
        let mut keyboard = Keyboard::new(matrix);
        keyboard.reset();
        keyboard.enqueue("S");
        keyboard.drain_event();
        assert_eq!(0xdf, keyboard.get_row(1));
    }
}
