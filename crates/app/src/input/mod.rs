// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::cast_lossless)]

use std::collections::HashSet;

use glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use zinc64_engine::C64;
use zinc64_engine::peripheral::Joystick;
use zinc64_engine::peripheral::joystick::Button;

use self::keymap::KeyMap;

pub mod keymap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoystickDevice {
    None,
    Numpad,
}

pub struct InputSystem {
    joystick_devices: [JoystickDevice; 2],
    pressed_joy_keys: HashSet<VirtualKeyCode>,
    pressed_joy_buttons: Vec<Button>,
}

impl InputSystem {
    pub fn build(joystick_devices: [JoystickDevice; 2]) -> Result<InputSystem, String> {
        Ok(InputSystem {
            joystick_devices,
            pressed_joy_keys: HashSet::new(),
            pressed_joy_buttons: Vec::new(),
        })
    }

    // TODO: track WindowEvent::ModifiersChanged instead of KeyboardInput::modifiers
    #[allow(deprecated)]
    pub fn handle_event(&mut self, c64: &mut C64, event: &Event<()>) {
        if let Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            modifiers,
                            ..
                        },
                    ..
                },
            ..
        } = event
        {
            match state {
                ElementState::Pressed => {
                    if let Some(key_event) = KeyMap::map_key(*virtual_code, *modifiers) {
                        c64.get_keyboard().on_key_down(key_event);
                    }
                    self.handle_joystick_key_down(c64, *virtual_code);
                }
                ElementState::Released => {
                    if let Some(key_event) = KeyMap::map_key(*virtual_code, *modifiers) {
                        c64.get_keyboard().on_key_up(key_event);
                    }
                    self.handle_joystick_key_up(c64, *virtual_code);
                }
            }
        }
    }

    // ---- numpad joystick ----

    fn handle_joystick_key_down(&mut self, c64: &mut C64, key: VirtualKeyCode) {
        if let Some(buttons) = map_numpad_key(key)
            && self.pressed_joy_keys.insert(key)
        {
            for button in buttons {
                self.pressed_joy_buttons.push(button);
                self.for_numpad_joysticks(c64, |joystick| joystick.press(button));
            }
        }
    }

    fn handle_joystick_key_up(&mut self, c64: &mut C64, key: VirtualKeyCode) {
        if let Some(buttons) = map_numpad_key(key)
            && self.pressed_joy_keys.remove(&key)
        {
            for button in buttons {
                if let Some(index) = self.pressed_joy_buttons.iter().position(|b| *b == button) {
                    self.pressed_joy_buttons.remove(index);
                }
                // Another key may still hold the same direction.
                if !self.pressed_joy_buttons.contains(&button) {
                    self.for_numpad_joysticks(c64, |joystick| joystick.release(button));
                }
            }
        }
    }

    fn for_numpad_joysticks(&self, c64: &mut C64, mut f: impl FnMut(&mut Joystick)) {
        if self.joystick_devices[0] == JoystickDevice::Numpad {
            f(c64.get_joystick1_mut());
        }
        if self.joystick_devices[1] == JoystickDevice::Numpad {
            f(c64.get_joystick2_mut());
        }
    }
}

fn map_numpad_key(key: VirtualKeyCode) -> Option<Vec<Button>> {
    let buttons = match key {
        VirtualKeyCode::Numpad0 | VirtualKeyCode::NumpadEnter => vec![Button::Fire],
        VirtualKeyCode::Numpad1 => vec![Button::Down, Button::Left],
        VirtualKeyCode::Numpad2 => vec![Button::Down],
        VirtualKeyCode::Numpad3 => vec![Button::Down, Button::Right],
        VirtualKeyCode::Numpad4 => vec![Button::Left],
        VirtualKeyCode::Numpad6 => vec![Button::Right],
        VirtualKeyCode::Numpad7 => vec![Button::Up, Button::Left],
        VirtualKeyCode::Numpad8 => vec![Button::Up],
        VirtualKeyCode::Numpad9 => vec![Button::Up, Button::Right],
        _ => return None,
    };
    Some(buttons)
}
