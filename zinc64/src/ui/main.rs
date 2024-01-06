// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::result::Result;

use glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glutin::window::Fullscreen;
use zinc64_loader::Loaders;

use crate::app::{AppState, JamAction, RuntimeState};
use crate::audio::AudioRenderer;
use crate::framework::Context;
use crate::input::InputSystem;
use crate::ui::console::ConsoleScreen;
use crate::ui::{Screen, Transition};
use crate::util::FileReader;
use crate::video::VideoRenderer;

pub struct MainScreen {
    // Components
    audio_device: AudioRenderer,
    input_system: InputSystem,
    video_renderer: VideoRenderer,
    // Runtime State
    next_keyboard_event: u64,
}

impl MainScreen {
    pub fn build(ctx: &mut Context, state: &mut AppState) -> Result<MainScreen, String> {
        // Initialize audio
        let mut audio_device = AudioRenderer::build(
            //&audio_sys,
            state.c64.get_config().sound.sample_rate as i32,
            1,
            state.c64.get_config().sound.buffer_size as u16,
            state.sound_buffer.clone(),
        )
        .map_err(|err| format!("{}", err))?;
        audio_device.start();
        // Initialize video
        let video_renderer = VideoRenderer::build(ctx, state)?;
        // Initialize input
        let input_system = InputSystem::build()?;
        Ok(MainScreen {
            audio_device,
            input_system,
            video_renderer,
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

    fn toggle_datassette_play(&mut self, state: &mut AppState) {
        let datassette = state.c64.get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_fullscreen(&mut self, ctx: &mut Context) {
        match ctx.platform.windowed_context.window().fullscreen() {
            None => {
                let monitor = ctx.platform.windowed_context.window().current_monitor();
                ctx.platform
                    .windowed_context
                    .window()
                    .set_fullscreen(Some(Fullscreen::Borderless(monitor)));
            }
            Some(_) => {
                ctx.platform.windowed_context.window().set_fullscreen(None);
            }
        }
    }

    fn toggle_mute(&mut self) {
        self.audio_device.toggle_mute();
    }

    fn toggle_pause(&mut self, state: &mut AppState) {
        let emu_state = state.state;
        match emu_state {
            RuntimeState::Running => self.set_state(state, RuntimeState::Paused),
            RuntimeState::Paused => self.set_state(state, RuntimeState::Running),
            _ => (),
        };
    }

    fn toggle_warp(&mut self, ctx: &mut Context, state: &mut AppState) {
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
            RuntimeState::Running => self.audio_device.play(),
            RuntimeState::Paused => self.audio_device.pause(),
            RuntimeState::Halted => self.audio_device.pause(),
            RuntimeState::Stopped => self.audio_device.pause(),
        }
    }
}

impl Screen<AppState> for MainScreen {
    fn handle_event(
        &mut self,
        ctx: &mut Context,
        state: &mut AppState,
        event: Event<()>,
    ) -> Result<Transition<AppState>, String> {
        let app_state = state;
        let transition = match &event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    self.video_renderer
                        .update_viewport(ctx, size.width as i32, size.height as i32);
                    Ok(Transition::None)
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            modifiers,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Escape, ElementState::Pressed) => {
                        let screen = ConsoleScreen::build(ctx, app_state)?;
                        Ok(Transition::Push(Box::new(screen)))
                    }
                    (VirtualKeyCode::H, ElementState::Pressed) if modifiers.alt() => {
                        self.halt(app_state)?;
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::M, ElementState::Pressed) if modifiers.alt() => {
                        self.toggle_mute();
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::P, ElementState::Pressed) if modifiers.alt() => {
                        self.toggle_pause(app_state);
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::Q, ElementState::Pressed) if modifiers.alt() => {
                        self.set_state(app_state, RuntimeState::Stopped);
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::W, ElementState::Pressed) if modifiers.alt() => {
                        self.toggle_warp(ctx, app_state);
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::Return, ElementState::Pressed) if modifiers.alt() => {
                        self.toggle_fullscreen(ctx);
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::F1, ElementState::Pressed) if modifiers.ctrl() => {
                        self.toggle_datassette_play(app_state);
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::F9, ElementState::Pressed) if modifiers.ctrl() => {
                        self.reset(app_state);
                        Ok(Transition::None)
                    }
                    _ => Ok(Transition::None),
                },
                WindowEvent::DroppedFile(path) => {
                    // info!("Dropped file {}", path);
                    match self.load_image(app_state, &path) {
                        Ok(_) => (),
                        Err(err) => error!("Failed to load image, error: {}", err),
                    }
                    Ok(Transition::None)
                }
                _ => Ok(Transition::None),
            },
            _ => Ok(Transition::None),
        };
        self.input_system.handle_event(&mut app_state.c64, &event);
        transition
    }

    fn update(
        &mut self,
        _ctx: &mut Context,
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
                if !vsync {
                    self.halt(state)?;
                }
                Ok(Transition::None)
            }
            RuntimeState::Paused => Ok(Transition::None),
            RuntimeState::Halted => Ok(Transition::None),
            RuntimeState::Stopped => Ok(Transition::Pop),
        }
    }

    fn draw(
        &mut self,
        ctx: &mut Context,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        if state.c64.get_vsync() {
            self.video_renderer.render(ctx)?;
            state.c64.reset_vsync();
            ctx.platform
                .windowed_context
                .swap_buffers()
                .map_err(|_| "failed to swap buffers")?;
        }
        Ok(Transition::None)
    }
}
