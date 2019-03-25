// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::iter::Iterator;
use std::result::Result;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::EventPump;
use zinc64_core::Shared;

use crate::app::AppState;
use crate::util::gfx;
use crate::util::{CircularBuffer, Font};

use super::{Action, BaseScreen, Screen};

const BG_COLOR: usize = 0;
const FG_COLOR: usize = 1;

// TODO fix colors
// TODO add read line

struct Console {
    // Configuration
    cols: u32,
    rows: u32,
    // Resources
    font: Font,
    // Runtime State
    buffer: CircularBuffer<u8>,
    buffer_pos: usize,
    screen: Vec<u8>,
}

impl Console {
    pub fn new(cols: u32, rows: u32, buffer_size: usize, font: Font) -> Self {
        Console {
            cols,
            rows,
            font,
            buffer: CircularBuffer::new(buffer_size),
            buffer_pos: 0,
            screen: vec![0x7f; (cols * rows) as usize],
        }
    }

    pub fn advance(&mut self) {
        if self.buffer.remaining(self.buffer_pos) > self.screen.len() {
            for _ in 0..self.cols {
                self.buffer_pos = self.buffer.advance(self.buffer_pos);
            }
        }
    }

    pub fn print(&mut self, text: &[u8]) {
        let mut col = self.buffer.remaining(self.buffer_pos) as u32 % self.cols;
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

    pub fn update(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let mut buffer_iter = self.buffer.iter_from(self.buffer_pos);
        let mut screen_pos = 0;
        let mut y = 0;
        for _ in 0..self.rows {
            let mut x = 0;
            for _ in 0..self.cols {
                let ch = buffer_iter.next().unwrap_or(&0x7f);
                if *ch != self.screen[screen_pos] {
                    self.screen[screen_pos] = *ch;
                    gfx::draw_char(canvas, &self.font, *ch, x, y, 1, 0)?;
                }
                screen_pos += 1;
                x += self.font.get_width();
            }
            y += self.font.get_height();
        }
        Ok(())
    }
}

#[allow(unused)]
pub struct ConsoleScreen {
    // Dependencies
    palette: [u32; 16],
    window: Shared<Canvas<Window>>,
    // Resources
    console: Console,
    screen_tex: Texture,
}

#[allow(unused)]
impl ConsoleScreen {
    pub fn build(
        cols: u32,
        rows: u32,
        buffer_size: usize,
        font: Font,
        palette: [u32; 16],
        window: Shared<Canvas<Window>>,
    ) -> Result<ConsoleScreen, String> {
        let mut screen_tex = window
            .borrow()
            .texture_creator()
            .create_texture_target(
                PixelFormatEnum::RGBA8888,
                cols * font.get_width(),
                rows * font.get_height(),
            )
            .map_err(|_| "failed to create texture")?;
        window
            .borrow_mut()
            .with_texture_canvas(&mut screen_tex, |canvas| {
                canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                canvas.clear();
            })
            .map_err(|e| e.to_string())?;
        let mut console = Console::new(cols, rows, buffer_size, font);
        console.print("Hello world 1\nHello world 2\nHello world 3\n".as_ref()); // FIXME
        Ok(ConsoleScreen {
            palette,
            window,
            console,
            screen_tex,
        })
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
                keycode: Some(_keycode),
                repeat: false,
                ..
            } => None,
            _ => None,
        }
    }

    fn render(&mut self) -> Result<(), String> {
        let console = &mut self.console;
        self.window
            .borrow_mut()
            .with_texture_canvas(&mut self.screen_tex, |canvas| {
                console
                    .update(canvas)
                    .expect("Failed to render console output");
            })
            .map_err(|e| e.to_string())?;
        let window = &mut self.window.borrow_mut();
        window.clear();
        window.copy(&self.screen_tex, None, None)?;
        window.present();
        Ok(())
    }
}

impl Screen<AppState> for ConsoleScreen {
    fn run(&mut self, events: &mut EventPump, state: &mut AppState) -> Result<Action, String> {
        BaseScreen::run(self, events, state)
    }
}
