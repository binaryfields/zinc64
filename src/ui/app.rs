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

use sdl2;
use sdl2::{EventPump, Sdl};
use sdl2::audio::AudioDevice;
use sdl2::event::Event;
use sdl2::keyboard;
use sdl2::keyboard::Keycode;

use zinc64::system::C64;
use zinc64::util::Dimension;
use zinc64::video::vic;

use super::audio::AppAudio;
use super::io::Io;
use super::renderer::Renderer;

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

pub struct Options {
    pub fullscreen: bool,
    pub jam_action: JamAction,
    pub height: u32,
    pub width: u32,
}

#[derive(Debug, PartialEq)]
enum State {
    Running,
    Paused,
    Stopped,
}

pub struct App {
    // Dependencies
    c64: C64,
    options: Options,
    // Components
    sdl_context: Sdl,
    audio_device: AudioDevice<AppAudio>,
    io: Io,
    renderer: Renderer,
    // Runtime State
    state: State,
    next_keyboard_event: u32,
}

impl App {
    pub fn new(c64: C64, options: Options) -> Result<App, String> {
        let sdl_context = sdl2::init()?;
        // Initialize video
        let sdl_video = sdl_context.video()?;
        info!(target: "ui", "Opening app window {}x{}", options.width, options.height);
        let vic_spec = vic::Spec::new(c64.get_config().model.vic_model);
        let window_size = Dimension::new(options.width as u16, options.height as u16);
        let screen_size = vic_spec.display_rect.size();
        let renderer = Renderer::new(
            &sdl_video,
            window_size,
            screen_size,
            options.fullscreen
        )?;
        // Initialize audio
        let sdl_audio = sdl_context.audio()?;
        let audio_device = AppAudio::new_device(
            &sdl_audio,
            c64.get_config().sound.sample_rate as i32,
            1,
            c64.get_config().sound.buffer_size as u16,
            c64.get_sound_buffer()
        )?;
        // Initialize I/O
        let sdl_joystick = sdl_context.joystick()?;
        let io = Io::new(
            &sdl_joystick,
            c64.get_keyboard(),
            c64.get_joystick1(),
            c64.get_joystick2(),
        )?;
        let app = App {
            c64,
            options,
            sdl_context,
            audio_device,
            io,
            renderer,
            state: State::Running,
            next_keyboard_event: 0,
        };
        Ok(app)
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!(target: "ui", "Running main loop");
        self.audio_device.resume();
        let mut events = self.sdl_context.event_pump().unwrap();
        let mut overflow_cycles = 0i32;
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
                        {
                            let frame_buffer = rt.borrow();
                            self.renderer.render(&frame_buffer)?;
                        }
                        rt.borrow_mut().set_sync(false);
                    }
                }
                State::Paused => {
                    self.handle_events(&mut events);
                    let wait = Duration::from_millis(20);
                    thread::sleep(wait);
                }
                State::Stopped => {
                    info!(target: "ui", "State {:?}", self.state);
                    break 'running;
                }
            }
        }
        Ok(())
    }

    fn handle_cpu_jam(&mut self) -> bool {
        let cpu = self.c64.get_cpu();
        match self.options.jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
                self.state = State::Stopped;
                false
            }
            JamAction::Reset => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
                self.reset();
                false
            }
        }
    }

    fn reset(&mut self) {
        self.c64.reset(false);
        self.next_keyboard_event = 0;
    }

    fn toggle_datassette_play(&mut self) {
        let datassette = self.c64.get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_pause(&mut self) {
        let new_state = match self.state {
            State::Running => Some(State::Paused),
            State::Paused => Some(State::Running),
            _ => None
        };
        if let Some(state) = new_state {
            self.state = state;
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
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.state = State::Stopped;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_pause();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.state = State::Stopped;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_warp();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F9),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.reset();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F1),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LCTRLMOD) =>
                {
                    self.toggle_datassette_play();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.renderer.toggle_fullscreen();
                }
                _ => {
                    self.io.handle_event(&event);
                }
            }
        }
        let keyboard = self.c64.get_keyboard();
        if keyboard.borrow().has_events() && self.c64.get_cycles() >= self.next_keyboard_event {
            keyboard.borrow_mut().drain_event();
            self.next_keyboard_event = self.c64.get_cycles().wrapping_add(20000);
        }
    }
}
