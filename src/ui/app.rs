/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::result::Result;

use c64::C64;

use sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};
use sdl2::video::Window;

pub struct AppWindow {
    c64: C64,
    renderer: Renderer<'static>,
    texture: Texture,
    event_pump: EventPump,
}

impl AppWindow {
    pub fn new(c64: C64) -> Result<AppWindow, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let window = video.window("zinc64", 800, 600)
            .position_centered()
            .opengl()
            .build()
            .unwrap();
        let renderer = window.renderer()
            .accelerated()
            .build()
            .unwrap();
        let texture = renderer.create_texture_streaming(PixelFormatEnum::RGB24, 256, 256)
            .unwrap();
        let event_pump = sdl.event_pump()
            .unwrap();
        Ok(
            AppWindow {
                c64: c64,
                renderer: renderer,
                texture: texture,
                event_pump: event_pump,
            }
        )
    }

    pub fn render(&mut self) {
        self.texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..256 {
                for x in 0..256 {
                    let offset = y * pitch + x * 3;
                    buffer[offset + 0] = x as u8;
                    buffer[offset + 1] = y as u8;
                    buffer[offset + 2] = 0;
                }
            }
        }).unwrap();

        self.renderer.clear();
        self.renderer.copy(&self.texture, None, Some(Rect::new(100, 100, 256, 256))).unwrap();
        self.renderer.copy_ex(&self.texture, None,
                              Some(Rect::new(450, 100, 256, 256)), 30.0, None, false, false).unwrap();
        self.renderer.present();
    }

    pub fn run(&mut self) {
        let mut last_pc = 0x0000;
        'running: loop {
            for event in self.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    Event::KeyDown { keycode: Some(Keycode::F9), keymod: LALTMOD, .. } => {
                        self.c64.reset();
                    }
                    Event::KeyDown { keycode: Some(key), .. } => {
                        let keyboard = self.c64.get_keyboard();
                        keyboard.borrow_mut().on_key_down(key);

                    }
                    Event::KeyUp { keycode: Some(key), .. } => {
                        let keyboard = self.c64.get_keyboard();
                        keyboard.borrow_mut().on_key_up(key);
                    }
                    _ => {}
                }
            }
            self.c64.step();
            // TODO c64: add breakpoint and infinite loop detection
            let cpu = self.c64.get_cpu();
            let pc = cpu.borrow().get_pc();
            if pc == 0x3463 {
                break 'running;
            }
            if pc == last_pc {
                panic!("trap at 0x{:x}", pc);
            }
            last_pc = pc;
        }
    }
}