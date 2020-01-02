// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

mod console;
mod main;

pub use main::MainScreen;

use sdl2::event::Event;

use crate::app::App;

pub enum Transition<T> {
    None,
    Push(Box<dyn Screen2<T>>),
    Pop,
}

pub trait Screen2<T> {
    fn handle_event(
        &mut self,
        ctx: &mut App,
        state: &mut T,
        event: Event,
    ) -> Result<Transition<T>, String>;

    fn update(&mut self, ctx: &mut App, state: &mut T) -> Result<Transition<T>, String>;

    fn draw(&mut self, ctx: &mut App, state: &mut T) -> Result<Transition<T>, String>;
}
