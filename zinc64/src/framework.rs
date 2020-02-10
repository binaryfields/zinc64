// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::platform::Platform;
use crate::time::Time;

use glutin::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
};

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
    fn handle_event(&mut self, ctx: &mut Context, event: Event<()>) -> Result<(), String>;

    fn update(&mut self, ctx: &mut Context) -> Result<(), String>;

    fn draw(&mut self, ctx: &mut Context) -> Result<(), String>;
}

pub fn run<S, I>(options: Options, init: I) -> Result<(), String>
where
    S: State,
    I: FnOnce(&mut Context) -> Result<S, String>,
{
    let time = Time::new(None);
    let (mut event_loop, platform) = Platform::build(
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
        if let Err(e) = tick(&mut event_loop, &mut ctx, &mut state) {
            ctx.running = false;
            return Err(e);
        }
    }
    Ok(())
}

fn tick<C>(event_loop: &mut EventLoop<()>, ctx: &mut Context, state: &mut C) -> Result<(), String>
where
    C: State,
{
    ctx.time.tick();
    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                ctx.running = false;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                ctx.platform.windowed_context.resize(size);
            }
            Event::MainEventsCleared => {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
        state.handle_event(ctx, event).expect("FIXME");
    });
    if ctx.time.has_timer_event() {
        state.update(ctx)?;
    }
    state.draw(ctx)?;
    std::thread::yield_now();
    Ok(())
}
