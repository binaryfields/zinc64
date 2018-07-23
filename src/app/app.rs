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

use std::net::SocketAddr;
use std::result::Result;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sdl2;
use sdl2::audio::AudioDevice;
use sdl2::event::Event;
use sdl2::keyboard;
use sdl2::keyboard::Keycode;
use sdl2::{EventPump, Sdl};
use time;
use zinc64::system::C64;

use super::audio::AppAudio;
use super::command::Command;
use super::debugger::Debugger;
use super::execution::{ExecutionEngine, State};
use super::io::Io;
use super::rap_server::RapServer;
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
    pub window_size: (u32, u32),
    pub speed: u8,
    pub warp_mode: bool,
    // Debug
    pub debug: bool,
    pub dbg_address: Option<SocketAddr>,
    pub jam_action: JamAction,
    pub rap_address: Option<SocketAddr>,
}

pub struct App {
    // Dependencies
    options: Options,
    // Components
    audio_device: AudioDevice<AppAudio>,
    command_rx: mpsc::Receiver<Command>,
    execution_engine: ExecutionEngine,
    io: Io,
    renderer: Renderer,
    sdl_context: Sdl,
    // Runtime State
    next_frame_ns: u64,
    next_keyboard_event: u64,
}

impl App {
    pub fn new(c64: C64, options: Options) -> Result<App, String> {
        let sdl_context = sdl2::init()?;
        // Initialize video
        let sdl_video = sdl_context.video()?;
        info!(target: "ui", "Opening app window {}x{}", options.window_size.0, options.window_size.1);
        let renderer = Renderer::new(
            &sdl_video,
            options.window_size,
            c64.get_config().model.frame_buffer_size,
            c64.get_config().model.viewport_offset,
            c64.get_config().model.viewport_size,
            options.fullscreen,
        )?;
        // Initialize audio
        let sdl_audio = sdl_context.audio()?;
        let mut audio_device = AppAudio::new_device(
            &sdl_audio,
            c64.get_config().sound.sample_rate as i32,
            1,
            c64.get_config().sound.buffer_size as u16,
            c64.get_sound_buffer(),
        )?;
        audio_device.lock().set_volume(100);
        // Initialize I/O
        let sdl_joystick = sdl_context.joystick()?;
        let io = Io::new(
            &sdl_joystick,
            c64.get_keyboard(),
            c64.get_joystick1(),
            c64.get_joystick2(),
        )?;
        // Initialize debuggers
        let (command_tx, command_rx) = mpsc::channel::<Command>();
        if options.debug {
            let address = options
                .dbg_address
                .unwrap_or(SocketAddr::from(([127, 0, 0, 1], 9999)));
            info!(target: "ui", "Starting debugger at {}", address);
            let command_tx_clone = command_tx.clone();
            thread::spawn(move || {
                let debugger = Debugger::new(command_tx_clone);
                debugger.start(address);
            });
        }
        if let Some(address) = options.rap_address {
            info!(target: "ui", "Starting rap server at {}", address);
            let command_tx_clone = command_tx.clone();
            thread::spawn(move || {
                let rap_server = RapServer::new(command_tx_clone);
                rap_server.start(address);
            });
        }
        let app = App {
            options,
            audio_device,
            command_rx,
            execution_engine: ExecutionEngine::new(c64),
            io,
            renderer,
            sdl_context,
            next_frame_ns: 0,
            next_keyboard_event: 0,
        };
        Ok(app)
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!(target: "ui", "Running main loop");
        let mut events = self.sdl_context.event_pump().unwrap();
        'running: loop {
            match self.execution_engine.get_state() {
                State::Starting => {
                    self.set_state(State::Running);
                }
                State::Running => {
                    let vsync = self.execution_engine.get_c64_mut().run_frame();
                    if vsync {
                        self.process_vsync();
                    } else {
                        self.execution_engine.halt();
                    }
                }
                State::Paused => {
                    self.process_vsync();
                    thread::sleep(Duration::from_millis(20));
                }
                State::Halted => {
                    self.handle_commands(true);
                    self.process_vsync();
                    thread::sleep(Duration::from_millis(20));
                }
                State::Stopped => {
                    info!(target: "ui", "State {:?}", self.execution_engine.get_state());
                    break 'running;
                }
            }
            self.handle_events(&mut events);
            self.handle_commands(false);
        }
        Ok(())
    }

    fn process_vsync(&mut self) {
        let rt = self.execution_engine.get_c64().get_frame_buffer();
        if rt.borrow().get_sync() {
            if !self.options.warp_mode {
                self.sync_frame();
            }
            {
                let frame_buffer = rt.borrow();
                self.renderer.render(&frame_buffer);
            }
            rt.borrow_mut().set_sync(false);
        }
    }

    pub fn sync_frame(&mut self) {
        let refresh_rate = self.execution_engine
            .get_c64()
            .get_config()
            .model
            .refresh_rate;
        let frame_duration_ns = (1_000_000_000.0 / refresh_rate) as u32;
        let frame_duration_scaled_ns = frame_duration_ns * 100 / self.options.speed as u32;
        let time_ns = time::precise_time_ns();
        let wait_ns = if self.next_frame_ns > time_ns {
            (self.next_frame_ns - time_ns) as u32
        } else {
            0
        };
        if wait_ns > 0 && wait_ns <= frame_duration_scaled_ns {
            thread::sleep(Duration::new(0, wait_ns));
        }
        self.next_frame_ns = time::precise_time_ns() + frame_duration_scaled_ns as u64;
    }

    #[allow(dead_code)]
    fn handle_cpu_jam(&mut self) -> bool {
        match self.options.jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", self.execution_engine.get_c64().get_cpu().get_pc());
                self.set_state(State::Stopped);
                false
            }
            JamAction::Reset => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", self.execution_engine.get_c64().get_cpu().get_pc());
                self.reset();
                false
            }
        }
    }

    fn reset(&mut self) {
        self.execution_engine.execute(&Command::SysReset(false)); // FIXME
        self.next_keyboard_event = 0;
    }

    fn set_state(&mut self, new_state: State) {
        if self.execution_engine.get_state() != new_state {
            self.execution_engine.set_state(new_state);
            self.update_audio_state();
        }
    }

    fn toggle_datassette_play(&mut self) {
        let datassette = self.execution_engine.get_c64().get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_mute(&mut self) {
        self.audio_device.lock().toggle_mute();
    }

    fn toggle_pause(&mut self) {
        match self.execution_engine.get_state() {
            State::Running => self.set_state(State::Paused),
            State::Paused => self.set_state(State::Running),
            _ => (),
        };
    }

    fn toggle_warp(&mut self) {
        let warp_mode = self.options.warp_mode;
        self.options.warp_mode = !warp_mode;
    }

    fn update_audio_state(&mut self) {
        match self.execution_engine.get_state() {
            State::Running => self.audio_device.resume(),
            State::Paused => self.audio_device.pause(),
            State::Halted => self.audio_device.pause(),
            State::Stopped => self.audio_device.pause(),
            _ => (),
        }
    }

    // -- Event Handling

    fn handle_commands(&mut self, debugging: bool) {
        if !debugging {
            match self.command_rx.try_recv() {
                Ok(command) => self.execution_engine.execute(&command),
                _ => Ok(()),
            };
        } else {
            let mut done = false;
            while !done {
                match self.command_rx.recv_timeout(Duration::from_millis(1)) {
                    Ok(command) => {
                        self.execution_engine.execute(&command);
                    }
                    _ => {
                        done = true;
                    }
                }
            }
        }
    }

    fn handle_events(&mut self, events: &mut EventPump) {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.set_state(State::Stopped);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::H),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.execution_engine.halt();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_mute();
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
                    self.set_state(State::Stopped);
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
        let keyboard = self.execution_engine.get_c64().get_keyboard();
        if keyboard.borrow().has_events()
            && self.execution_engine.get_c64().get_cycles() >= self.next_keyboard_event
        {
            keyboard.borrow_mut().drain_event();
            self.next_keyboard_event = self.execution_engine
                .get_c64()
                .get_cycles()
                .wrapping_add(20000);
        }
    }
}
