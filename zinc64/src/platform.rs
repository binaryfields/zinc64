// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::collections::HashMap;

use glow;
use sdl2::joystick::Joystick;
use sdl2::video::{GLContext, SwapInterval, Window};
use sdl2::Sdl;

use crate::gfx::gl::GlDevice;

pub struct Platform {
    pub sdl: Sdl,
    _sdl_gl_context: GLContext,
    pub window: Window,
    pub gl: GlDevice,
    pub joysticks: HashMap<u32, Joystick>,
}

impl Platform {
    pub fn build(title: &str, window_size: (u32, u32), fullscreen: bool) -> Result<Self, String> {
        info!("Opening app window {}x{}", window_size.0, window_size.1);
        let sdl = sdl2::init()?;
        let video_sys = sdl.video()?;
        // Initialize gl
        let gl_attr = video_sys.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);
        // Initialize window
        let mut window_builder = video_sys.window(title, window_size.0, window_size.1);
        window_builder.opengl();
        if fullscreen {
            window_builder.fullscreen();
        } else {
            window_builder.position_centered();
            window_builder.resizable();
        }
        let window = window_builder
            .build()
            .map_err(|_| "failed to create window")?;
        let sdl_gl_context = window
            .gl_create_context()
            .map_err(|_| "failed to create gl context")?;
        let gl_ctx =
            glow::Context::from_loader_function(|s| video_sys.gl_get_proc_address(s) as *const _);
        video_sys
            .gl_set_swap_interval(SwapInterval::VSync)
            .map_err(|_| "failed to set vsync")?;
        // Initialize joysticks
        let joysticks = HashMap::new();
        /* FIXME
        let joystick_sys = sdl.joystick()?;
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
        */
        Ok(Platform {
            sdl,
            _sdl_gl_context: sdl_gl_context,
            window,
            gl: GlDevice::new(gl_ctx),
            joysticks,
        })
    }
}
