// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;

use glutin::event::Event;
use zinc64_core::util::Shared;
use zinc64_debug::{Command, Debugger};
use zinc64_core::device::joystick;
use zinc64_system::C64;

use crate::audio::SoundBuffer;
use crate::console::Console;
use crate::debug::Debug;
use crate::framework::{Context, State};
use crate::gfx::Font;
use crate::ui::{MainScreen, Screen, Transition};
use crate::video::VideoBuffer;

const CONSOLE_BUFFER: usize = 2048;

#[derive(Copy, Clone, Debug)]
pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

pub struct Options {
    // Emulator
    pub jam_action: JamAction,
    pub speed: u8,
    pub warp_mode: bool,
    // Controllers
    pub joydev_1: joystick::Mode,
    pub joydev_2: joystick::Mode,
    // Debug
    pub debug: bool,
    pub dbg_address: SocketAddr,
    pub rap_address: SocketAddr,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RuntimeState {
    Running,
    Paused,
    Halted,
    Stopped,
}

pub struct AppState {
    pub state: RuntimeState,
    pub c64: C64,
    pub console: Console,
    pub console_history: Vec<String>,
    pub debug: Debug,
    pub options: Options,
    pub sound_buffer: Arc<SoundBuffer>,
    pub video_buffer: Shared<VideoBuffer>,
}

pub struct App {
    state: AppState,
    screens: Vec<Box<dyn Screen<AppState>>>,
}

impl App {
    pub fn build(
        ctx: &mut Context,
        c64: C64,
        sound_buffer: Arc<SoundBuffer>,
        video_buffer: Shared<VideoBuffer>,
        options: Options,
    ) -> Result<App, String> {
        let window_size = ctx.platform.windowed_context.window().inner_size();
        // Initialize fps
        let fps = if !options.warp_mode {
            Some(c64.get_config().model.refresh_rate as f64)
        } else {
            None
        };
        ctx.time.set_fps(fps);
        // Initiliaze console
        let font = Font::load_psf(Path::new("res/font/font.psf"))?;
        let cols = window_size.width / font.get_width();
        let rows = window_size.height / font.get_height();
        let mut console = Console::new(cols, rows, CONSOLE_BUFFER);
        console.print("Type ? for the list of available commands\n".as_bytes());
        console.save_pos();
        // Initialize debuggers
        let (debug_tx, debug_rx) = mpsc::channel::<Command>();
        if options.debug {
            let address = options.dbg_address;
            info!("Starting debugger at {}", address);
            let debug_tx_clone = debug_tx.clone();
            thread::spawn(move || {
                let debugger = Debugger::new(debug_tx_clone);
                debugger.start(address).expect("Failed to start debugger");
            });
        }
        // Initialize state
        let mut state = AppState {
            state: RuntimeState::Running,
            c64,
            console,
            console_history: Vec::new(),
            debug: Debug::new(debug_rx),
            options,
            sound_buffer,
            video_buffer,
        };
        let main_screen = MainScreen::build(ctx, &mut state)?;
        let mut screens: Vec<Box<dyn Screen<AppState>>> = Vec::new();
        screens.push(Box::new(main_screen));
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
