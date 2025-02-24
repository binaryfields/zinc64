// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::cast_lossless)]

use std::result::Result;

use glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glutin::window::Fullscreen;

use crate::app::AppState;
use crate::audio::AudioRenderer;
use crate::framework::Context;
use crate::input::InputSystem;
use crate::session::RunState;
use crate::ui::console::ConsoleScreen;
use crate::ui::{Screen, Transition};
use crate::video::VideoRenderer;

pub struct MainScreen {
    // Components
    audio_device: Option<AudioRenderer>,
    input_system: InputSystem,
    video_renderer: VideoRenderer,
    // Runtime State
    audio_state: RunState,
    presented_frame: Option<u32>,
}

impl MainScreen {
    pub fn build(ctx: &mut Context, state: &mut AppState) -> Result<MainScreen, String> {
        // Initialize audio
        let audio_device = if state.settings.audio {
            let mut audio_device = AudioRenderer::build(
                state.session.c64().get_config().sound.sample_rate as i32,
                1,
                state.settings.audio_buffer as u16,
                state.sound_buffer.clone(),
            )
            .map_err(|err| format!("{}", err))?;
            audio_device.start();
            Some(audio_device)
        } else {
            None
        };
        // Initialize video
        let video_renderer = VideoRenderer::build(ctx, state)?;
        // Initialize input
        let input_system =
            InputSystem::build([state.settings.joystick_1, state.settings.joystick_2])?;
        Ok(MainScreen {
            audio_device,
            input_system,
            video_renderer,
            audio_state: RunState::Running,
            presented_frame: None,
        })
    }

    // ---- hotkey toggles ----

    fn toggle_mute(&mut self) {
        if let Some(audio_device) = &mut self.audio_device {
            audio_device.toggle_mute();
        }
    }

    fn toggle_warp(&mut self, ctx: &mut Context, state: &mut AppState) {
        state.settings.warp_mode = !state.settings.warp_mode;
        let refresh_rate = state.session.c64().get_config().model.refresh_rate;
        ctx.time.set_fps(state.settings.frame_rate(refresh_rate));
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

    // ---- audio ----

    fn follow_audio(&mut self, state: RunState) {
        if self.audio_state == state {
            return;
        }
        if let Some(audio_device) = &mut self.audio_device {
            match state {
                RunState::Running => audio_device.play(),
                RunState::Paused | RunState::Halted | RunState::Stopped => audio_device.pause(),
            }
        }
        self.audio_state = state;
    }
}

impl Screen<AppState> for MainScreen {
    // TODO: track WindowEvent::ModifiersChanged instead of KeyboardInput::modifiers
    #[allow(deprecated)]
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
                        app_state.session.halt()?;
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::M, ElementState::Pressed) if modifiers.alt() => {
                        self.toggle_mute();
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::P, ElementState::Pressed) if modifiers.alt() => {
                        app_state.session.toggle_pause();
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::Q, ElementState::Pressed) if modifiers.alt() => {
                        app_state.session.quit();
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
                        app_state.session.toggle_tape();
                        Ok(Transition::None)
                    }
                    (VirtualKeyCode::F9, ElementState::Pressed) if modifiers.ctrl() => {
                        app_state.session.reset(false);
                        Ok(Transition::None)
                    }
                    _ => Ok(Transition::None),
                },
                WindowEvent::DroppedFile(path) => {
                    if let Err(err) = app_state.session.open_image(path) {
                        error!("Failed to load image, error: {}", err);
                    }
                    Ok(Transition::None)
                }
                _ => Ok(Transition::None),
            },
            _ => Ok(Transition::None),
        };
        self.input_system
            .handle_event(app_state.session.c64_mut(), &event);
        transition
    }

    fn update(
        &mut self,
        _ctx: &mut Context,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        if state.session.update()? {
            state.sound_buffer.push(state.session.c64().samples());
        }
        self.follow_audio(state.session.state());
        match state.session.state() {
            RunState::Stopped => Ok(Transition::Pop),
            _ => Ok(Transition::None),
        }
    }

    fn draw(
        &mut self,
        ctx: &mut Context,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        let frame = state.session.c64().get_frame_count();
        if self.presented_frame != Some(frame) {
            self.video_renderer
                .render(ctx, state.session.c64().frame())?;
            self.presented_frame = Some(frame);
            ctx.platform
                .windowed_context
                .swap_buffers()
                .map_err(|_| "failed to swap buffers")?;
        }
        Ok(Transition::None)
    }
}
