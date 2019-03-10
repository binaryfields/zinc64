// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::collections::HashSet;

use sdl2;
use sdl2::event::Event;
use sdl2::joystick;
use sdl2::keyboard::{Keycode, Mod};
use zinc64_core::Shared;
use zinc64_emu::device::joystick::Button;
use zinc64_emu::device::{Joystick, Keyboard};

use crate::util::keymap::KeyMap;

pub struct InputSystem {
    // Components
    keyboard: Shared<Keyboard>,
    joystick1: Option<Shared<Joystick>>,
    joystick2: Option<Shared<Joystick>>,
    // Resources
    #[allow(dead_code)]
    sdl_joystick1: Option<joystick::Joystick>,
    #[allow(dead_code)]
    sdl_joystick2: Option<joystick::Joystick>,
    // Runtime state
    pressed_joy_keys: HashSet<Keycode>,
    pressed_joy_buttons: Vec<Button>,
}

impl InputSystem {
    pub fn build(
        sdl_joystick: &sdl2::JoystickSubsystem,
        keyboard: Shared<Keyboard>,
        joystick1: Option<Shared<Joystick>>,
        joystick2: Option<Shared<Joystick>>,
    ) -> Result<InputSystem, String> {
        sdl_joystick.set_event_state(true);
        let sdl_joystick1 = joystick1.as_ref().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                sdl_joystick.open(joystick.borrow().get_index() as u32).ok()
            } else {
                None
            }
        });
        let sdl_joystick2 = joystick2.as_ref().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                sdl_joystick.open(joystick.borrow().get_index() as u32).ok()
            } else {
                None
            }
        });
        let input_system = InputSystem {
            //event_pump: sdl_context.event_pump().unwrap(),
            keyboard,
            joystick1,
            joystick2,
            sdl_joystick1,
            sdl_joystick2,
            pressed_joy_keys: HashSet::new(),
            pressed_joy_buttons: Vec::new(),
        };
        Ok(input_system)
    }

    pub fn handle_event(&mut self, event: &Event) {
        match *event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(key_event) = KeyMap::map_key(key, keymod) {
                    self.keyboard.borrow_mut().on_key_down(key_event);
                }
            }
            Event::KeyUp {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(key_event) = KeyMap::map_key(key, keymod) {
                    self.keyboard.borrow_mut().on_key_up(key_event);
                }
            }
            _ => {}
        }
        self.handle_joystick_event(event);
    }

    fn get_joystick(&self, index: u8) -> Option<Shared<Joystick>> {
        if let Some(ref joystick) = self.joystick1 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        if let Some(ref joystick) = self.joystick2 {
            if joystick.borrow().get_index() == index {
                return Some(joystick.clone());
            }
        }
        None
    }

    fn handle_joystick_button_down(&mut self, button: Button) {
        self.pressed_joy_buttons.push(button);
        if let Some(ref mut joystick) = self.joystick1 {
            if joystick.borrow().is_virtual() {
                joystick.borrow_mut().on_key_down(button);
            }
        }
        if let Some(ref mut joystick) = self.joystick2 {
            if joystick.borrow().is_virtual() {
                joystick.borrow_mut().on_key_down(button);
            }
        }
    }

    fn handle_joystick_button_up(&mut self, button: Button) {
        if let Some(index) = self.pressed_joy_buttons.iter().position(|b| *b == button) {
            self.pressed_joy_buttons.remove(index);
        }
        if !self.pressed_joy_buttons.contains(&button) {
            if let Some(ref mut joystick) = self.joystick1 {
                if joystick.borrow().is_virtual() {
                    joystick.borrow_mut().on_key_up(button);
                }
            }
            if let Some(ref mut joystick) = self.joystick2 {
                if joystick.borrow().is_virtual() {
                    joystick.borrow_mut().on_key_up(button);
                }
            }
        }
    }

    fn handle_joystick_event(&mut self, event: &Event) {
        match *event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                if let Some(buttons) = self.map_joystick_key(key, keymod) {
                    if !self.pressed_joy_keys.contains(&key) {
                        self.pressed_joy_keys.insert(key);
                        self.handle_joystick_button_down(buttons.0);
                        if let Some(button1) = buttons.1 {
                            self.handle_joystick_button_down(button1);
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
                    self.handle_joystick_button_up(buttons.0);
                    if let Some(button1) = buttons.1 {
                        self.handle_joystick_button_up(button1);
                    }
                }
            }
            Event::JoyAxisMotion {
                which,
                axis_idx,
                value,
                ..
            } => {
                if let Some(ref mut joystick) = self.get_joystick(which as u8) {
                    joystick.borrow_mut().on_axis_motion(axis_idx, value);
                }
            }
            Event::JoyButtonDown {
                which, button_idx, ..
            } => {
                if let Some(ref mut joystick) = self.get_joystick(which as u8) {
                    joystick.borrow_mut().on_button_down(button_idx);
                }
            }
            Event::JoyButtonUp {
                which, button_idx, ..
            } => {
                if let Some(ref mut joystick) = self.get_joystick(which as u8) {
                    joystick.borrow_mut().on_button_up(button_idx);
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
