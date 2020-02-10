// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::iter::Iterator;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

use cgmath::num_traits::zero;
use cgmath::{vec2, Vector2};
use glutin::event::{
    ElementState, Event, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent,
};

use crate::app::AppState;
use crate::cmd::Executor;
use crate::console::Console;
use crate::framework::Context;
use crate::gfx::{gl, sprite, Color, Font, Rect, RectI};
use crate::util::keymap;

use super::{Screen, Transition};

const BLANK_CHAR: u8 = 32;
const CURSOR_BLINK_DUR: u32 = 25;
const CURSOR_CHAR: u8 = 8;
const PROMPT: &str = "> ";

pub struct ConsoleScreen {
    // Configuration
    rows: u32,
    cols: u32,
    palette: [Color; 2],
    // Resources
    batch: sprite::Batch,
    font: Font,
    font_tex: Rc<gl::Texture>,
    // Runtime state
    cmd_handler: Executor,
    cursor_timer: u32,
    cursor_visibility: bool,
    history_pos: isize,
    input_buffer: Vec<u8>,
}

impl ConsoleScreen {
    pub fn build(ctx: &mut Context, state: &mut AppState) -> Result<ConsoleScreen, String> {
        let cols = state.console.cols;
        let rows = state.console.rows;
        let palette = [
            Color::from_rgb(0x28, 0x28, 0x28),
            Color::from_rgb(0xeb, 0xdb, 0xb2),
        ];

        let gl = &mut ctx.platform.gl;

        let font = Font::load_psf(Path::new("res/font/font.psf"))?;
        let font_data = font.as_rgba();
        let font_data_len = font_data.len() * core::mem::size_of::<u32>();
        let font_ptr =
            unsafe { core::slice::from_raw_parts(font_data.as_ptr() as *const u8, font_data_len) };
        let font_tex_size = vec2(
            font.get_glypth_count() * font.get_width(),
            font.get_height(),
        );
        let font_tex = Rc::new(gl.create_texture(font_tex_size.cast::<i32>().unwrap())?);
        gl.set_texture_data(&font_tex, font_ptr);

        let screen_element_count = (cols * rows) as usize;
        let screen_size = vec2(cols * font.get_width(), rows * font.get_height())
            .cast::<f32>()
            .unwrap();
        let window_size = ctx.platform.windowed_context.window().inner_size();

        let mut batch = sprite::Batch::new(gl, screen_element_count)?;
        batch.set_projection(gl, Rect::from_points(zero(), screen_size), true);
        batch.set_viewport(
            gl,
            RectI::new(
                zero(),
                Vector2::new(window_size.width as i32, window_size.height as i32),
            ),
        );

        Ok(ConsoleScreen {
            rows,
            cols,
            palette,
            batch,
            font,
            font_tex,
            cmd_handler: Executor::new(),
            cursor_timer: CURSOR_BLINK_DUR,
            cursor_visibility: false,
            history_pos: -1,
            input_buffer: Vec::new(),
        })
    }

    fn handle_input(
        &mut self,
        state: &mut AppState,
        virtual_code: VirtualKeyCode,
        elmt_state: ElementState,
        modifiers: ModifiersState,
    ) -> Option<String> {
        match (virtual_code, elmt_state) {
            (VirtualKeyCode::Return, ElementState::Pressed) => {
                state.console.restore_pos();
                state.console.print(PROMPT.as_ref());
                state.console.print(&self.input_buffer);
                state.console.print(&['\n' as u8]);
                state.console.save_pos();
                let input = std::str::from_utf8(&self.input_buffer).unwrap().to_string();
                self.input_buffer.clear();
                if !input.is_empty() {
                    let recent = state
                        .console_history
                        .get(0)
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if input.as_str() != recent {
                        state.console_history.insert(0, input.clone());
                    }
                    self.history_pos = -1;
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                    Some(input)
                } else {
                    self.history_pos = -1;
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                    None
                }
            }
            (VirtualKeyCode::Back, ElementState::Pressed) => {
                self.input_buffer.pop();
                self.reset_cursor(true);
                self.print_input(&mut state.console);
                None
            }
            (VirtualKeyCode::Up, ElementState::Pressed) => {
                if self.history_pos < (state.console_history.len() - 1) as isize {
                    self.history_pos += 1;
                    let input = state.console_history[self.history_pos as usize].as_bytes();
                    self.input_buffer.clear();
                    self.input_buffer.extend_from_slice(input);
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                }
                None
            }
            (VirtualKeyCode::Down, ElementState::Pressed) => {
                if self.history_pos >= 0 {
                    self.history_pos -= 1;
                    if self.history_pos >= 0 {
                        let input = state.console_history[self.history_pos as usize].as_bytes();
                        self.input_buffer.clear();
                        self.input_buffer.extend_from_slice(input);
                    } else {
                        self.input_buffer.clear();
                    }
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                }
                None
            }
            (_, ElementState::Pressed) => {
                let c = keymap::to_ascii2(virtual_code, modifiers);
                if c != '\0' {
                    self.input_buffer.push(c as u8);
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                }
                None
            }
            _ => None,
        }
    }

    fn print_input(&mut self, console: &mut Console) {
        console.restore_pos();
        console.print(PROMPT.as_ref());
        if !self.input_buffer.is_empty() {
            console.print(&self.input_buffer);
        }
        if self.cursor_visibility {
            console.print(&[CURSOR_CHAR]);
        }
    }

    // -- Cursor

    fn blink_cursor(&mut self, console: &mut Console) {
        self.cursor_timer -= 1;
        if self.cursor_timer == 0 {
            self.reset_cursor(!self.cursor_visibility);
            self.print_input(console);
        }
    }

    fn reset_cursor(&mut self, visible: bool) {
        self.cursor_timer = CURSOR_BLINK_DUR;
        self.cursor_visibility = visible;
    }
}

impl Screen<AppState> for ConsoleScreen {
    fn handle_event(
        &mut self,
        ctx: &mut Context,
        state: &mut AppState,
        event: Event<()>,
    ) -> Result<Transition<AppState>, String> {
        let app_state = state;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    self.batch.set_viewport(
                        &mut ctx.platform.gl,
                        RectI::new(zero(), vec2(size.width as i32, size.height as i32)),
                    );
                    Ok(Transition::None)
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            modifiers,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Escape, ElementState::Pressed) => Ok(Transition::Pop),
                    _ => {
                        if let Some(input) =
                            self.handle_input(app_state, virtual_code, state, modifiers)
                        {
                            app_state.console.restore_pos();
                            match self.cmd_handler.execute(
                                &input,
                                &mut app_state.c64,
                                &mut app_state.console,
                            ) {
                                Ok(_) => {}
                                Err(error) => {
                                    app_state.console.print("ERROR: ".as_bytes());
                                    app_state.console.print(error.as_bytes());
                                    app_state.console.print(&['\n' as u8]);
                                }
                            }
                            app_state.console.save_pos();
                        }
                        Ok(Transition::None)
                    }
                },
                _ => Ok(Transition::None),
            },
            _ => Ok(Transition::None),
        }
    }

    fn update(
        &mut self,
        _ctx: &mut Context,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        self.blink_cursor(&mut state.console);
        Ok(Transition::None)
    }

    fn draw(
        &mut self,
        ctx: &mut Context,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        let font_size = self.font.get_size().cast::<f32>().unwrap();
        let mut screen_data = state.console.screen_data();
        let gl = &mut ctx.platform.gl;
        gl.clear(self.palette[0]);

        self.batch.begin(gl, Some(self.font_tex.clone()));
        for row in 0..self.rows {
            let y = row * self.font.get_height();
            let mut x = 0;
            for _col in 0..self.cols {
                let ch = screen_data.next().unwrap_or(&BLANK_CHAR);
                let dst = Rect::new(vec2(x as f32, y as f32), font_size);
                let uv = self.font.get_tex_coords(*ch as u32);
                self.batch.push(gl, dst, uv, self.palette[1]);
                x += self.font.get_width();
            }
        }
        self.batch.end(gl);

        ctx.platform
            .windowed_context
            .swap_buffers()
            .map_err(|_| "failed to swap buffers")?;
        Ok(Transition::None)
    }
}
