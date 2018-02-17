/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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
use std::rc::Rc;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::joystick;
use zinc64::device::{Joystick, Keyboard};
use zinc64::device::joystick::Button;

use super::keymap::KeyMap;

pub struct Io {
    //event_pump: EventPump,
    keyboard: Rc<RefCell<Keyboard>>,
    joystick1: Option<Rc<RefCell<Joystick>>>,
    joystick2: Option<Rc<RefCell<Joystick>>>,
    #[allow(dead_code)] sdl_joystick1: Option<joystick::Joystick>,
    #[allow(dead_code)] sdl_joystick2: Option<joystick::Joystick>,
}

impl Io {
    pub fn new(
        sdl_joystick: &sdl2::JoystickSubsystem,
        keyboard: Rc<RefCell<Keyboard>>,
        joystick1: Option<Rc<RefCell<Joystick>>>,
        joystick2: Option<Rc<RefCell<Joystick>>>,
    ) -> Result<Io, String> {
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
        let io = Io {
            //event_pump: sdl_context.event_pump().unwrap(),
            keyboard,
            joystick1,
            joystick2,
            sdl_joystick1,
            sdl_joystick2,
        };
        Ok(io)
    }

    pub fn handle_event(&self, event: &Event) {
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

    fn get_joystick(&self, index: u8) -> Option<Rc<RefCell<Joystick>>> {
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

    fn handle_joystick_event(&self, event: &Event) {
        match *event {
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                ..
            } => {
                let is_virtual_1 = self.joystick1
                    .as_ref()
                    .map(|joystick| joystick.borrow().is_virtual())
                    .unwrap_or(false);
                let is_virtual_2 = self.joystick2
                    .as_ref()
                    .map(|joystick| joystick.borrow().is_virtual())
                    .unwrap_or(false);
                if is_virtual_1 || is_virtual_2 {
                    if let Some(joy_button) = self.map_joystick_key(key, keymod) {
                        if is_virtual_1 {
                            if let Some(ref joystick) = self.joystick1 {
                                joystick.borrow_mut().on_key_down(joy_button);
                            }
                        }
                        if is_virtual_2 {
                            if let Some(ref joystick) = self.joystick2 {
                                joystick.borrow_mut().on_key_down(joy_button);
                            }
                        }
                    }
                }
            }
            Event::KeyUp {
                keycode: Some(key),
                keymod,
                ..
            } => {
                let is_virtual_1 = self.joystick1
                    .as_ref()
                    .map(|joystick| joystick.borrow().is_virtual())
                    .unwrap_or(false);
                let is_virtual_2 = self.joystick2
                    .as_ref()
                    .map(|joystick| joystick.borrow().is_virtual())
                    .unwrap_or(false);
                if is_virtual_1 || is_virtual_2 {
                    if let Some(joy_button) = self.map_joystick_key(key, keymod) {
                        if is_virtual_1 {
                            if let Some(ref joystick) = self.joystick1 {
                                joystick.borrow_mut().on_key_up(joy_button);
                            }
                        }
                        if is_virtual_2 {
                            if let Some(ref joystick) = self.joystick2 {
                                joystick.borrow_mut().on_key_up(joy_button);
                            }
                        }
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

    fn map_joystick_key(&self, keycode: Keycode, _keymod: Mod) -> Option<Button> {
        match keycode {
            Keycode::Kp2 => Some(Button::Down),
            Keycode::Kp4 => Some(Button::Left),
            Keycode::Kp6 => Some(Button::Right),
            Keycode::Kp8 => Some(Button::Up),
            Keycode::KpEnter => Some(Button::Fire),
            _ => None,
        }
    }
}
