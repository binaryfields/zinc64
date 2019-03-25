// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use sdl2::keyboard;
use sdl2::keyboard::{Keycode, Mod};
use zinc64_emu::device::keyboard::{Key, KeyEvent};

pub struct KeyMap;

impl KeyMap {
    pub fn map_key(keycode: Keycode, keymod: Mod) -> Option<KeyEvent> {
        match keycode {
            // Numerical
            Keycode::Num0
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num9))
            }
            Keycode::Num0 => Some(KeyEvent::new(Key::Num0)),
            Keycode::Num1 => Some(KeyEvent::new(Key::Num1)),
            Keycode::Num2
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::At))
            }
            Keycode::Num2 => Some(KeyEvent::new(Key::Num2)),
            Keycode::Num3 => Some(KeyEvent::new(Key::Num3)),
            Keycode::Num4 => Some(KeyEvent::new(Key::Num4)),
            Keycode::Num5 => Some(KeyEvent::new(Key::Num5)),
            Keycode::Num6
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num7))
            }
            Keycode::Num6 => Some(KeyEvent::new(Key::Num6)),
            Keycode::Num7
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num6))
            }
            Keycode::Num7 => Some(KeyEvent::new(Key::Num7)),
            Keycode::Num8
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Asterisk))
            }
            Keycode::Num8 => Some(KeyEvent::new(Key::Num8)),
            Keycode::Num9
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num8))
            }
            Keycode::Num9 => Some(KeyEvent::new(Key::Num9)),
            // Alpha
            Keycode::A => Some(KeyEvent::new(Key::A)),
            Keycode::B => Some(KeyEvent::new(Key::B)),
            Keycode::C => Some(KeyEvent::new(Key::C)),
            Keycode::D => Some(KeyEvent::new(Key::D)),
            Keycode::E => Some(KeyEvent::new(Key::E)),
            Keycode::F => Some(KeyEvent::new(Key::F)),
            Keycode::G => Some(KeyEvent::new(Key::G)),
            Keycode::H => Some(KeyEvent::new(Key::H)),
            Keycode::I => Some(KeyEvent::new(Key::I)),
            Keycode::J => Some(KeyEvent::new(Key::J)),
            Keycode::K => Some(KeyEvent::new(Key::K)),
            Keycode::L => Some(KeyEvent::new(Key::L)),
            Keycode::M => Some(KeyEvent::new(Key::M)),
            Keycode::N => Some(KeyEvent::new(Key::N)),
            Keycode::O => Some(KeyEvent::new(Key::O)),
            Keycode::P => Some(KeyEvent::new(Key::P)),
            Keycode::Q => Some(KeyEvent::new(Key::Q)),
            Keycode::R => Some(KeyEvent::new(Key::R)),
            Keycode::S => Some(KeyEvent::new(Key::S)),
            Keycode::T => Some(KeyEvent::new(Key::T)),
            Keycode::U => Some(KeyEvent::new(Key::U)),
            Keycode::V => Some(KeyEvent::new(Key::V)),
            Keycode::W => Some(KeyEvent::new(Key::W)),
            Keycode::X => Some(KeyEvent::new(Key::X)),
            Keycode::Y => Some(KeyEvent::new(Key::Y)),
            Keycode::Z => Some(KeyEvent::new(Key::Z)),
            // Symbols
            Keycode::Asterisk => Some(KeyEvent::new(Key::Asterisk)),
            Keycode::At => Some(KeyEvent::new(Key::At)),
            Keycode::Backslash
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_mod(Key::Minus, Key::LShift))
            }
            Keycode::Backspace => Some(KeyEvent::new(Key::Backspace)),
            Keycode::Caret => Some(KeyEvent::new(Key::Caret)),
            Keycode::Colon => Some(KeyEvent::new(Key::Colon)),
            Keycode::Comma => Some(KeyEvent::new(Key::Comma)),
            Keycode::Dollar => Some(KeyEvent::new(Key::Dollar)),
            Keycode::Equals
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Plus))
            }
            Keycode::Equals => Some(KeyEvent::new(Key::Equals)),
            Keycode::LeftBracket => Some(KeyEvent::with_mod(Key::Colon, Key::LShift)),
            Keycode::Minus => Some(KeyEvent::new(Key::Minus)),
            Keycode::Period => Some(KeyEvent::new(Key::Period)),
            Keycode::Plus => Some(KeyEvent::new(Key::Plus)),
            Keycode::Quote
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num2))
            }
            Keycode::Quote => Some(KeyEvent::with_mod(Key::Num7, Key::LShift)),
            Keycode::Return => Some(KeyEvent::new(Key::Return)),
            Keycode::RightBracket => Some(KeyEvent::with_mod(Key::Semicolon, Key::LShift)),
            Keycode::Semicolon
                if keymod.contains(keyboard::Mod::LSHIFTMOD)
                    || keymod.contains(keyboard::Mod::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Colon))
            }
            Keycode::Semicolon => Some(KeyEvent::new(Key::Semicolon)),
            Keycode::Slash => Some(KeyEvent::new(Key::Slash)),
            Keycode::Space => Some(KeyEvent::new(Key::Space)),
            // Control
            Keycode::Down => Some(KeyEvent::new(Key::CrsrDown)),
            Keycode::Home => Some(KeyEvent::new(Key::Home)),
            Keycode::LCtrl => Some(KeyEvent::new(Key::Ctrl)),
            Keycode::Left => Some(KeyEvent::with_mod(Key::CrsrRight, Key::LShift)),
            Keycode::LGui => Some(KeyEvent::new(Key::LGui)),
            Keycode::LShift => Some(KeyEvent::new(Key::LShift)),
            Keycode::Pause => Some(KeyEvent::new(Key::Pause)),
            Keycode::RCtrl => Some(KeyEvent::new(Key::Ctrl)),
            Keycode::Right => Some(KeyEvent::new(Key::CrsrRight)),
            Keycode::RShift => Some(KeyEvent::new(Key::RShift)),
            Keycode::Up => Some(KeyEvent::with_mod(Key::CrsrDown, Key::LShift)),
            // Function
            Keycode::F1 => Some(KeyEvent::new(Key::F1)),
            Keycode::F3 => Some(KeyEvent::new(Key::F3)),
            Keycode::F5 => Some(KeyEvent::new(Key::F5)),
            Keycode::F7 => Some(KeyEvent::new(Key::F7)),
            _ => None,
        }
    }
}
