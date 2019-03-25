// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

mod console;
mod main;

use std::thread;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::EventPump;

pub use self::console::ConsoleScreen;
pub use self::main::MainScreen;

#[allow(unused)]
pub enum Action {
    Console,
    Main,
    Exit,
}

pub trait Screen<T> {
    fn run(&mut self, events: &mut EventPump, state: &mut T) -> Result<Action, String>;
}

pub trait BaseScreen<T>: Screen<T> {
    fn handle_event(&mut self, event: &Event, state: &mut T) -> Option<Action>;

    fn render(&mut self) -> Result<(), String>;

    fn run(&mut self, events: &mut EventPump, state: &mut T) -> Result<Action, String> {
        'running: loop {
            for event in events.poll_iter() {
                if let Some(action) = self.handle_event(&event, state) {
                    return Ok(action);
                }
            }
            self.render()?;
            thread::sleep(Duration::from_millis(20));
        }
    }
}
