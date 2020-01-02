// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::thread;

use sdl2::event::Event;
use zinc64_core::Shared;
use zinc64_debug::{Command, Debugger};
use zinc64_emu::device::joystick;
use zinc64_emu::system::C64;

use crate::audio::SoundBuffer;
use crate::console::Console;
use crate::debug::Debug;
use crate::gfx::Font;
use crate::platform::Platform;
use crate::time::Time;
use crate::ui::{MainScreen, Screen2, Transition};
use crate::video::VideoBuffer;

const APP_NAME: &'static str = "zinc64";
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
    // Window
    pub fullscreen: bool,
    pub window_size: (u32, u32),
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
    pub platform: Platform,
    pub options: Options,
    pub sound_buffer: Arc<SoundBuffer>,
    pub video_buffer: Shared<VideoBuffer>,
}

pub trait Controller {
    fn handle_events(&mut self, ctx: &mut App) -> Result<(), String>;

    fn update(&mut self, ctx: &mut App) -> Result<(), String>;

    fn draw(&mut self, ctx: &mut App) -> Result<(), String>;
}

pub struct App {
    running: bool,
    pub time: Time,
}

impl App {
    pub fn new(time: Time) -> Self {
        Self {
            running: false,
            time,
        }
    }

    pub fn run<C, I>(&mut self, init: I) -> Result<(), String>
    where
        C: Controller,
        I: FnOnce(&mut App) -> Result<C, String>,
    {
        let mut controller = init(self)?;
        self.running = true;
        while self.running {
            if let Err(e) = self.tick(&mut controller) {
                self.running = false;
                return Err(e);
            }
        }
        Ok(())
    }

    fn tick<C>(&mut self, controller: &mut C) -> Result<(), String>
    where
        C: Controller,
    {
        self.time.tick();
        controller.handle_events(self)?;
        if self.time.has_timer_event() {
            controller.update(self)?;
        }
        controller.draw(self)?;
        std::thread::yield_now();
        Ok(())
    }
}

pub struct AppController {
    state: AppState,
    screens: Vec<Box<dyn Screen2<AppState>>>,
}

impl AppController {
    pub fn build(
        ctx: &mut App,
        c64: C64,
        sound_buffer: Arc<SoundBuffer>,
        video_buffer: Shared<VideoBuffer>,
        options: Options,
    ) -> Result<AppController, String> {
        let platform = Platform::build(APP_NAME, &options)?;
        // Initialize fps
        let fps = if !options.warp_mode {
            Some(c64.get_config().model.refresh_rate as f64)
        } else {
            None
        };
        ctx.time.set_fps(fps);
        // Initiliaze console
        let font = Font::load_psf(Path::new("res/font/font.psf"))?;
        let cols = options.window_size.0 / font.get_width();
        let rows = options.window_size.1 / font.get_height();
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
            platform,
            options,
            sound_buffer,
            video_buffer,
        };
        let main_screen = MainScreen::build(&mut state)?;
        let mut screens: Vec<Box<dyn Screen2<AppState>>> = Vec::new();
        screens.push(Box::new(main_screen));
        Ok(AppController { state, screens })
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

impl Controller for AppController {
    fn handle_events(&mut self, ctx: &mut App) -> Result<(), String> {
        match self.screens.last_mut() {
            Some(screen) => {
                let mut events = self.state.platform.sdl.event_pump().unwrap();
                for event in events.poll_iter() {
                    match event {
                        Event::Quit { .. } => {
                            ctx.running = false;
                            break;
                        }
                        _ => {
                            let transition = screen.handle_event(ctx, &mut self.state, event)?;
                            match transition {
                                Transition::None => {}
                                other => {
                                    self.process_transition(other);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            None => {
                ctx.running = false;
            }
        }
        Ok(())
    }

    fn update(&mut self, ctx: &mut App) -> Result<(), String> {
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

    fn draw(&mut self, ctx: &mut App) -> Result<(), String> {
        match self.screens.last_mut() {
            Some(screen) => {
                let transition = screen.draw(ctx, &mut self.state)?;
                self.state.platform.window.present();
                self.process_transition(transition);
            }
            None => {
                ctx.running = false;
            }
        }
        Ok(())
    }
}
