/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sdl2;
use sdl2::{EventPump, Sdl};
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::joystick::Joystick;
use sdl2::keyboard;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{FullscreenType, Window};
use time;

use zinc64::device::joystick::Button;
use zinc64::device::keyboard::{Key, KeyEvent};
use zinc64::sound::SoundBuffer;
use zinc64::system::C64;
use zinc64::video::vic;

pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

impl JamAction {
    pub fn from(action: &str) -> JamAction {
        match action {
            "continue" => JamAction::Continue,
            "quit" => JamAction::Quit,
            "reset" => JamAction::Reset,
            _ => panic!("invalid jam action {}", action),
        }
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Running,
    Paused,
    Stopped,
}

pub struct Options {
    pub fullscreen: bool,
    pub jam_action: JamAction,
    pub height: u32,
    pub width: u32,
}

struct AppAudio {
    buffer: Arc<Mutex<SoundBuffer>>,
}

impl AudioCallback for AppAudio {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let mut input = self.buffer.lock().unwrap();
        for x in out.iter_mut() {
            let sample = input.pop();
            *x = sample as f32 * 0.000020; // FIXME magic value
        }
    }
}

// TODO ui/audio: play/resume/volume

pub struct AppWindow {
    // Dependencies
    c64: C64,
    // Audio
    audio_device: AudioDevice<AppAudio>,
    // Video
    sdl: Sdl,
    canvas: Canvas<Window>,
    // Devices
    #[allow(dead_code)] joystick1: Option<Joystick>,
    #[allow(dead_code)] joystick2: Option<Joystick>,
    // Configuration
    jam_action: JamAction,
    // Runtime State
    state: State,
    last_frame_ts: u64,
    next_keyboard_event: u32,
}

impl AppWindow {
    pub fn new(c64: C64, options: Options) -> Result<AppWindow, String> {
        let sdl = sdl2::init()?;
        // Initialize video
        info!(target: "ui", "Opening app window {}x{}", options.width, options.height);
        let video = sdl.video()?;
        let mut builder = video.window("zinc64", options.width, options.height);
        builder.position_centered();
        builder.resizable();
        builder.opengl();
        if options.fullscreen {
            builder.fullscreen();
        }
        let window = builder.build().unwrap();
        let canvas = window.into_canvas().build().unwrap();
        // Initialize audio
        let audio = sdl.audio()?;
        let audio_spec = AudioSpecDesired {
            freq: Some(c64.get_config().sound.sample_rate as i32),
            channels: Some(1),
            samples: Some(c64.get_config().sound.buffer_size as u16),
        };
        let audio_device = audio.open_playback(None, &audio_spec, |spec| {
            info!(target: "audio", "{:?}", spec);
            AppAudio {
                buffer: c64.get_sound_buffer(),
            }
        })?;
        // Initialize devices
        let joystick_subsystem = sdl.joystick()?;
        joystick_subsystem.set_event_state(true);
        let joystick1 = c64.get_joystick1().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                joystick_subsystem
                    .open(joystick.borrow().get_index() as u32)
                    .ok()
            } else {
                None
            }
        });
        let joystick2 = c64.get_joystick2().and_then(|joystick| {
            if !joystick.borrow().is_virtual() {
                info!(target: "ui", "Opening joystick {}", joystick.borrow().get_index());
                joystick_subsystem
                    .open(joystick.borrow().get_index() as u32)
                    .ok()
            } else {
                None
            }
        });
        Ok(AppWindow {
            c64: c64,
            sdl: sdl,
            audio_device: audio_device,
            canvas: canvas,
            joystick1: joystick1,
            joystick2: joystick2,
            jam_action: options.jam_action,
            state: State::Running,
            last_frame_ts: 0,
            next_keyboard_event: 0,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        info!(target: "ui", "Running main loop");
        self.audio_device.resume();
        let vic_spec = vic::Spec::new(self.c64.get_config().model.vic_model);
        let screen_size = vic_spec.display_rect.size();
        let texture_creator: TextureCreator<_> = self.canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::ARGB8888,
                screen_size.width as u32,
                screen_size.height as u32,
            )
            .unwrap();
        let mut events = self.sdl.event_pump().unwrap();
        let mut overflow_cycles = 0i32;
        'running: loop {
            match self.state {
                State::Running => {
                    self.handle_events(&mut events);
                    overflow_cycles = self.c64.run_frame(overflow_cycles);
                    if self.c64.is_cpu_jam() {
                        self.handle_cpu_jam();
                    }
                    let rt = self.c64.get_render_target();
                    if rt.borrow().get_sync() {
                        self.render(&mut texture)?;
                    }
                }
                State::Paused => {
                    self.handle_events(&mut events);
                    let wait = Duration::from_millis(20);
                    thread::sleep(wait);
                }
                State::Stopped => {
                    info!(target: "ui", "State {:?}", self.state);
                    break 'running;
                }
            }
        }
        Ok(())
    }

    fn handle_cpu_jam(&mut self) -> bool {
        let cpu = self.c64.get_cpu();
        match self.jam_action {
            JamAction::Continue => true,
            JamAction::Quit => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
                self.state = State::Stopped;
                false
            }
            JamAction::Reset => {
                warn!(target: "ui", "CPU JAM detected at 0x{:x}", cpu.borrow().get_pc());
                self.reset();
                false
            }
        }
    }

    fn render(&mut self, texture: &mut Texture) -> Result<(), String> {
        let rt = self.c64.get_render_target();
        texture
            .update(None, rt.borrow().get_pixel_data(), rt.borrow().get_pitch())
            .map_err(|_| "failed to update texture")?;
        self.canvas.clear();
        self.canvas.copy(texture, None, None)?;
        self.canvas.present();
        rt.borrow_mut().set_sync(false);
        self.last_frame_ts = time::precise_time_ns();
        Ok(())
    }

    fn reset(&mut self) {
        self.c64.reset(false);
        self.next_keyboard_event = 0;
    }

    fn toggle_datassette_play(&mut self) {
        let datassette = self.c64.get_datasette();
        if !datassette.borrow().is_playing() {
            datassette.borrow_mut().play();
        } else {
            datassette.borrow_mut().stop();
        }
    }

    fn toggle_fullscreen(&mut self) {
        let window = self.canvas.window_mut();
        match window.fullscreen_state() {
            FullscreenType::Off => {
                window.set_fullscreen(FullscreenType::True).unwrap();
            }
            FullscreenType::True => {
                window.set_fullscreen(FullscreenType::Off).unwrap();
            }
            _ => panic!("invalid fullscreen mode"),
        }
    }

    fn toggle_pause(&mut self) {
        match self.state {
            State::Running => self.state = State::Paused,
            State::Paused => self.state = State::Running,
            _ => {}
        }
    }

    fn toggle_warp(&mut self) {
        let warp_mode = self.c64.get_warp_mode();
        self.c64.set_warp_mode(!warp_mode);
    }

    // -- Event Handling

    fn handle_events(&mut self, events: &mut EventPump) {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.state = State::Stopped;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_pause();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.state = State::Stopped;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_warp();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F9),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.reset();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F1),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LCTRLMOD) =>
                {
                    self.toggle_datassette_play();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if keymod.contains(keyboard::LALTMOD) =>
                {
                    self.toggle_fullscreen();
                }
                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    ..
                } => {
                    if let Some(key_event) = self.map_key_event(key, keymod) {
                        let keyboard = self.c64.get_keyboard();
                        keyboard.borrow_mut().on_key_down(key_event);
                        if let Some(ref mut joystick) = self.c64.get_joystick1() {
                            if joystick.borrow().is_virtual() {
                                if let Some(joy_button) = self.map_joy_event(key, keymod) {
                                    joystick.borrow_mut().on_key_down(joy_button);
                                }
                            }
                        }
                        if let Some(ref mut joystick) = self.c64.get_joystick2() {
                            if joystick.borrow().is_virtual() {
                                if let Some(joy_button) = self.map_joy_event(key, keymod) {
                                    joystick.borrow_mut().on_key_down(joy_button);
                                }
                            }
                        }
                    }
                }
                Event::KeyUp {
                    keycode: Some(key),
                    keymod,
                    ..
                } => {
                    if let Some(key_event) = self.map_key_event(key, keymod) {
                        let keyboard = self.c64.get_keyboard();
                        keyboard.borrow_mut().on_key_up(key_event);
                        if let Some(ref mut joystick) = self.c64.get_joystick1() {
                            if joystick.borrow().is_virtual() {
                                if let Some(joy_button) = self.map_joy_event(key, keymod) {
                                    joystick.borrow_mut().on_key_up(joy_button);
                                }
                            }
                        }
                        if let Some(ref mut joystick) = self.c64.get_joystick2() {
                            if joystick.borrow().is_virtual() {
                                if let Some(joy_button) = self.map_joy_event(key, keymod) {
                                    joystick.borrow_mut().on_key_up(joy_button);
                                }
                            }
                        }
                    }
                }
                Event::JoyAxisMotion {
                    which,
                    axis_idx,
                    value,
                    ..
                } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_axis_motion(axis_idx, value);
                    }
                }
                Event::JoyButtonDown {
                    which, button_idx, ..
                } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_button_down(button_idx);
                    }
                }
                Event::JoyButtonUp {
                    which, button_idx, ..
                } => {
                    if let Some(ref mut joystick) = self.c64.get_joystick(which as u8) {
                        joystick.borrow_mut().on_button_up(button_idx);
                    }
                }
                _ => {}
            }
        }
        let keyboard = self.c64.get_keyboard();
        if keyboard.borrow().has_events() && self.c64.get_cycles() >= self.next_keyboard_event {
            keyboard.borrow_mut().drain_event();
            self.next_keyboard_event = self.c64.get_cycles().wrapping_add(20000);
        }
    }

    fn map_joy_event(&self, keycode: Keycode, _keymod: Mod) -> Option<Button> {
        match keycode {
            Keycode::Kp2 => Some(Button::Down),
            Keycode::Kp4 => Some(Button::Left),
            Keycode::Kp6 => Some(Button::Right),
            Keycode::Kp8 => Some(Button::Up),
            Keycode::KpEnter => Some(Button::Fire),
            _ => None,
        }
    }

    fn map_key_event(&self, keycode: Keycode, keymod: Mod) -> Option<KeyEvent> {
        match keycode {
            // Numerical
            Keycode::Num0
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num9))
            }
            Keycode::Num0 => Some(KeyEvent::new(Key::Num0)),
            Keycode::Num1 => Some(KeyEvent::new(Key::Num1)),
            Keycode::Num2
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::At))
            }
            Keycode::Num2 => Some(KeyEvent::new(Key::Num2)),
            Keycode::Num3 => Some(KeyEvent::new(Key::Num3)),
            Keycode::Num4 => Some(KeyEvent::new(Key::Num4)),
            Keycode::Num5 => Some(KeyEvent::new(Key::Num5)),
            Keycode::Num6
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num7))
            }
            Keycode::Num6 => Some(KeyEvent::new(Key::Num6)),
            Keycode::Num7
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num6))
            }
            Keycode::Num7 => Some(KeyEvent::new(Key::Num7)),
            Keycode::Num8
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Asterisk))
            }
            Keycode::Num8 => Some(KeyEvent::new(Key::Num8)),
            Keycode::Num9
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num8))
            }
            Keycode::Num9 => Some(KeyEvent::new(Key::Num9)),
            // Alpha
            Keycode::A => Some(KeyEvent::new(Key::A)),
            Keycode::B => Some(KeyEvent::new(Key::B)),
            Keycode::C => Some(KeyEvent::new(Key::C)),
            Keycode::D => Some(KeyEvent::new(Key::D)),
            Keycode::E => Some(KeyEvent::new(Key::E)),
            Keycode::F => Some(KeyEvent::new(Key::F)),
            Keycode::G => Some(KeyEvent::new(Key::G)),
            Keycode::H => Some(KeyEvent::new(Key::H)),
            Keycode::I => Some(KeyEvent::new(Key::I)),
            Keycode::J => Some(KeyEvent::new(Key::J)),
            Keycode::K => Some(KeyEvent::new(Key::K)),
            Keycode::L => Some(KeyEvent::new(Key::L)),
            Keycode::M => Some(KeyEvent::new(Key::M)),
            Keycode::N => Some(KeyEvent::new(Key::N)),
            Keycode::O => Some(KeyEvent::new(Key::O)),
            Keycode::P => Some(KeyEvent::new(Key::P)),
            Keycode::Q => Some(KeyEvent::new(Key::Q)),
            Keycode::R => Some(KeyEvent::new(Key::R)),
            Keycode::S => Some(KeyEvent::new(Key::S)),
            Keycode::T => Some(KeyEvent::new(Key::T)),
            Keycode::U => Some(KeyEvent::new(Key::U)),
            Keycode::V => Some(KeyEvent::new(Key::V)),
            Keycode::W => Some(KeyEvent::new(Key::W)),
            Keycode::X => Some(KeyEvent::new(Key::X)),
            Keycode::Y => Some(KeyEvent::new(Key::Y)),
            Keycode::Z => Some(KeyEvent::new(Key::Z)),
            //
            Keycode::Asterisk => Some(KeyEvent::new(Key::Asterisk)),
            Keycode::At => Some(KeyEvent::new(Key::At)),
            Keycode::Backslash
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_mod(Key::Minus, Key::LShift))
            }
            Keycode::Backspace => Some(KeyEvent::new(Key::Backspace)),
            Keycode::Caret => Some(KeyEvent::new(Key::Caret)),
            Keycode::Colon => Some(KeyEvent::new(Key::Colon)),
            Keycode::Comma => Some(KeyEvent::new(Key::Comma)),
            Keycode::Dollar => Some(KeyEvent::new(Key::Dollar)),
            Keycode::Equals
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Plus))
            }
            Keycode::Equals => Some(KeyEvent::new(Key::Equals)),
            Keycode::LeftBracket => Some(KeyEvent::with_mod(Key::Colon, Key::LShift)),
            Keycode::Minus => Some(KeyEvent::new(Key::Minus)),
            Keycode::Period => Some(KeyEvent::new(Key::Period)),
            Keycode::Plus => Some(KeyEvent::new(Key::Plus)),
            Keycode::Quote
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::new(Key::Num2))
            }
            Keycode::Quote => Some(KeyEvent::with_mod(Key::Num7, Key::LShift)),
            Keycode::Return => Some(KeyEvent::new(Key::Return)),
            Keycode::RightBracket => Some(KeyEvent::with_mod(Key::Semicolon, Key::LShift)),
            Keycode::Semicolon
                if keymod.contains(keyboard::LSHIFTMOD) || keymod.contains(keyboard::RSHIFTMOD) =>
            {
                Some(KeyEvent::with_disabled_shift(Key::Colon))
            }
            Keycode::Semicolon => Some(KeyEvent::new(Key::Semicolon)),
            Keycode::Slash => Some(KeyEvent::new(Key::Slash)),
            Keycode::Space => Some(KeyEvent::new(Key::Space)),
            //
            Keycode::Down => Some(KeyEvent::new(Key::CrsrDown)),
            Keycode::Home => Some(KeyEvent::new(Key::Home)),
            Keycode::LCtrl => Some(KeyEvent::new(Key::Ctrl)),
            Keycode::Left => Some(KeyEvent::with_mod(Key::CrsrRight, Key::LShift)),
            Keycode::LGui => Some(KeyEvent::new(Key::LGui)),
            Keycode::LShift => Some(KeyEvent::new(Key::LShift)),
            Keycode::Pause => Some(KeyEvent::new(Key::Pause)),
            Keycode::RCtrl => Some(KeyEvent::new(Key::Ctrl)),
            Keycode::Right => Some(KeyEvent::new(Key::CrsrRight)),
            Keycode::RShift => Some(KeyEvent::new(Key::RShift)),
            Keycode::Up => Some(KeyEvent::with_mod(Key::CrsrDown, Key::LShift)),
            // Function
            Keycode::F1 => Some(KeyEvent::new(Key::F1)),
            Keycode::F3 => Some(KeyEvent::new(Key::F3)),
            Keycode::F5 => Some(KeyEvent::new(Key::F5)),
            Keycode::F7 => Some(KeyEvent::new(Key::F7)),
            _ => None,
        }
    }
}
