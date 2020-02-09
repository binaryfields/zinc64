// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::platform::Platform;
use crate::time::Time;

pub struct Context {
    pub platform: Platform,
    pub time: Time,
    pub running: bool,
}

pub struct Options {
    pub title: String,
    pub window_size: (u32, u32),
    pub fullscreen: bool,
}

pub trait State {
    fn handle_events(&mut self, ctx: &mut Context) -> Result<(), String>;

    fn update(&mut self, ctx: &mut Context) -> Result<(), String>;

    fn draw(&mut self, ctx: &mut Context) -> Result<(), String>;
}

pub fn run<S, I>(options: Options, init: I) -> Result<(), String>
where
    S: State,
    I: FnOnce(&mut Context) -> Result<S, String>,
{
    let time = Time::new(None);
    let platform = Platform::build(
        options.title.as_str(),
        options.window_size,
        options.fullscreen,
    )?;
    let mut ctx = Context {
        platform,
        time,
        running: false,
    };
    let mut state = init(&mut ctx)?;
    ctx.running = true;
    while ctx.running {
        if let Err(e) = tick(&mut ctx, &mut state) {
            ctx.running = false;
            return Err(e);
        }
    }
    Ok(())
}

fn tick<C>(ctx: &mut Context, state: &mut C) -> Result<(), String>
where
    C: State,
{
    ctx.time.tick();
    state.handle_events(ctx)?;
    if ctx.time.has_timer_event() {
        state.update(ctx)?;
    }
    state.draw(ctx)?;
    std::thread::yield_now();
    Ok(())
}
