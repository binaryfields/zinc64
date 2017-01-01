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
use std::thread;
use std::time::Duration;

use c64::C64;

use sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};
use sdl2::video::Window;
use time;

#[derive(PartialEq)]
enum State {
    Running,
    Paused,
    Stopped,
}

pub struct AppWindow {
    c64: C64,
    renderer: Renderer<'static>,
    texture: Texture,
    event_pump: EventPump,
    state: State,
    last_frame_ts: u64,
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
        let screen_size = c64.get_config().visible_size;
        let texture = renderer.create_texture_streaming(PixelFormatEnum::ARGB8888,
                                                        screen_size.width as u32,
                                                        screen_size.height as u32)
            .unwrap();
        let event_pump = sdl.event_pump()
            .unwrap();
        Ok(
            AppWindow {
                c64: c64,
                renderer: renderer,
                texture: texture,
                event_pump: event_pump,
                state: State::Running,
                last_frame_ts: 0,
            }
        )
    }

    fn handle_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    self.state = State::Stopped;
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
    }

    fn render(&mut self) {
        let screen_size = self.c64.get_config().visible_size;
        let rt_ref = self.c64.get_render_target();
        {
            let rt = rt_ref.borrow();
            self.texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..(screen_size.height as usize) {
                    for x in 0..(screen_size.width as usize) {
                        let offset = y * pitch + x * 4;
                        let color = rt.read(x as u16, y as u16);
                        buffer[offset + 0] = (color & 0x000000ff) as u8;
                        buffer[offset + 1] = ((color & 0x0000ff00) >> 8) as u8;
                        buffer[offset + 2] = ((color & 0x00ff0000) >> 16) as u8;
                        buffer[offset + 3] = 0 as u8;
                    }
                }
            }).unwrap();
        }
        rt_ref.borrow_mut().set_sync(false);
        self.renderer.clear();
        self.renderer.copy(&self.texture, None, None).unwrap();
        self.renderer.present();
        self.last_frame_ts = time::precise_time_ns();
    }

    pub fn run(&mut self) {
        while self.state == State::Running {
            self.handle_events();
            self.run_frame();
        }
    }

    fn run_frame(&mut self) {
        let frame_cycles = (self.c64.get_config().cpu_frequency as f64
            / self.c64.get_config().refresh_rate) as u64;
        let mut last_pc = 0x0000;
        let rt = self.c64.get_render_target();
        for i in 0..frame_cycles {
            self.c64.step();
            if rt.borrow().get_sync() {
                self.wait_vsync();
                self.render();
            }
            // TODO c64: add breakpoint and infinite loop detection
            let cpu = self.c64.get_cpu();
            let pc = cpu.borrow().get_pc();
            if pc == 0x3463 {
                self.state = State::Stopped;
            }
            if pc == last_pc {
                panic!("trap at 0x{:x}", pc);
            }
            last_pc = pc;
        }
    }

    fn wait_vsync(&self) {
        let elapsed_ns = time::precise_time_ns() - self.last_frame_ts;
        if elapsed_ns < self.c64.get_config().refrest_rate_ns {
            let wait_ns = self.c64.get_config().refrest_rate_ns - elapsed_ns;
            let wait = Duration::from_millis(wait_ns / 1_000_000);
            thread::sleep(wait);
        }
    }
}