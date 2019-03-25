// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::thread;

use sdl2::joystick::Joystick;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;
use zinc64_core::{new_shared, Shared};
use zinc64_debug::{Command, Debugger};
use zinc64_emu::system::C64;

use crate::debug::Debug;
use crate::palette::Palette;
use crate::sound_buffer::SoundBuffer;
use crate::ui::{Action, ConsoleScreen, MainScreen, Screen};
use crate::util::Font;
use crate::video_buffer::VideoBuffer;
use std::path::Path;

const CONSOLE_COLS: u32 = 80;
const CONSOLE_ROWS: u32 = 40;
const CONSOLE_BUFFER: usize = 2048;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State {
    New,
    Running,
    Paused,
    Halted,
    Stopped,
}

#[derive(Copy, Clone, Debug)]
pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

pub struct Options {
    pub fullscreen: bool,
    pub jam_action: JamAction,
    pub speed: u8,
    pub warp_mode: bool,
    pub window_size: (u32, u32),
    // Debug
    pub debug: bool,
    pub dbg_address: SocketAddr,
    pub rap_address: SocketAddr,
}

pub struct AppState {
    pub c64: C64,
    pub debug: Debug,
    pub state: State,
    pub options: Options,
}

#[allow(unused)]
pub struct App {
    // Resources
    sdl_context: Sdl,
    sdl_joystick1: Option<Joystick>,
    sdl_joystick2: Option<Joystick>,
    window: Shared<Canvas<Window>>,
    // Screens
    main_screen: Shared<dyn Screen<AppState>>,
    console_screen: Shared<dyn Screen<AppState>>,
    // State
    state: AppState,
}

impl App {
    pub fn build(
        c64: C64,
        sound_buffer: Arc<SoundBuffer>,
        video_buffer: Shared<VideoBuffer>,
        options: Options,
    ) -> Result<App, String> {
        let sdl_context = sdl2::init()?;
        // Initialize window
        info!(
            "Opening app window {}x{}",
            options.window_size.0, options.window_size.1
        );
        let sdl_video = sdl_context.video()?;
        let mut window_builder =
            sdl_video.window("zinc64", options.window_size.0, options.window_size.1);
        window_builder.opengl();
        if options.fullscreen {
            window_builder.fullscreen();
        } else {
            window_builder.position_centered();
            window_builder.resizable();
        }
        let window = window_builder
            .build()
            .map_err(|_| "failed to create window")?;
        let window = new_shared(
            window
                .into_canvas()
                .accelerated()
                .present_vsync()
                .build()
                .map_err(|_| "failed to create window")?,
        );
        // Initialize resources
        let sdl_joystick = sdl_context.joystick()?;
        sdl_joystick.set_event_state(true);
        let sdl_joystick1 = c64.get_joystick1().as_ref().and_then(|joystick| {
            if !joystick.is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.get_index());
                sdl_joystick.open(joystick.get_index() as u32).ok()
            } else {
                None
            }
        });
        let sdl_joystick2 = c64.get_joystick2().as_ref().and_then(|joystick| {
            if !joystick.is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.get_index());
                sdl_joystick.open(joystick.get_index() as u32).ok()
            } else {
                None
            }
        });
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
        let state = AppState {
            c64,
            state: State::New,
            debug: Debug::new(debug_rx),
            options,
        };
        // Initialize screens
        let main_screen = new_shared(MainScreen::build(
            &sdl_context,
            &state.c64,
            window.clone(),
            sound_buffer,
            video_buffer,
        )?);
        let console_screen = new_shared(ConsoleScreen::build(
            CONSOLE_COLS,
            CONSOLE_ROWS,
            CONSOLE_BUFFER,
            Font::load_psf(Path::new("res/font/font.psf"))?,
            Palette::default(),
            window.clone(),
        )?);
        Ok(App {
            sdl_context,
            sdl_joystick1,
            sdl_joystick2,
            window,
            main_screen,
            console_screen,
            state,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!(target: "app", "Running main loop");
        let mut events = self.sdl_context.event_pump().unwrap();
        let mut screen = self.main_screen.clone();
        'running: loop {
            let action = screen.borrow_mut().run(&mut events, &mut self.state)?;
            match action {
                Action::Main => {
                    screen = self.main_screen.clone();
                }
                Action::Console => {
                    screen = self.console_screen.clone();
                }
                Action::Exit => {
                    break 'running;
                }
            }
        }
        Ok(())
    }
}
