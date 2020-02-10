// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// use std::collections::HashMap;

use glow;

use crate::gfx::gl::GlDevice;

pub struct Platform {
    pub windowed_context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub gl: GlDevice,
}

// FIXME add fullscreen support

impl Platform {
    pub fn build(
        title: &str,
        window_size: (u32, u32),
        _fullscreen: bool,
    ) -> Result<(glutin::event_loop::EventLoop<()>, Platform), String> {
        info!("Opening app window {}x{}", window_size.0, window_size.1);
        let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title(title)
            .with_decorations(true)
            .with_resizable(true)
            .with_inner_size(glutin::dpi::LogicalSize::new(
                window_size.0 as f32,
                window_size.1 as f32,
            ));
        let windowed_context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        configure_theme(&windowed_context);
        let gl_ctx = glow::Context::from_loader_function(|s| {
            windowed_context.get_proc_address(s) as *const _
        });
        // Initialize joysticks
        /*
        FIXME
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
        */
        let platform = Platform {
            windowed_context,
            gl: GlDevice::new(gl_ctx),
        };
        Ok((event_loop, platform))
    }
}

#[cfg(target_os = "linux")]
fn configure_theme(windowed_context: &glutin::WindowedContext<glutin::PossiblyCurrent>) {
    use glutin::platform::unix::WindowExtUnix;
    windowed_context
        .window()
        .set_wayland_theme(theme::DarkTheme);
}

#[cfg(not(target_os = "linux"))]
fn configure_theme(windowed_context: &glutin::WindowedContext<glutin::PossiblyCurrent>) {
}

#[cfg(target_os = "linux")]
mod theme {
    use glutin::platform::unix::{ButtonState, Theme};

    const PRIMARY_BG_ACTIVE: [u8; 4] = [0xFF, 0x28, 0x28, 0x28];
    const PRIMARY_BG_INACTIVE: [u8; 4] = [0xFF, 0x35, 0x35, 0x35];
    const SECONDARY_ACTIVE: [u8; 4] = [0xFF, 0xc3, 0xc3, 0xc3];
    const BUTTON_HOVER: [u8; 4] = [0xFF, 0x50, 0x50, 0x50];

    pub struct DarkTheme;

    impl Theme for DarkTheme {
        /// Primary color of the scheme.
        fn primary_color(&self, window_active: bool) -> [u8; 4] {
            if window_active {
                PRIMARY_BG_ACTIVE
            } else {
                PRIMARY_BG_INACTIVE
            }
        }

        /// Secondary color of the scheme.
        fn secondary_color(&self, window_active: bool) -> [u8; 4] {
            if window_active {
                SECONDARY_ACTIVE
            } else {
                PRIMARY_BG_INACTIVE
            }
        }

        /// Color for the close button.
        fn close_button_color(&self, status: ButtonState) -> [u8; 4] {
            match status {
                ButtonState::Hovered => BUTTON_HOVER,
                _ => PRIMARY_BG_ACTIVE,
            }
        }

        /// Icon color for the close button, defaults to the secondary color.
        #[allow(unused_variables)]
        fn close_button_icon_color(&self, status: ButtonState) -> [u8; 4] {
            self.secondary_color(true)
        }

        /// Background color for the maximize button.
        fn maximize_button_color(&self, status: ButtonState) -> [u8; 4] {
            match status {
                ButtonState::Hovered => BUTTON_HOVER,
                _ => PRIMARY_BG_ACTIVE,
            }
        }

        /// Icon color for the maximize button, defaults to the secondary color.
        #[allow(unused_variables)]
        fn maximize_button_icon_color(&self, status: ButtonState) -> [u8; 4] {
            self.secondary_color(true)
        }

        /// Background color for the minimize button.
        fn minimize_button_color(&self, status: ButtonState) -> [u8; 4] {
            match status {
                ButtonState::Hovered => BUTTON_HOVER,
                _ => PRIMARY_BG_ACTIVE,
            }
        }

        /// Icon color for the minimize button, defaults to the secondary color.
        #[allow(unused_variables)]
        fn minimize_button_icon_color(&self, status: ButtonState) -> [u8; 4] {
            self.secondary_color(true)
        }
    }
}
