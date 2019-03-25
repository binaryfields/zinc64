// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::collections::HashSet;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use zinc64_emu::device::joystick::Button;

use crate::util::keymap::KeyMap;
use zinc64_emu::system::C64;

pub struct InputSystem {
    pressed_joy_keys: HashSet<Keycode>,
    pressed_joy_buttons: Vec<Button>,
}

impl InputSystem {
    pub fn build() -> Result<InputSystem, String> {
        Ok(InputSystem {
            pressed_joy_keys: HashSet::new(),
            pressed_joy_buttons: Vec::new(),
        })
    }

    pub fn handle_event(&mut self, c64: &mut C64, event: &Event) {
        match *event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(key_event) = KeyMap::map_key(key, keymod) {
                    c64.get_keyboard().on_key_down(key_event);
                }
            }
            Event::KeyUp {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(key_event) = KeyMap::map_key(key, keymod) {
                    c64.get_keyboard().on_key_up(key_event);
                }
            }
            _ => {}
        }
        self.handle_joystick_event(c64, event);
    }

    fn handle_joystick_button_down(&mut self, c64: &mut C64, button: Button) {
        self.pressed_joy_buttons.push(button);
        if let Some(ref mut joystick) = c64.get_joystick1_mut() {
            if joystick.is_virtual() {
                joystick.on_key_down(button);
            }
        }
        if let Some(ref mut joystick) = c64.get_joystick2_mut() {
            if joystick.is_virtual() {
                joystick.on_key_down(button);
            }
        }
    }

    fn handle_joystick_button_up(&mut self, c64: &mut C64, button: Button) {
        if let Some(index) = self.pressed_joy_buttons.iter().position(|b| *b == button) {
            self.pressed_joy_buttons.remove(index);
        }
        if !self.pressed_joy_buttons.contains(&button) {
            if let Some(ref mut joystick) = c64.get_joystick1_mut() {
                if joystick.is_virtual() {
                    joystick.on_key_up(button);
                }
            }
            if let Some(ref mut joystick) = c64.get_joystick1_mut() {
                if joystick.is_virtual() {
                    joystick.on_key_up(button);
                }
            }
        }
    }

    fn handle_joystick_event(&mut self, c64: &mut C64, event: &Event) {
        match *event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(buttons) = self.map_joystick_key(key, keymod) {
                    if !self.pressed_joy_keys.contains(&key) {
                        self.pressed_joy_keys.insert(key);
                        self.handle_joystick_button_down(c64, buttons.0);
                        if let Some(button1) = buttons.1 {
                            self.handle_joystick_button_down(c64, button1);
                        }
                    }
                }
            }
            Event::KeyUp {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(buttons) = self.map_joystick_key(key, keymod) {
                    self.pressed_joy_keys.remove(&key);
                    self.handle_joystick_button_up(c64, buttons.0);
                    if let Some(button1) = buttons.1 {
                        self.handle_joystick_button_up(c64, button1);
                    }
                }
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                ..
            } => {
                if let Some(ref mut joystick) = c64.get_joystick1_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_axis_motion(axis_idx, value);
                    }
                }
                if let Some(ref mut joystick) = c64.get_joystick2_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_axis_motion(axis_idx, value);
                    }
                }
            }
            Event::JoyButtonDown {
                which, button_idx, ..
            } => {
                if let Some(ref mut joystick) = c64.get_joystick1_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_button_down(button_idx);
                    }
                }
                if let Some(ref mut joystick) = c64.get_joystick2_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_button_down(button_idx);
                    }
                }
            }
            Event::JoyButtonUp {
                which, button_idx, ..
            } => {
                if let Some(ref mut joystick) = c64.get_joystick1_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_button_up(button_idx);
                    }
                }
                if let Some(ref mut joystick) = c64.get_joystick2_mut() {
                    if joystick.get_index() == which as u8 {
                        joystick.on_button_up(button_idx);
                    }
                }
            }
            _ => {}
        }
    }

    fn map_joystick_key(&self, keycode: Keycode, _keymod: Mod) -> Option<(Button, Option<Button>)> {
        match keycode {
            Keycode::Kp0 => Some((Button::Fire, None)),
            Keycode::Kp1 => Some((Button::Down, Some(Button::Left))),
            Keycode::Kp2 => Some((Button::Down, None)),
            Keycode::Kp3 => Some((Button::Down, Some(Button::Right))),
            Keycode::Kp4 => Some((Button::Left, None)),
            Keycode::Kp6 => Some((Button::Right, None)),
            Keycode::Kp7 => Some((Button::Up, Some(Button::Left))),
            Keycode::Kp8 => Some((Button::Up, None)),
            Keycode::Kp9 => Some((Button::Up, Some(Button::Right))),
            Keycode::KpEnter => Some((Button::Fire, None)),
            _ => None,
        }
    }
}
