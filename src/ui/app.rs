/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
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

#[derive(Debug, PartialEq)]
enum State {
    Running,
    Paused,
    Stopped,
    Trapped,
}

pub struct Options {
    pub fullscreen: bool,
    pub height: u32,
    pub width: u32,
}

pub struct AppWindow {
    // Dependencies
    c64: C64,
    // Renderer
    sdl: Sdl,
    renderer: Renderer<'static>,
    texture: Texture,
    // Devices
    joystick1: Option<Joystick>,
    joystick2: Option<Joystick>,
    // Runtime State
    state: State,
    last_frame_ts: u64,
    next_keyboard_event: u32,
    warp_mode: bool,
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
                state: State::Running,
                last_frame_ts: 0,
                next_keyboard_event: 0,
                warp_mode: false,
            }
        )
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

    pub fn run(&mut self) {
        info!(target: "ui", "Running app main loop");
        let mut events = self.sdl.event_pump().unwrap();
        'running: loop {
            match self.state {
                State::Running => {
                    self.handle_events(&mut events);
                    self.run_frame();
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

    fn run_frame(&mut self) {
        let frame_cycles = (self.c64.get_config().cpu_frequency as f64
            / self.c64.get_config().refresh_rate) as u64;
        let rt = self.c64.get_render_target();
        let mut last_pc = 0x0000;
        for i in 0..frame_cycles {
            self.c64.step();
            if rt.borrow().get_sync() {
                if !self.warp_mode {
                    self.wait_vsync();
                }
                self.render();
            }
            if self.c64.check_breakpoints() {
                self.state = State::Trapped;
                break;
            }
            let cpu = self.c64.get_cpu();
            let pc = cpu.borrow().get_pc();
            if pc == last_pc {
                self.state = State::Trapped;
                break;
            }
            last_pc = pc;
        }
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
        self.warp_mode = !self.warp_mode;
    }

    fn wait_vsync(&self) {
        let elapsed_ns = time::precise_time_ns() - self.last_frame_ts;
        if elapsed_ns < self.c64.get_config().refrest_rate_ns {
            let wait_ns = self.c64.get_config().refrest_rate_ns - elapsed_ns;
            let wait = Duration::from_millis(wait_ns / 1_000_000);
            thread::sleep(wait);
        }
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
