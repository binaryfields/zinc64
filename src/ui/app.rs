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

use std::result::Result;
use std::thread;
use std::time::Duration;

use c64::C64;
use sdl2;
use sdl2::{EventPump, Sdl};
use sdl2::event::Event;
use sdl2::joystick::Joystick;
use sdl2::keyboard;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};
use sdl2::video::{FullscreenType, Window};
use time;

pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

impl JamAction {
    pub fn from(action: &str) -> JamAction {
        match action {
            "continue" => JamAction::Continue,
            "quit" => JamAction::Quit,
            "reset" => JamAction::Reset,
            _ => panic!("invalid jam action {}", action),
        }
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Running,
    Paused,
    Stopped,
    Trapped,
}

pub struct Options {
    pub fullscreen: bool,
    pub jam_action: JamAction,
    pub height: u32,
    pub width: u32,
}

pub struct AppWindow {
    // Dependencies
    c64: C64,
    // Renderer
    renderer: Renderer<'static>,
    texture: Texture,
    sdl: Sdl,
    // Devices
    joystick1: Option<Joystick>,
    joystick2: Option<Joystick>,
    // Configuration
    jam_action: JamAction,
    // Runtime State
    state: State,
    last_frame_ts: u64,
    next_keyboard_event: u64,
}

impl AppWindow {
    pub fn new(c64: C64, options: Options) -> Result<AppWindow, String> {
        let sdl = sdl2::init()?;
        // Initialize renderer
        info!(target: "ui", "Opening app window {}x{}", options.width, options.height);
        let video = sdl.video()?;
        let mut builder = video.window("zinc64", options.width, options.height);
        builder.position_centered();
        builder.opengl();
        if options.fullscreen {
            builder.fullscreen();
        }
        let window = builder.build().unwrap();
        let renderer = window.renderer()
            .accelerated()
            .build()
            .unwrap();
        let screen_size = c64.get_config().visible_size;
        let texture = renderer.create_texture_streaming(PixelFormatEnum::ARGB8888,
                                                        screen_size.width as u32,
                                                        screen_size.height as u32)
            .unwrap();
        // Initialize devices
        let joystick_subsystem = sdl.joystick()?;
        joystick_subsystem.set_event_state(true);
        let joystick1 = c64.get_joystick1().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                joystick_subsystem.open(joystick.borrow().get_index() as u32).ok()
            } else {
                None
            }
        });
        let joystick2 = c64.get_joystick2().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                joystick_subsystem.open(joystick.borrow().get_index() as u32).ok()
            } else {
                None
            }
        });
        Ok(
            AppWindow {
                c64: c64,
                sdl: sdl,
                renderer: renderer,
                texture: texture,
                joystick1: joystick1,
                joystick2: joystick2,
                jam_action: options.jam_action,
                state: State::Running,
                last_frame_ts: 0,
                next_keyboard_event: 0,
            }
        )
    }

    pub fn run(&mut self) {
        info!(target: "ui", "Running main loop");
        let mut events = self.sdl.event_pump().unwrap();
        let mut overflow_cycles = 0;
        'running: loop {
            match self.state {
                State::Running => {
                    self.handle_events(&mut events);
                    overflow_cycles = self.c64.run_frame(overflow_cycles);
                    if self.c64.is_cpu_jam() {
                        self.handle_cpu_jam();
                    }
                    let rt = self.c64.get_render_target();
                    if rt.borrow().get_sync() {
                        self.render();
                    }
                },
                State::Paused => {
                    self.handle_events(&mut events);
                    let wait = Duration::from_millis(20);
                    thread::sleep(wait);
                },
                State::Stopped => {
                    info!(target: "ui", "State {:?}", self.state);
                    break 'running;
                },
                State::Trapped => {
                    let cpu = self.c64.get_cpu();
                    info!(target: "ui", "State {:?} at 0x{:x}", self.state, cpu.borrow().get_pc());
                    break 'running;
                },
            }
        }
    }

    fn handle_cpu_jam(&mut self) -> bool {
        let cpu = self.c64.get_cpu();
        warn!(target: "ui", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
        match self.jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                self.state = State::Stopped;
                false
            },
            JamAction::Reset => {
                self.reset();
                false
            },
        }
    }

    fn render(&mut self) {
        let rt = self.c64.get_render_target();
        self.texture.update(None, rt.borrow().get_pixel_data(), rt.borrow().get_pitch());
        self.renderer.clear();
        self.renderer.copy(&self.texture, None, None).unwrap();
        self.renderer.present();
        rt.borrow_mut().set_sync(false);
        self.last_frame_ts = time::precise_time_ns();
    }

    fn reset(&mut self) {
        self.c64.reset();
        self.next_keyboard_event = 0;
    }

    fn toggle_fullscreen(&mut self) {
        match self.renderer.window_mut() {
            Some(ref mut window) => {
                match window.fullscreen_state() {
                    FullscreenType::Off => {
                        window.set_fullscreen(FullscreenType::True).unwrap();
                    },
                    FullscreenType::True => {
                        window.set_fullscreen(FullscreenType::Off).unwrap();
                    },
                    _ => panic!("invalid fullscreen mode"),
                }
            },
            None => panic!("invalid window"),
        }
    }

    fn toggle_pause(&mut self) {
        match self.state {
            State::Running => self.state = State::Paused,
            State::Paused => self.state = State::Running,
            _ => {},
        }
    }

    fn toggle_warp(&mut self) {
        let warp_mode = self.c64.get_warp_mode();
        self.c64.set_warp_mode(!warp_mode);
    }

    // -- Event Handling

    fn handle_events(&mut self, events: &mut EventPump) {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    self.state = State::Stopped;
                },
                Event::KeyDown { keycode: Some(Keycode::P), keymod: keymod, repeat: false, .. }
                if keymod.contains(keyboard::LALTMOD) => {
                    self.toggle_pause();
                },
                Event::KeyDown { keycode: Some(Keycode::Q), keymod: keymod, repeat: false, .. }
                if keymod.contains(keyboard::LALTMOD) => {
                    self.state = State::Stopped;
                },
                Event::KeyDown { keycode: Some(Keycode::W), keymod: keymod, repeat: false, .. }
                if keymod.contains(keyboard::LALTMOD) => {
                    self.toggle_warp();
                },
                Event::KeyDown { keycode: Some(Keycode::Return), keymod: keymod, repeat: false, .. }
                if keymod.contains(keyboard::LALTMOD) => {
                    self.toggle_fullscreen();
                },
                Event::KeyDown { keycode: Some(Keycode::F9), keymod: keymod, repeat: false, .. }
                if keymod.contains(keyboard::LALTMOD) => {
                    self.c64.reset();
                }
                Event::KeyDown { keycode: Some(key), .. } => {
                    let keyboard = self.c64.get_keyboard();
                    keyboard.borrow_mut().on_key_down(key);
                    if let Some(ref mut joystick) = self.c64.get_joystick1() {
                        if joystick.borrow().is_virtual() {
                            joystick.borrow_mut().on_key_down(key);
                        }
                    }
                    if let Some(ref mut joystick) = self.c64.get_joystick2() {
                        if joystick.borrow().is_virtual() {
                            joystick.borrow_mut().on_key_down(key);
                        }
                    }
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    let keyboard = self.c64.get_keyboard();
                    keyboard.borrow_mut().on_key_up(key);
                    if let Some(ref mut joystick) = self.c64.get_joystick1() {
                        if joystick.borrow().is_virtual() {
                            joystick.borrow_mut().on_key_up(key);
                        }
                    }
                    if let Some(ref mut joystick) = self.c64.get_joystick2() {
                        if joystick.borrow().is_virtual() {
                            joystick.borrow_mut().on_key_up(key);
                        }
                    }
                },
                Event::JoyAxisMotion { which: which, axis_idx: axis_idx, value: value, .. } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_axis_motion(axis_idx, value);
                    }
                },
                Event::JoyButtonDown { which: which, button_idx: button_idx, .. } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_button_down(button_idx);
                    }
                },
                Event::JoyButtonUp { which: which, button_idx: button_idx, .. } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_button_up(button_idx);
                    }
                },
                _ => {}
            }
        }
        let keyboard = self.c64.get_keyboard();
        if keyboard.borrow().has_events() && self.c64.get_cycles() >= self.next_keyboard_event {
            keyboard.borrow_mut().drain_event();
            self.next_keyboard_event = self.c64.get_cycles().wrapping_add(20000);
        }
    }
}
