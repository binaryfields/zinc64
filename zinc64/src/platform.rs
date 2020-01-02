// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::app::Options;
use sdl2::joystick::Joystick;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;
use std::collections::HashMap;

pub struct Platform {
    pub sdl: Sdl,
    pub window: Canvas<Window>,
    pub joysticks: HashMap<u32, Joystick>,
}

impl Platform {
    pub fn build(title: &str, options: &Options) -> Result<Self, String> {
        info!(
            "Opening app window {}x{}",
            options.window_size.0, options.window_size.1
        );
        let sdl = sdl2::init()?;
        let video_sys = sdl.video()?;
        let joystick_sys = sdl.joystick()?;
        // Initialize window
        let mut window_builder =
            video_sys.window(title, options.window_size.0, options.window_size.1);
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
        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|_| "failed to create window")?;
        // Initialize joysticks
        let mut joysticks = HashMap::new();
        joystick_sys.set_event_state(true);
        let joy_idx = [
            options.joydev_1.index() as u32,
            options.joydev_2.index() as u32,
        ];
        for idx in &joy_idx {
            match idx {
                0..=1 => {
                    let joystick = joystick_sys
                        .open(*idx)
                        .map_err(|_| "failed to open joystick")?;
                    joysticks.insert(*idx, joystick);
                }
                _ => {}
            }
        }
        Ok(Platform {
            sdl,
            window: canvas,
            joysticks,
        })
    }
}
