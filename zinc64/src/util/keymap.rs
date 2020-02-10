// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use glutin::event::{ModifiersState, VirtualKeyCode};
use zinc64_emu::device::keyboard::{Key, KeyEvent};

pub struct KeyMap;

impl KeyMap {
    pub fn map_key(virtual_code: VirtualKeyCode, modifiers: ModifiersState) -> Option<KeyEvent> {
        match virtual_code {
            // Numerical
            VirtualKeyCode::Key0 if modifiers.shift() => Some(KeyEvent::new(Key::Num9)),
            VirtualKeyCode::Key2 if modifiers.shift() => {
                Some(KeyEvent::with_disabled_shift(Key::At))
            }
            VirtualKeyCode::Key4 if modifiers.shift() => {
                Some(KeyEvent::with_disabled_shift(Key::Dollar))
            }
            VirtualKeyCode::Key6 if modifiers.shift() => Some(KeyEvent::new(Key::Num7)),
            VirtualKeyCode::Key7 if modifiers.shift() => Some(KeyEvent::new(Key::Num6)),
            VirtualKeyCode::Key8 if modifiers.shift() => {
                Some(KeyEvent::with_disabled_shift(Key::Asterisk))
            }
            VirtualKeyCode::Key9 if modifiers.shift() => Some(KeyEvent::new(Key::Num8)),
            VirtualKeyCode::Key0 => Some(KeyEvent::new(Key::Num0)),
            VirtualKeyCode::Key1 => Some(KeyEvent::new(Key::Num1)),
            VirtualKeyCode::Key2 => Some(KeyEvent::new(Key::Num2)),
            VirtualKeyCode::Key3 => Some(KeyEvent::new(Key::Num3)),
            VirtualKeyCode::Key4 => Some(KeyEvent::new(Key::Num4)),
            VirtualKeyCode::Key5 => Some(KeyEvent::new(Key::Num5)),
            VirtualKeyCode::Key6 => Some(KeyEvent::new(Key::Num6)),
            VirtualKeyCode::Key7 => Some(KeyEvent::new(Key::Num7)),
            VirtualKeyCode::Key8 => Some(KeyEvent::new(Key::Num8)),
            VirtualKeyCode::Key9 => Some(KeyEvent::new(Key::Num9)),
            // Alpha
            VirtualKeyCode::A => Some(KeyEvent::new(Key::A)),
            VirtualKeyCode::B => Some(KeyEvent::new(Key::B)),
            VirtualKeyCode::C => Some(KeyEvent::new(Key::C)),
            VirtualKeyCode::D => Some(KeyEvent::new(Key::D)),
            VirtualKeyCode::E => Some(KeyEvent::new(Key::E)),
            VirtualKeyCode::F => Some(KeyEvent::new(Key::F)),
            VirtualKeyCode::G => Some(KeyEvent::new(Key::G)),
            VirtualKeyCode::H => Some(KeyEvent::new(Key::H)),
            VirtualKeyCode::I => Some(KeyEvent::new(Key::I)),
            VirtualKeyCode::J => Some(KeyEvent::new(Key::J)),
            VirtualKeyCode::K => Some(KeyEvent::new(Key::K)),
            VirtualKeyCode::L => Some(KeyEvent::new(Key::L)),
            VirtualKeyCode::M => Some(KeyEvent::new(Key::M)),
            VirtualKeyCode::N => Some(KeyEvent::new(Key::N)),
            VirtualKeyCode::O => Some(KeyEvent::new(Key::O)),
            VirtualKeyCode::P => Some(KeyEvent::new(Key::P)),
            VirtualKeyCode::Q => Some(KeyEvent::new(Key::Q)),
            VirtualKeyCode::R => Some(KeyEvent::new(Key::R)),
            VirtualKeyCode::S => Some(KeyEvent::new(Key::S)),
            VirtualKeyCode::T => Some(KeyEvent::new(Key::T)),
            VirtualKeyCode::U => Some(KeyEvent::new(Key::U)),
            VirtualKeyCode::V => Some(KeyEvent::new(Key::V)),
            VirtualKeyCode::W => Some(KeyEvent::new(Key::W)),
            VirtualKeyCode::X => Some(KeyEvent::new(Key::X)),
            VirtualKeyCode::Y => Some(KeyEvent::new(Key::Y)),
            VirtualKeyCode::Z => Some(KeyEvent::new(Key::Z)),
            // Control
            VirtualKeyCode::Back => Some(KeyEvent::new(Key::Backspace)),
            VirtualKeyCode::Down => Some(KeyEvent::new(Key::CrsrDown)),
            VirtualKeyCode::Home => Some(KeyEvent::new(Key::Home)),
            VirtualKeyCode::LControl => Some(KeyEvent::new(Key::Ctrl)),
            VirtualKeyCode::Left => Some(KeyEvent::with_mod(Key::CrsrRight, Key::LShift)),
            VirtualKeyCode::LWin => Some(KeyEvent::new(Key::LGui)),
            VirtualKeyCode::LShift => Some(KeyEvent::new(Key::LShift)),
            VirtualKeyCode::RControl => Some(KeyEvent::new(Key::Ctrl)),
            VirtualKeyCode::Return => Some(KeyEvent::new(Key::Return)),
            VirtualKeyCode::Right => Some(KeyEvent::new(Key::CrsrRight)),
            VirtualKeyCode::RShift => Some(KeyEvent::new(Key::RShift)),
            VirtualKeyCode::Tab => Some(KeyEvent::new(Key::RunStop)),
            VirtualKeyCode::Up => Some(KeyEvent::with_mod(Key::CrsrDown, Key::LShift)),
            // Function
            VirtualKeyCode::F1 => Some(KeyEvent::new(Key::F1)),
            VirtualKeyCode::F3 => Some(KeyEvent::new(Key::F3)),
            VirtualKeyCode::F5 => Some(KeyEvent::new(Key::F5)),
            VirtualKeyCode::F7 => Some(KeyEvent::new(Key::F7)),
            // Symbols
            VirtualKeyCode::At => Some(KeyEvent::new(Key::At)),
            VirtualKeyCode::Backslash if modifiers.shift() => {
                Some(KeyEvent::with_mod(Key::Minus, Key::LShift))
            }
            VirtualKeyCode::Caret => Some(KeyEvent::new(Key::Caret)),
            VirtualKeyCode::Comma => Some(KeyEvent::new(Key::Comma)),
            VirtualKeyCode::Equals if modifiers.shift() => {
                Some(KeyEvent::with_disabled_shift(Key::Plus))
            }
            VirtualKeyCode::Equals => Some(KeyEvent::new(Key::Equals)),
            VirtualKeyCode::LBracket => Some(KeyEvent::with_mod(Key::Colon, Key::LShift)),
            VirtualKeyCode::Minus => Some(KeyEvent::new(Key::Minus)),
            VirtualKeyCode::Period => Some(KeyEvent::new(Key::Period)),
            VirtualKeyCode::RBracket => Some(KeyEvent::with_mod(Key::Semicolon, Key::LShift)),
            VirtualKeyCode::Semicolon if modifiers.shift() => {
                Some(KeyEvent::with_disabled_shift(Key::Colon))
            }
            VirtualKeyCode::Semicolon => Some(KeyEvent::new(Key::Semicolon)),
            VirtualKeyCode::Slash => Some(KeyEvent::new(Key::Slash)),
            VirtualKeyCode::Space => Some(KeyEvent::new(Key::Space)),
            // VirtualKeyCode::Quote if modifiers.shift() => Some(KeyEvent::new(Key::Num2))
            // VirtualKeyCode::Quote => Some(KeyEvent::with_mod(Key::Num7, Key::LShift)),
            _ => None,
        }
    }
}

pub fn to_ascii2(virtual_code: VirtualKeyCode, modifiers: ModifiersState) -> char {
    let c = if modifiers.shift() {
        match virtual_code {
            VirtualKeyCode::Key0 => '!',
            VirtualKeyCode::Key1 => '@',
            VirtualKeyCode::Key2 => '#',
            VirtualKeyCode::Key3 => '$',
            VirtualKeyCode::Key4 => '%',
            VirtualKeyCode::Key5 => '^',
            VirtualKeyCode::Key6 => '&',
            VirtualKeyCode::Key7 => '*',
            VirtualKeyCode::Key8 => '(',
            VirtualKeyCode::Key9 => ')',
            VirtualKeyCode::A => 'A',
            VirtualKeyCode::B => 'B',
            VirtualKeyCode::C => 'C',
            VirtualKeyCode::D => 'D',
            VirtualKeyCode::E => 'E',
            VirtualKeyCode::F => 'F',
            VirtualKeyCode::G => 'G',
            VirtualKeyCode::H => 'H',
            VirtualKeyCode::I => 'I',
            VirtualKeyCode::J => 'J',
            VirtualKeyCode::K => 'K',
            VirtualKeyCode::L => 'L',
            VirtualKeyCode::M => 'M',
            VirtualKeyCode::N => 'N',
            VirtualKeyCode::O => 'O',
            VirtualKeyCode::P => 'P',
            VirtualKeyCode::Q => 'Q',
            VirtualKeyCode::R => 'R',
            VirtualKeyCode::S => 'S',
            VirtualKeyCode::T => 'T',
            VirtualKeyCode::U => 'U',
            VirtualKeyCode::V => 'V',
            VirtualKeyCode::W => 'W',
            VirtualKeyCode::X => 'X',
            VirtualKeyCode::Y => 'Y',
            VirtualKeyCode::Z => 'Z',
            VirtualKeyCode::Apostrophe => '"',
            VirtualKeyCode::Backslash => '|',
            VirtualKeyCode::Comma => '<',
            VirtualKeyCode::Equals => '+',
            VirtualKeyCode::LBracket => '{',
            VirtualKeyCode::Minus => '_',
            VirtualKeyCode::Period => '>',
            VirtualKeyCode::Slash => '?',
            VirtualKeyCode::RBracket => '}',
            VirtualKeyCode::Semicolon => ':',
            _ => '\0',
        }
    } else {
        '\0'
    };
    if c != '\0' {
        c
    } else {
        match virtual_code {
            VirtualKeyCode::Key0 => '0',
            VirtualKeyCode::Key1 => '1',
            VirtualKeyCode::Key2 => '2',
            VirtualKeyCode::Key3 => '3',
            VirtualKeyCode::Key4 => '4',
            VirtualKeyCode::Key5 => '5',
            VirtualKeyCode::Key6 => '6',
            VirtualKeyCode::Key7 => '7',
            VirtualKeyCode::Key8 => '8',
            VirtualKeyCode::Key9 => '9',
            VirtualKeyCode::A => 'a',
            VirtualKeyCode::B => 'b',
            VirtualKeyCode::C => 'c',
            VirtualKeyCode::D => 'd',
            VirtualKeyCode::E => 'e',
            VirtualKeyCode::F => 'f',
            VirtualKeyCode::G => 'g',
            VirtualKeyCode::H => 'h',
            VirtualKeyCode::I => 'i',
            VirtualKeyCode::J => 'j',
            VirtualKeyCode::K => 'k',
            VirtualKeyCode::L => 'l',
            VirtualKeyCode::M => 'm',
            VirtualKeyCode::N => 'n',
            VirtualKeyCode::O => 'o',
            VirtualKeyCode::P => 'p',
            VirtualKeyCode::Q => 'q',
            VirtualKeyCode::R => 'r',
            VirtualKeyCode::S => 's',
            VirtualKeyCode::T => 't',
            VirtualKeyCode::U => 'u',
            VirtualKeyCode::V => 'v',
            VirtualKeyCode::W => 'w',
            VirtualKeyCode::X => 'x',
            VirtualKeyCode::Y => 'y',
            VirtualKeyCode::Z => 'z',
            VirtualKeyCode::Backslash => '\\',
            VirtualKeyCode::Equals => '=',
            VirtualKeyCode::Colon => ':',
            VirtualKeyCode::Comma => ',',
            VirtualKeyCode::LBracket => '[',
            VirtualKeyCode::Minus => '-',
            VirtualKeyCode::Period => '.',
            VirtualKeyCode::RBracket => ']',
            VirtualKeyCode::Slash => '/',
            VirtualKeyCode::Semicolon => ';',
            VirtualKeyCode::Space => ' ',
            VirtualKeyCode::Back => '\0',
            VirtualKeyCode::Tab => '\t',
            VirtualKeyCode::Return => '\n',
            VirtualKeyCode::Escape => '\0',
            _ => '\0',
        }
    }
}
