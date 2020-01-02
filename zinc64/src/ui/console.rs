// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::iter::Iterator;
use std::path::Path;
use std::result::Result;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::app::{App, AppState};
use crate::cmd::Executor;
use crate::console::Console;
use crate::gfx::{self, Font};
use crate::util::{circular_buffer, keymap};

use super::{Screen2, Transition};

const BLANK_CHAR: u8 = 32;
const CURSOR_BLINK_DUR: u32 = 25;
const CURSOR_CHAR: u8 = 8;
const PROMPT: &str = "> ";

pub struct ConsoleScreen {
    // Configuration
    bg_color: Color,
    scale_pct: u32,
    screen_dim: (u32, u32),
    // Resources
    cmd_handler: Executor,
    renderer: Renderer,
    screen_tex: Texture,
    // Runtime state
    cursor_timer: u32,
    cursor_visibility: bool,
    history_pos: isize,
    input_buffer: Vec<u8>,
}

impl ConsoleScreen {
    pub fn build(ctx: &mut AppState) -> Result<ConsoleScreen, String> {
        let font = Font::load_psf(Path::new("res/font/font.psf"))?;
        let cols = ctx.console.cols;
        let rows = ctx.console.rows;
        let screen_dim = (cols * font.get_width(), rows * font.get_height());
        let screen_tex = ctx
            .platform
            .window
            .texture_creator()
            .create_texture_target(PixelFormatEnum::RGBA8888, screen_dim.0, screen_dim.1)
            .map_err(|_| "failed to create texture")?;
        let palette = [Color::from((45, 45, 45)), Color::from((143, 135, 114))];
        let renderer = Renderer::new(cols, rows, font, palette);
        Ok(ConsoleScreen {
            bg_color: palette[0],
            scale_pct: 100,
            screen_dim,
            cmd_handler: Executor::new(),
            renderer,
            screen_tex,
            cursor_timer: CURSOR_BLINK_DUR,
            cursor_visibility: false,
            history_pos: -1,
            input_buffer: Vec::new(),
        })
    }

    fn blink_cursor(&mut self, console: &mut Console) {
        self.cursor_timer -= 1;
        if self.cursor_timer == 0 {
            self.reset_cursor(!self.cursor_visibility);
            self.print_input(console);
        }
    }

    fn get_viewport_rect(&self, output_size: &(u32, u32)) -> Rect {
        let center = Point::new(output_size.0 as i32 / 2, output_size.1 as i32 / 2);
        Rect::from_center(
            center,
            self.screen_dim.0 * self.scale_pct / 100,
            self.screen_dim.1 * self.scale_pct / 100,
        )
    }

    fn handle_input(
        &mut self,
        state: &mut AppState,
        keycode: &Keycode,
        keymod: &Mod,
    ) -> Option<String> {
        match *keycode {
            Keycode::Return => {
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
            Keycode::Backspace => {
                self.input_buffer.pop();
                self.reset_cursor(true);
                self.print_input(&mut state.console);
                None
            }
            Keycode::Up => {
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
            Keycode::Down => {
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
            _ => {
                let c = keymap::to_ascii(keycode, keymod);
                if c != '\0' {
                    self.input_buffer.push(c as u8);
                    self.reset_cursor(true);
                    self.print_input(&mut state.console);
                }
                None
            }
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

    fn reset_cursor(&mut self, visible: bool) {
        self.cursor_timer = CURSOR_BLINK_DUR;
        self.cursor_visibility = visible;
    }
}

impl Screen2<AppState> for ConsoleScreen {
    fn handle_event(
        &mut self,
        _ctx: &mut App,
        state: &mut AppState,
        event: Event,
    ) -> Result<Transition<AppState>, String> {
        match &event {
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } if *keycode == Keycode::Escape => Ok(Transition::Pop),
            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                repeat: false,
                ..
            } => {
                if let Some(input) = self.handle_input(state, keycode, keymod) {
                    state.console.restore_pos();
                    match self
                        .cmd_handler
                        .execute(&input, &mut state.c64, &mut state.console)
                    {
                        Ok(_) => {}
                        Err(error) => {
                            state.console.print("ERROR: ".as_bytes());
                            state.console.print(error.as_bytes());
                            state.console.print(&['\n' as u8]);
                        }
                    }
                    state.console.save_pos();
                }
                Ok(Transition::None)
            }
            _ => Ok(Transition::None),
        }
    }

    fn update(
        &mut self,
        _ctx: &mut App,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        self.blink_cursor(&mut state.console);
        Ok(Transition::None)
    }

    fn draw(
        &mut self,
        _ctx: &mut App,
        state: &mut AppState,
    ) -> Result<Transition<AppState>, String> {
        let mut screen_data = state.console.screen_data();
        let renderer = &mut self.renderer;
        state
            .platform
            .window
            .with_texture_canvas(&mut self.screen_tex, |canvas| {
                renderer
                    .render(&mut screen_data, canvas)
                    .expect("Failed to render console output");
            })
            .map_err(|e| e.to_string())?;
        let window = &mut state.platform.window;
        let output_size = window.output_size()?;
        window.set_draw_color(self.bg_color);
        window.clear();
        window.copy(&self.screen_tex, None, self.get_viewport_rect(&output_size))?;
        Ok(Transition::None)
    }
}

struct Renderer {
    // Configuration
    cols: u32,
    rows: u32,
    font: Font,
    palette: [Color; 2],
    // Runtime State
    screen: Vec<u8>,
}

impl Renderer {
    fn new(cols: u32, rows: u32, font: Font, palette: [Color; 2]) -> Self {
        Renderer {
            cols,
            rows,
            font,
            palette,
            screen: vec![0x7f; (cols * rows) as usize],
        }
    }

    fn render(
        &mut self,
        buffer: &mut circular_buffer::Iter<u8>,
        canvas: &mut Canvas<Window>,
    ) -> Result<(), String> {
        let mut screen_pos = 0;
        let mut y = 0;
        for _ in 0..self.rows {
            let mut x = 0;
            for _ in 0..self.cols {
                let ch = buffer.next().unwrap_or(&BLANK_CHAR);
                if *ch != self.screen[screen_pos] {
                    self.screen[screen_pos] = *ch;
                    gfx::draw_char(
                        canvas,
                        &self.font,
                        *ch,
                        x,
                        y,
                        self.palette[1],
                        self.palette[0],
                    )?;
                }
                screen_pos += 1;
                x += self.font.get_width();
            }
            y += self.font.get_height();
        }
        Ok(())
    }
}
