// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::result::Result;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use sdl2::audio::AudioDevice;
use sdl2::event::Event;
use sdl2::keyboard::{self, Keycode};
use sdl2::render::Canvas;
use sdl2::video::{self, Window};
use sdl2::{self, EventPump, Sdl};
use time;
use zinc64_core::Shared;
use zinc64_emu::system::C64;
use zinc64_loader::Loaders;

use crate::app::{AppState, JamAction, State};
use crate::audio::AudioRenderer;
use crate::debug::Debug;
use crate::input::InputSystem;
use crate::sound_buffer::SoundBuffer;
use crate::ui::{Action, Screen};
use crate::util::FileReader;
use crate::video_buffer::VideoBuffer;
use crate::video_renderer::VideoRenderer;

pub struct MainScreen {
    // Dependencies
    window: Shared<Canvas<Window>>,
    // Components
    audio_device: AudioDevice<AudioRenderer>,
    input_system: InputSystem,
    video_renderer: VideoRenderer,
    // Runtime State
    next_frame_ns: u64,
    next_keyboard_event: u64,
}

impl MainScreen {
    pub fn build(
        sdl_context: &Sdl,
        c64: &C64,
        window: Shared<Canvas<Window>>,
        sound_buffer: Arc<SoundBuffer>,
        video_buffer: Shared<VideoBuffer>,
    ) -> Result<MainScreen, String> {
        // Initialize audio
        let sdl_audio = sdl_context.audio()?;
        let audio_device = AudioRenderer::new_device(
            &sdl_audio,
            c64.get_config().sound.sample_rate as i32,
            1,
            c64.get_config().sound.buffer_size as u16,
            sound_buffer.clone(),
        )?;
        // Initialize video
        let video_renderer = VideoRenderer::build(
            &window.borrow(),
            c64.get_config().model.frame_buffer_size,
            c64.get_config().model.viewport_offset,
            c64.get_config().model.viewport_size,
            video_buffer.clone(),
        )?;
        // Initialize input
        let input_system = InputSystem::build()?;
        Ok(MainScreen {
            window,
            audio_device,
            input_system,
            video_renderer,
            next_frame_ns: 0,
            next_keyboard_event: 0,
        })
    }

    fn handle_event(&mut self, event: &Event, state: &mut AppState) -> Option<Action> {
        match event {
            Event::Quit { .. } => Some(Action::Exit),
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } if *keycode == Keycode::Escape => Some(Action::Console),
            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                repeat: false,
                ..
            } => {
                if keymod.contains(keyboard::Mod::LALTMOD)
                    || keymod.contains(keyboard::Mod::RALTMOD)
                {
                    if *keycode == Keycode::H {
                        self.halt(state);
                    } else if *keycode == Keycode::M {
                        self.toggle_mute();
                    } else if *keycode == Keycode::P {
                        self.toggle_pause(state);
                    } else if *keycode == Keycode::Q {
                        self.set_state(state, State::Stopped);
                    } else if *keycode == Keycode::W {
                        self.toggle_warp(state);
                    } else if *keycode == Keycode::Return {
                        self.toggle_fullscreen();
                    }
                } else if keymod.contains(keyboard::Mod::LCTRLMOD)
                    || keymod.contains(keyboard::Mod::RCTRLMOD)
                {
                    if *keycode == Keycode::F1 {
                        self.toggle_datassette_play(state);
                    } else if *keycode == Keycode::F9 {
                        self.reset(state);
                    }
                }
                None
            }
            Event::DropFile { filename, .. } => {
                info!("Dropped file {}", filename);
                match self.load_image(state, &Path::new(&filename)) {
                    Ok(_) => (),
                    Err(err) => error!("Failed to load image, error: {}", err),
                }
                None
            }
            _ => None,
        }
    }

    fn halt(&mut self, state: &mut AppState) {
        self.set_state(state, State::Halted);
        state.debug.halt();
    }

    fn load_image(&mut self, state: &mut AppState, path: &Path) -> Result<(), String> {
        let ext = path.extension().map(|s| s.to_str().unwrap());
        let loader = Loaders::from_ext(ext)?;
        let file = File::open(path).map_err(|err| format!("{}", err))?;
        let mut reader = FileReader(BufReader::new(file));
        let mut autostart = loader.autostart(&mut reader)?;
        autostart.execute(&mut state.c64);
        Ok(())
    }

    #[allow(dead_code)]
    fn process_cpu_jam(&mut self, state: &mut AppState) -> bool {
        let jam_action = state.options.jam_action;
        match jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                warn!("CPU JAM detected at 0x{:x}", state.c64.get_cpu().get_pc());
                self.set_state(state, State::Stopped);
                false
            }
            JamAction::Reset => {
                warn!("CPU JAM detected at 0x{:x}", state.c64.get_cpu().get_pc());
                self.reset(state);
                false
            }
        }
    }

    fn process_keyboard_events(&mut self, state: &mut AppState) {
        if state.c64.get_keyboard().has_events()
            && state.c64.get_cycles() >= self.next_keyboard_event
        {
            state.c64.get_keyboard().drain_event();
            self.next_keyboard_event = state.c64.get_cycles().wrapping_add(20000);
        }
    }

    fn process_vsync(&mut self, state: &mut AppState) -> Result<(), String> {
        if state.c64.get_vsync() {
            if !state.options.warp_mode {
                self.sync_frame(state);
            }
            self.video_renderer.render(&mut self.window.borrow_mut())?;
            state.c64.reset_vsync();
        }
        Ok(())
    }

    fn reset(&mut self, state: &mut AppState) {
        state.c64.reset(false);
        self.next_keyboard_event = 0;
    }

    fn set_state(&mut self, state: &mut AppState, new_state: State) {
        if state.state != new_state {
            state.state = new_state;
            self.update_audio_state(state);
        }
    }

    fn sync_frame(&mut self, state: &mut AppState) {
        let refresh_rate = state.c64.get_config().model.refresh_rate;
        let frame_duration_ns = (1_000_000_000.0 / refresh_rate) as u32;
        let frame_duration_scaled_ns = frame_duration_ns * 100 / state.options.speed as u32;
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

    fn toggle_datassette_play(&mut self, state: &mut AppState) {
        let datassette = state.c64.get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_fullscreen(&mut self) {
        let tmp = &mut self.window.borrow_mut();
        let window = tmp.window_mut();
        match window.fullscreen_state() {
            video::FullscreenType::Off => {
                window.set_fullscreen(video::FullscreenType::True).unwrap();
            }
            video::FullscreenType::True | video::FullscreenType::Desktop => {
                window.set_fullscreen(video::FullscreenType::Off).unwrap();
            }
        }
    }

    fn toggle_mute(&mut self) {
        self.audio_device.lock().toggle_mute();
    }

    fn toggle_pause(&mut self, state: &mut AppState) {
        let emu_state = state.state;
        match emu_state {
            State::Running => self.set_state(state, State::Paused),
            State::Paused => self.set_state(state, State::Running),
            _ => (),
        };
    }

    fn toggle_warp(&mut self, state: &mut AppState) {
        let value = state.options.warp_mode;
        state.options.warp_mode = !value;
    }

    fn update_audio_state(&mut self, state: &mut AppState) {
        let emu_state = state.state;
        match emu_state {
            State::Running => self.audio_device.resume(),
            State::Paused => self.audio_device.pause(),
            State::Halted => self.audio_device.pause(),
            State::Stopped => self.audio_device.pause(),
            _ => (),
        }
    }
}

impl Screen<AppState> for MainScreen {
    fn run(&mut self, events: &mut EventPump, state: &mut AppState) -> Result<Action, String> {
        // handle event
        // update
        // render
        // sleep
        'running: loop {
            for event in events.poll_iter() {
                if let Some(action) = self.handle_event(&event, state) {
                    return Ok(action);
                }
                self.input_system.handle_event(&mut state.c64, &event);
            }
            self.process_keyboard_events(state);
            loop {
                let debugging = state.state == State::Halted;
                let command_maybe = state.debug.poll(debugging);
                if let Some(command) = command_maybe {
                    let result = state.debug.execute(&mut state.c64, &command);
                    match result {
                        Ok(Some(new_state)) => self.set_state(state, new_state),
                        _ => (),
                    }
                } else {
                    break;
                }
            }
            match state.state {
                State::New => {
                    self.set_state(state, State::Running);
                }
                State::Running => {
                    let vsync = state.c64.run_frame();
                    if vsync {
                        self.process_vsync(state)?;
                    } else {
                        self.halt(state);
                    }
                }
                State::Paused => {
                    thread::sleep(Duration::from_millis(20));
                }
                State::Halted => {
                    // self.handle_commands(true);
                    self.process_vsync(state)?;
                    thread::sleep(Duration::from_millis(20));
                }
                State::Stopped => {
                    return Ok(Action::Exit);
                }
            }
        }
    }
}
