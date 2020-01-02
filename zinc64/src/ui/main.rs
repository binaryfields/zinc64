// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::result::Result;

use sdl2;
use sdl2::audio::AudioDevice;
use sdl2::event::Event;
use sdl2::keyboard::{self, Keycode};
use sdl2::video;
use zinc64_loader::Loaders;

use crate::app::{App, AppState, JamAction, RuntimeState};
use crate::audio::AudioRenderer;
use crate::input::InputSystem;
use crate::ui::console::ConsoleScreen;
use crate::ui::{Screen2, Transition};
use crate::util::FileReader;
use crate::video::VideoRenderer;

pub struct MainScreen {
    // Components
    audio_device: AudioDevice<AudioRenderer>,
    input_system: InputSystem,
    video_renderer: VideoRenderer,
    // Runtime State
    #[allow(unused)]
    next_frame_ns: u64,
    next_keyboard_event: u64,
}

impl MainScreen {
    pub fn build(ctx: &mut AppState) -> Result<MainScreen, String> {
        // Initialize audio
        let audio_sys = ctx.platform.sdl.audio()?;
        let audio_device = AudioRenderer::build_device(
            &audio_sys,
            ctx.c64.get_config().sound.sample_rate as i32,
            1,
            ctx.c64.get_config().sound.buffer_size as u16,
            ctx.sound_buffer.clone(),
        )?;
        audio_device.resume();
        // Initialize video
        let video_renderer = VideoRenderer::build(
            &ctx.platform.window,
            ctx.c64.get_config().model.frame_buffer_size,
            ctx.c64.get_config().model.viewport_offset,
            ctx.c64.get_config().model.viewport_size,
            ctx.video_buffer.clone(),
        )?;
        // Initialize input
        let input_system = InputSystem::build()?;
        Ok(MainScreen {
            audio_device,
            input_system,
            video_renderer,
            next_frame_ns: 0,
            next_keyboard_event: 0,
        })
    }

    fn halt(&mut self, state: &mut AppState) -> Result<(), String> {
        self.set_state(state, RuntimeState::Halted);
        state.debug.halt()
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
        let jam_action = JamAction::Quit; // FIXME state.options.jam_action;
        match jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                warn!("CPU JAM detected at 0x{:x}", state.c64.get_cpu().get_pc());
                self.set_state(state, RuntimeState::Stopped);
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

    fn reset(&mut self, state: &mut AppState) {
        state.c64.reset(false);
        self.next_keyboard_event = 0;
    }

    fn set_state(&mut self, state: &mut AppState, new_state: RuntimeState) {
        if state.state != new_state {
            state.state = new_state;
            self.update_audio_state(state);
        }
    }

    /*
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
    */

    fn toggle_datassette_play(&mut self, state: &mut AppState) {
        let datassette = state.c64.get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_fullscreen(&mut self, state: &mut AppState) {
        let tmp = &mut state.platform.window;
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
            RuntimeState::Running => self.set_state(state, RuntimeState::Paused),
            RuntimeState::Paused => self.set_state(state, RuntimeState::Running),
            _ => (),
        };
    }

    fn toggle_warp(&mut self, ctx: &mut App, state: &mut AppState) {
        let value = state.options.warp_mode;
        state.options.warp_mode = !value;
        let fps = if !state.options.warp_mode {
            Some(state.c64.get_config().model.refresh_rate as f64)
        } else {
            None
        };
        ctx.time.set_fps(fps);
    }

    fn update_audio_state(&mut self, state: &mut AppState) {
        let emu_state = state.state;
        match emu_state {
            RuntimeState::Running => self.audio_device.resume(),
            RuntimeState::Paused => self.audio_device.pause(),
            RuntimeState::Halted => self.audio_device.pause(),
            RuntimeState::Stopped => self.audio_device.pause(),
        }
    }
}

impl Screen2<AppState> for MainScreen {
    fn handle_event(
        &mut self,
        ctx: &mut App,
        state: &mut AppState,
        event: Event,
    ) -> Result<Transition<AppState>, String> {
        let transition = match &event {
            Event::Quit { .. } => Ok(Transition::Pop),
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } if *keycode == Keycode::Escape => {
                let screen = ConsoleScreen::build(state)?;
                Ok(Transition::Push(Box::new(screen)))
            }
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
                        self.halt(state)?;
                    } else if *keycode == Keycode::M {
                        self.toggle_mute();
                    } else if *keycode == Keycode::P {
                        self.toggle_pause(state);
                    } else if *keycode == Keycode::Q {
                        self.set_state(state, RuntimeState::Stopped);
                    } else if *keycode == Keycode::W {
                        self.toggle_warp(ctx, state);
                    } else if *keycode == Keycode::Return {
                        self.toggle_fullscreen(state);
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
                Ok(Transition::None)
            }
            Event::DropFile { filename, .. } => {
                info!("Dropped file {}", filename);
                match self.load_image(state, &Path::new(&filename)) {
                    Ok(_) => (),
                    Err(err) => error!("Failed to load image, error: {}", err),
                }
                Ok(Transition::None)
            }
            _ => Ok(Transition::None),
        };
        self.input_system.handle_event(&mut state.c64, &event);
        transition
    }

    fn update(
        &mut self,
        _ctx: &mut App,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        self.process_keyboard_events(state);
        loop {
            let debugging = state.state == RuntimeState::Halted;
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
            RuntimeState::Running => {
                let vsync = state.c64.run_frame();
                if vsync {
                    // self.process_vsync(state)?;
                } else {
                    self.halt(state)?;
                }
                Ok(Transition::None)
            }
            RuntimeState::Paused => Ok(Transition::None),
            RuntimeState::Halted => {
                // self.handle_commands(true);
                // self.process_vsync(state)?;
                Ok(Transition::None)
            }
            RuntimeState::Stopped => Ok(Transition::Pop),
        }
    }

    fn draw(
        &mut self,
        _ctx: &mut App,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        if state.c64.get_vsync() {
            self.video_renderer.render(&mut state.platform.window)?;
            state.c64.reset_vsync();
        } else {
            if !state.options.warp_mode {
                // std::thread::sleep(Duration::from_millis(1));
            }
        }
        Ok(Transition::None)
    }
}
