// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::path::Path;
use std::sync::Arc;

use glutin::event::Event;

use crate::audio::SoundBuffer;
use crate::console::Console;
use crate::framework::gfx::Font;
use crate::framework::{Context, State};
use crate::session::Session;
use crate::settings::Settings;
use crate::ui::{MainScreen, Screen, Transition};

const CONSOLE_BUFFER: usize = 2048;

pub struct AppState {
    pub session: Session,
    pub console: Console,
    pub console_history: Vec<String>,
    pub settings: Settings,
    pub sound_buffer: Arc<SoundBuffer>,
}

pub struct App {
    state: AppState,
    screens: Vec<Box<dyn Screen<AppState>>>,
}

impl App {
    pub fn build(
        ctx: &mut Context,
        session: Session,
        sound_buffer: Arc<SoundBuffer>,
        settings: Settings,
    ) -> Result<App, String> {
        let window_size = ctx.platform.windowed_context.window().inner_size();
        // Initialize fps
        let refresh_rate = session.c64().get_config().model.refresh_rate;
        ctx.time.set_fps(settings.frame_rate(refresh_rate));
        // Initiliaze console
        let font = Font::load_psf(Path::new("res/font/font.psf"))?;
        let cols = window_size.width / font.get_width();
        let rows = window_size.height / font.get_height();
        let mut console = Console::new(cols, rows, CONSOLE_BUFFER);
        console.print("Type ? for the list of available commands\n".as_bytes());
        console.save_pos();
        // Initialize state
        let mut state = AppState {
            session,
            console,
            console_history: Vec::new(),
            settings,
            sound_buffer,
        };
        let main_screen = MainScreen::build(ctx, &mut state)?;
        let screens: Vec<Box<dyn Screen<AppState>>> = vec![Box::new(main_screen)];
        Ok(App { state, screens })
    }

    fn process_transition(&mut self, transition: Transition<AppState>) {
        match transition {
            Transition::None => {}
            Transition::Push(next) => {
                self.screens.push(next);
            }
            Transition::Pop => {
                self.screens.pop();
            }
        }
    }
}

impl State for App {
    fn handle_event(&mut self, ctx: &mut Context, event: Event<()>) -> Result<(), String> {
        match self.screens.last_mut() {
            Some(screen) => {
                let transition = screen.handle_event(ctx, &mut self.state, event)?;
                self.process_transition(transition);
            }
            None => {
                ctx.running = false;
            }
        }
        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> Result<(), String> {
        match self.screens.last_mut() {
            Some(screen) => {
                let transition = screen.update(ctx, &mut self.state)?;
                self.process_transition(transition);
            }
            None => {
                ctx.running = false;
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), String> {
        match self.screens.last_mut() {
            Some(screen) => {
                let transition = screen.draw(ctx, &mut self.state)?;
                self.process_transition(transition);
            }
            None => {
                ctx.running = false;
            }
        }
        Ok(())
    }
}
