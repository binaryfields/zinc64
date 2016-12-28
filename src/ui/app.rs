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
extern crate sdl2;

use std::result::Result;

use self::sdl2::pixels::PixelFormatEnum;
use self::sdl2::rect::Rect;
use self::sdl2::event::{Event, EventPump};
use self::sdl2::keyboard::Keycode;
use self::sdl2::video::Window;
use self::sdl2::render::Renderer;
use self::sdl2::render::Texture;

struct MainWindow {
    window: Window,
    renderer: Renderer,
    texture: Texture,
    event_pump: EventPump,
}

impl MainWindow {
    pub fn new() -> Result<MainWindow, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let window = video.window("zinc64", 800, 600)
            .position_centered()
            .opengl()
            .build()?;
        let renderer = window.renderer().build()?;
        let texture = renderer.create_texture_streaming(PixelFormatEnum::RGB24, 256, 256)?;
        let event_pump = sdl.event_pump()?;
        MainWindow {
            window: window,
            renderer: renderer,
            texture: texture,
            event_pump: event_pump,
        }
    }

    pub fn render(&mut self) {
        self.texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..256 {
                for x in 0..256 {
                    let offset = y*pitch + x*3;
                    buffer[offset + 0] = x as u8;
                    buffer[offset + 1] = y as u8;
                    buffer[offset + 2] = 0;
                }
            }
        }).unwrap();

        self.renderer.clear();
        self.renderer.copy(&texture, None, Some(Rect::new(100, 100, 256, 256))).unwrap();
        self.renderer.copy_ex(&texture, None,
                         Some(Rect::new(450, 100, 256, 256)), 30.0, None, false, false).unwrap();
        self.renderer.present();
    }

    pub fn run(&mut self) {
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..}
                    | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }
            // The rest of the game loop goes here...
        }
    }
}
}