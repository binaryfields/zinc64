// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::iter::Iterator;
use std::result::Result;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::EventPump;
use zinc64_core::Shared;

use crate::app::AppState;
use crate::command::CmdHandler;
use crate::util::{circular_buffer, CircularBuffer, Font};
use crate::util::{gfx, keymap};

use super::{Action, BaseScreen, Screen};

const BLANK_CHAR: u8 = 32;
const CURSOR_BLINK_DUR: u32 = 25;
const CURSOR_CHAR: u8 = 8;
const PROMPT: &str = "> ";

struct Console {
    // Configuration
    cols: u32,
    rows: u32,
    // Runtime state
    buffer: CircularBuffer<u8>,
    buffer_pos_snapshot: (usize, usize),
    screen_pos: usize,
    screen_pos_snapshot: usize,
}

impl Console {
    fn new(cols: u32, rows: u32, buffer_size: usize) -> Self {
        Console {
            rows,
            cols,
            buffer: CircularBuffer::new(buffer_size),
            buffer_pos_snapshot: (0, 0),
            screen_pos: 0,
            screen_pos_snapshot: 0,
        }
    }

    fn advance(&mut self) {
        if self.buffer.remaining(self.screen_pos) > ((self.rows - 1) * self.cols) as usize {
            for _ in 0..self.cols {
                self.screen_pos = self.buffer.advance(self.screen_pos);
            }
        }
    }

    fn print(&mut self, text: &[u8]) {
        let mut col = self.buffer.remaining(self.screen_pos) as u32 % self.cols;
        for ch in text {
            if *ch == '\n' as u8 {
                while col < self.cols {
                    self.buffer.push(' ' as u8);
                    col += 1;
                }
            } else {
                self.buffer.push(*ch);
                col += 1;
            }
            if col == self.cols {
                col = 0;
                self.advance();
            }
        }
    }

    fn restore_pos(&mut self) {
        self.buffer.restore_pos(self.buffer_pos_snapshot);
        self.screen_pos = self.screen_pos_snapshot;
    }

    fn save_pos(&mut self) {
        self.buffer_pos_snapshot = self.buffer.snapshot_pos();
        self.screen_pos_snapshot = self.screen_pos;
    }

    fn screen_data(&self) -> circular_buffer::Iter<u8> {
        self.buffer.iter_from(self.screen_pos)
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

pub struct ConsoleScreen {
    // Dependencies
    window: Shared<Canvas<Window>>,
    // Configuration
    bg_color: Color,
    scale_pct: u32,
    screen_dim: (u32, u32),
    // Resources
    cmd_handler: CmdHandler,
    renderer: Renderer,
    screen_tex: Texture,
    // Runtime state
    console: Console,
    cursor_timer: u32,
    cursor_visibility: bool,
    history: Vec<String>,
    history_pos: isize,
    input_buffer: Vec<u8>,
}

impl ConsoleScreen {
    pub fn build(
        cols: u32,
        rows: u32,
        buffer_size: usize,
        font: Font,
        scale_pct: u32,
        palette: [Color; 2],
        window: Shared<Canvas<Window>>,
    ) -> Result<ConsoleScreen, String> {
        let screen_dim = (cols * font.get_width(), rows * font.get_height());
        let screen_tex = window
            .borrow()
            .texture_creator()
            .create_texture_target(PixelFormatEnum::RGBA8888, screen_dim.0, screen_dim.1)
            .map_err(|_| "failed to create texture")?;
        info!("Creating console {}x{}", cols, rows);
        let mut console = Console::new(cols, rows, buffer_size);
        console.print("Type ? for the list of available commands\n".as_bytes());
        console.save_pos();
        Ok(ConsoleScreen {
            window,
            bg_color: palette[0],
            scale_pct,
            screen_dim,
            cmd_handler: CmdHandler::new(),
            renderer: Renderer::new(cols, rows, font, palette),
            screen_tex,
            console,
            cursor_timer: CURSOR_BLINK_DUR,
            cursor_visibility: false,
            history: Vec::new(),
            history_pos: -1,
            input_buffer: Vec::new(),
        })
    }

    fn blink_cursor(&mut self) {
        self.cursor_timer -= 1;
        if self.cursor_timer == 0 {
            self.reset_cursor(!self.cursor_visibility);
            self.print_input();
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

    fn handle_input(&mut self, keycode: &Keycode, keymod: &Mod) -> Option<String> {
        match *keycode {
            Keycode::Return => {
                self.console.restore_pos();
                self.console.print(PROMPT.as_ref());
                self.console.print(&self.input_buffer);
                self.console.print(&['\n' as u8]);
                self.console.save_pos();
                let input = std::str::from_utf8(&self.input_buffer).unwrap().to_string();
                self.input_buffer.clear();
                if !input.is_empty() {
                    let recent = self.history.get(0).map(|s| s.as_str()).unwrap_or("");
                    if input.as_str() != recent {
                        self.history.insert(0, input.clone());
                    }
                    self.history_pos = -1;
                    self.reset_cursor(true);
                    self.print_input();
                    Some(input)
                } else {
                    self.history_pos = -1;
                    self.reset_cursor(true);
                    self.print_input();
                    None
                }
            }
            Keycode::Backspace => {
                self.input_buffer.pop();
                self.reset_cursor(true);
                self.print_input();
                None
            }
            Keycode::Up => {
                if self.history_pos < (self.history.len() - 1) as isize {
                    self.history_pos += 1;
                    let input = self.history[self.history_pos as usize].as_bytes();
                    self.input_buffer.clear();
                    self.input_buffer.extend_from_slice(input);
                    self.reset_cursor(true);
                    self.print_input();
                }
                None
            }
            Keycode::Down => {
                if self.history_pos >= 0 {
                    self.history_pos -= 1;
                    if self.history_pos >= 0 {
                        let input = self.history[self.history_pos as usize].as_bytes();
                        self.input_buffer.clear();
                        self.input_buffer.extend_from_slice(input);
                    } else {
                        self.input_buffer.clear();
                    }
                    self.reset_cursor(true);
                    self.print_input();
                }
                None
            }
            _ => {
                let c = keymap::to_ascii(keycode, keymod);
                if c != '\0' {
                    self.input_buffer.push(c as u8);
                    self.reset_cursor(true);
                    self.print_input();
                }
                None
            }
        }
    }

    fn print_input(&mut self) {
        self.console.restore_pos();
        self.console.print(PROMPT.as_ref());
        if !self.input_buffer.is_empty() {
            self.console.print(&self.input_buffer);
        }
        if self.cursor_visibility {
            self.console.print(&[CURSOR_CHAR]);
        }
    }

    fn reset_cursor(&mut self, visible: bool) {
        self.cursor_timer = CURSOR_BLINK_DUR;
        self.cursor_visibility = visible;
    }
}

impl BaseScreen<AppState> for ConsoleScreen {
    fn handle_event(&mut self, event: &Event, _state: &mut AppState) -> Option<Action> {
        match event {
            Event::Quit { .. } => Some(Action::Exit),
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } if *keycode == Keycode::Escape => Some(Action::Main),
            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                repeat: false,
                ..
            } => {
                if let Some(input) = self.handle_input(keycode, keymod) {
                    self.console.restore_pos();
                    match self.cmd_handler.handle(&input, &mut _state.c64) {
                        Ok(output) => {
                            self.console.print(output.as_bytes());
                        }
                        Err(error) => {
                            self.console.print("ERROR: ".as_bytes());
                            self.console.print(error.as_bytes());
                            self.console.print(&['\n' as u8]);
                        }
                    }
                    self.console.save_pos();
                }
                None
            }
            _ => None,
        }
    }

    fn render(&mut self) -> Result<(), String> {
        let mut screen_data = self.console.screen_data();
        let renderer = &mut self.renderer;
        self.window
            .borrow_mut()
            .with_texture_canvas(&mut self.screen_tex, |canvas| {
                renderer
                    .render(&mut screen_data, canvas)
                    .expect("Failed to render console output");
            })
            .map_err(|e| e.to_string())?;
        let window = &mut self.window.borrow_mut();
        let output_size = window.output_size()?;
        window.set_draw_color(self.bg_color);
        window.clear();
        window.copy(&self.screen_tex, None, self.get_viewport_rect(&output_size))?;
        window.present();
        Ok(())
    }

    fn tick(&mut self) {
        self.blink_cursor();
    }
}

impl Screen<AppState> for ConsoleScreen {
    fn run(&mut self, events: &mut EventPump, state: &mut AppState) -> Result<Action, String> {
        BaseScreen::run(self, events, state)
    }
}
