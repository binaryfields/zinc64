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

use std::cell::RefCell;
use std::rc::Rc;

use config::Config;
use cpu::CpuIo;
use cpu::interrupt;
use log::LogLevel;
use mem::{Addressable, Memory};
use video::ColorRam;
use util::{Dimension, Rect};
use util::bit;

use super::RenderTarget;

// SPEC: The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64

// TODO vic: impl bad line logic
// TODO vic: implement sprites

#[derive(Copy, Clone)]
enum Mode {
    // (ECM/BMM/MCM=0/0/0)
    Text = 0x00,
    // (ECM/BMM/MCM=0/0/1)
    McText = 0x01,
    // (ECM/BMM/MCM=0/1/0)
    Bitmap = 0x02,
    // (ECM/BMM/MCM=0/1/1)
    McBitmap = 0x03,
    // (ECM/BMM/MCM=1/0/0)
    EcmText = 0x04,
    // (ECM/BMM/MCM=1/0/1)
    InvalidText = 0x05,
    // (ECM/BMM/MCM=1/1/0)
    InvalidBitmap1 = 0x06,
    // (ECM/BMM/MCM=1/1/1)
    InvalidBitmap2 = 0x07,
}

impl Mode {
    pub fn from(mode: u8) -> Mode {
        match mode {
            0x00 => Mode::Text,
            0x01 => Mode::McText,
            0x02 => Mode::Bitmap,
            0x03 => Mode::McBitmap,
            0x04 => Mode::EcmText,
            0x05 => Mode::InvalidText,
            0x06 => Mode::InvalidBitmap1,
            0x07 => Mode::InvalidBitmap2,
            _ => panic!("invalid mode {}", mode),
        }
    }

    pub fn value(&self) -> u8 {
        *self as u8
    }
}

#[derive(Copy, Clone)]
pub enum Reg {
    M0X = 0x00,
    M0Y = 0x01,
    M1X = 0x02,
    M1Y = 0x03,
    M2X = 0x04,
    M2Y = 0x05,
    M3X = 0x06,
    M3Y = 0x07,
    M4X = 0x08,
    M4Y = 0x09,
    M5X = 0x0a,
    M5Y = 0x0b,
    M6X = 0x0c,
    M6Y = 0x0d,
    M7X = 0x0e,
    M7Y = 0x0f,
    MX8 = 0x10,
    CR1 = 0x11,
    RASTER = 0x12,
    LPX = 0x13,
    LPY = 0x14,
    ME = 0x15,
    CR2 = 0x16,
    MYE = 0x17,
    MEMPTR = 0x18,
    IRR = 0x19,
    IMR = 0x1a,
    MDP = 0x1b,
    MMC = 0x1c,
    MXE = 0x1d,
    MM = 0x1e,
    MD = 0x1f,
    EC = 0x20,
    B0C = 0x21,
    B1C = 0x22,
    B2C = 0x23,
    B3C = 0x24,
    MM0 = 0x25,
    MM1 = 0x26,
    M0C = 0x27,
    M1C = 0x28,
    M2C = 0x29,
    M3C = 0x2a,
    M4C = 0x2b,
    M5C = 0x2c,
    M6C = 0x2d,
    M7C = 0x2e,
    IGNORE = 0xff,
}

impl Reg {
    pub fn from(reg: u8) -> Reg {
        match reg {
            0x00 => Reg::M0X,
            0x01 => Reg::M0Y,
            0x02 => Reg::M1X,
            0x03 => Reg::M1Y,
            0x04 => Reg::M2X,
            0x05 => Reg::M2Y,
            0x06 => Reg::M3X,
            0x07 => Reg::M3Y,
            0x08 => Reg::M4X,
            0x09 => Reg::M4Y,
            0x0a => Reg::M5X,
            0x0b => Reg::M5Y,
            0x0c => Reg::M6X,
            0x0d => Reg::M6Y,
            0x0e => Reg::M7X,
            0x0f => Reg::M7Y,
            0x10 => Reg::MX8,
            0x11 => Reg::CR1,
            0x12 => Reg::RASTER,
            0x13 => Reg::LPX,
            0x14 => Reg::LPY,
            0x15 => Reg::ME,
            0x16 => Reg::CR2,
            0x17 => Reg::MYE,
            0x18 => Reg::MEMPTR,
            0x19 => Reg::IRR,
            0x1a => Reg::IMR,
            0x1b => Reg::MDP,
            0x1c => Reg::MMC,
            0x1d => Reg::MXE,
            0x1e => Reg::MM,
            0x1f => Reg::MD,
            0x20 => Reg::EC,
            0x21 => Reg::B0C,
            0x22 => Reg::B1C,
            0x23 => Reg::B2C,
            0x24 => Reg::B3C,
            0x25 => Reg::MM0,
            0x26 => Reg::MM1,
            0x27 => Reg::M0C,
            0x28 => Reg::M1C,
            0x29 => Reg::M2C,
            0x2a => Reg::M3C,
            0x2b => Reg::M4C,
            0x2c => Reg::M5C,
            0x2d => Reg::M6C,
            0x2e => Reg::M7C,
            0x2f...0x3f => Reg::IGNORE,
            _ => panic!("invalid reg {}", reg),
        }
    }
}

#[derive(Copy, Clone)]
struct Sprite {
    enabled: bool,
    x: u16,
    y: u8,
    color: u8,
    expand_x: bool,
    expand_y: bool,
    multicolor: bool,
    priority: bool,
}

impl Sprite {
    pub fn new() -> Sprite {
        Sprite {
            enabled: false,
            x: 0,
            y: 0,
            color: 0,
            expand_x: false,
            expand_y: false,
            multicolor: false,
            priority: false,
        }
    }

    pub fn reset(&mut self) {
        self.enabled = false;
        self.x = 0;
        self.y = 0;
        self.color = 0;
        self.expand_x = false;
        self.expand_y = false;
        self.multicolor = false;
        self.priority = true;
    }
}

#[allow(dead_code)]
pub struct Vic {
    // Dependencies
    config: Config,
    color_ram: Rc<RefCell<ColorRam>>,
    cpu_io: Rc<RefCell<CpuIo>>,
    mem: Rc<RefCell<Memory>>,
    rt: Rc<RefCell<RenderTarget>>,
    // Configuration
    mode: Mode,
    den: bool,
    rsel: bool,
    csel: bool,
    scroll_x: u8,
    scroll_y: u8,
    // Dimensions
    graphics: Rect,
    screen: Rect,
    window: Rect,
    // Interrupts
    int_data: u8,
    int_mask: u8,
    raster_compare: u16,
    // Memory Pointers
    char_base: u16,
    video_matrix: u16,
    // Sprite and Color Data
    background_color: [u8; 4],
    border_color: u8,
    light_pen_pos: [u8; 2],
    sprites: [Sprite; 8],
    sprite_multicolor: [u8; 2],
    // Runtime State
    raster: u16,
    cycle: u16,
    vc_base: u16,
    vc: u16,
    rc: u8,
    sprite_ptrs: [u16; 8],
    sprite_mc: [u8; 8],
    sprite_mcbase: [u8; 8],
}

impl Vic {
    pub fn new(
        config: Config,
        color_ram: Rc<RefCell<ColorRam>>,
        cpu_io: Rc<RefCell<CpuIo>>,
        mem: Rc<RefCell<Memory>>,
        rt: Rc<RefCell<RenderTarget>>,
    ) -> Vic {
        let mut vic = Vic {
            config: config,
            color_ram: color_ram,
            cpu_io: cpu_io,
            mem: mem,
            rt: rt,
            mode: Mode::Text,
            den: false,
            rsel: false,
            csel: false,
            scroll_x: 0,
            scroll_y: 0,
            graphics: Rect::new(0, 0, 0, 0),
            screen: config.screen,
            window: Rect::new(0, 0, 0, 0),
            int_data: 0x00,
            int_mask: 0x00,
            raster_compare: 0x00,
            char_base: 0,
            video_matrix: 0,
            border_color: 0,
            background_color: [0, 0, 0, 0],
            light_pen_pos: [0; 2],
            sprites: [Sprite::new(); 8],
            sprite_multicolor: [0; 2],
            raster: 0,
            cycle: 0,
            vc_base: 0,
            vc: 0,
            rc: 0,
            sprite_ptrs: [0; 8],
            sprite_mc: [0; 8],
            sprite_mcbase: [0; 8],
        };
        vic.update_display_dims();
        vic
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Text;
        self.den = true;
        self.rsel = true;
        self.csel = true;
        self.scroll_x = 0;
        self.scroll_y = 3;
        self.int_data = 0;
        self.int_mask = 0;
        self.raster_compare = 0;
        self.char_base = 4096;
        self.video_matrix = 1024;
        self.border_color = 0x0e;
        self.background_color = [0x06, 0, 0, 0];
        self.light_pen_pos = [0; 2];
        for i in 0..8 {
            self.sprites[i].reset();
        }
        self.sprite_multicolor = [0; 2];
        self.raster = 0x0100;
        self.cycle = 0;
        self.vc_base = 0;
        self.vc = 0;
        self.rc = 0;
    }

    pub fn step(&mut self) {
        // Process interrupts
        let rst_int = match self.cycle {
            0 if self.raster != 0 && self.raster == self.raster_compare => true,
            1 if self.raster == 0 && self.raster == self.raster_compare => true,
            _ => false,
        };
        if rst_int {
            self.int_data |= 1 << 0;
            if (self.int_mask & self.int_data) != 0 {
                self.cpu_io.borrow_mut().irq.set(interrupt::Source::Vic);
            }
        }
        // Prepare sprite data
        if self.cycle == 0 {
            self.fetch_sprite_pointers();
            for i in 0..8 {
                if self.sprites[i].y as u16 == self.raster {
                    self.sprite_mc[i] = 0;
                }
            }
        }
        // 2. In the first phase of cycle 14 of each line, VC is loaded from VCBASE
        // (VCBASE->VC) and VMLI is cleared. f there is a Bad Line Condition in
        // this phase, RC is also reset to zero.
        if self.cycle == 14 {
            self.vc = self.vc_base;
            if self.is_bad_line(self.raster) {
                self.rc = 0;
            }
        }
        let x_pos = self.cycle << 3;
        let y_pos = self.raster;
        if self.screen.contains(x_pos, y_pos) {
            let x_screen = x_pos - self.screen.left;
            let y_screen = self.raster - self.screen.top;
            if self.graphics.contains(x_pos, y_pos) {
                if self.window.contains(x_screen, y_screen) {
                    self.draw(x_screen, y_screen, self.vc, self.rc);
                // 4. VC and VMLI are incremented after each g-access in display state.
                } else {
                    self.draw_border(x_screen, y_screen);
                }
                self.vc += 1;
            } else {
                self.draw_border(x_screen, y_screen);
            }
            // Draw Sprites
            if self.cycle == 58 {
                self.draw_sprites(y_pos);
            }
        }
        // 5. In the first phase of cycle 58, the VIC checks if RC=7. If so, the video
        // logic goes to idle state and VCBASE is loaded from VC (VC->VCBASE).
        if self.cycle == 58 {
            if self.rc == 7 {
                self.vc_base = self.vc;
            }
            self.rc += 1;
        }
        // Update counters/runtime state
        self.cycle += 1;
        if self.cycle >= self.config.raster_line_cycles {
            self.cycle = 0;
            self.raster += 1;
            if self.raster >= self.config.raster_size.height {
                self.raster = 0;
                // 1. Once somewhere outside of the range of raster lines $30-$f7, VCBASE is reset
                // to zero.
                self.vc_base = 0;
                let mut rt = self.rt.borrow_mut();
                rt.set_sync(true);
            }
        }
    }

    fn update_display_dims(&mut self) {
        self.graphics = self.config
            .graphics
            .offset(self.scroll_x as i16, self.scroll_y as i16);
        let window_x = if self.csel { 128 } else { 128 + 7 };
        let window_width = if self.csel { 320 } else { 304 };
        let window_y = if self.rsel { 51 } else { 55 };
        let window_height = if self.rsel { 200 } else { 192 };
        self.window = Rect::new_with_dim(
            window_x - self.screen.left,
            window_y - self.screen.top,
            Dimension::new(window_width, window_height),
        );
    }

    #[inline(always)]
    fn is_bad_line(&self, raster: u16) -> bool {
        if raster >= self.graphics.top {
            (raster - self.graphics.top) % 8 == 0
        } else {
            false
        }
    }

    // -- Draw Ops

    fn draw(&self, x: u16, y: u16, vc: u16, rc: u8) {
        match self.mode {
            Mode::Text => {
                let char_code = self.fetch_char_code(vc);
                let char_color = self.fetch_char_color(vc);
                let char_data = self.fetch_char_pixels(char_code, rc);
                self.draw_char_text(x, y, char_data, char_color);
            }
            Mode::McText => {
                let char_code = self.fetch_char_code(vc);
                let char_color = self.fetch_char_color(vc);
                let char_data = self.fetch_char_pixels(char_code, rc);
                if bit::test(char_color, 3) {
                    self.draw_char_mctext(x, y, char_data, char_color);
                } else {
                    self.draw_char_text(x, y, char_data, char_color);
                }
            }
            Mode::EcmText => {
                let c_data = self.fetch_char_code(vc);
                let char_code = c_data & 0x3f;
                let char_color_0_src = c_data >> 6;
                let char_color = self.fetch_char_color(vc);
                let char_data = self.fetch_char_pixels(char_code, rc);
                self.draw_char_ecm(x, y, char_data, char_color, char_color_0_src);
            }
            Mode::Bitmap => {
                let bitmap_color = self.fetch_bitmap_color(vc);
                let bitmap_data = self.fetch_bitmap_pixels(vc, rc);
                let color_1 = bitmap_color >> 4;
                let color_0 = bitmap_color & 0x0f;
                self.draw_bitmap(x, y, bitmap_data, color_1, color_0);
            }
            Mode::McBitmap => {
                let bitmap_color = self.fetch_bitmap_color(vc);
                let bitmap_data = self.fetch_bitmap_pixels(vc, rc);
                let color_01 = bitmap_color >> 4;
                let color_10 = bitmap_color & 0x0f;
                let color_11 = self.fetch_char_color(vc);
                self.draw_bitmap_mc(x, y, bitmap_data, color_01, color_10, color_11);
            }
            Mode::InvalidBitmap1 | Mode::InvalidBitmap2 => {
                self.draw_blank(x, y);
            }
            _ => panic!("unsupported graphics mode {}", self.mode.value()),
        }
    }

    fn draw_bitmap(&self, x: u16, y: u16, data: u8, color_1: u8, color_0: u8) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..8u16 {
            let x_pos = x + 7 - i;
            if x_pos < self.window.right {
                let color = if bit::test(data, i as u8) {
                    color_1
                } else {
                    color_0
                };
                rt.write(x_pos, y, color);
            } else {
                rt.write(x_pos, y, self.border_color);
            }
        }
    }

    fn draw_bitmap_mc(&self, x: u16, y: u16, data: u8, color_01: u8, color_10: u8, color_11: u8) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..4u16 {
            let x_pos = x + 6 - (i << 1);
            if x_pos <= self.config.window.right {
                let source = (data >> (i as u8 * 2)) & 0x03;
                let color = match source {
                    0 => self.background_color[0],
                    1 => color_01,
                    2 => color_10,
                    3 => color_11,
                    _ => panic!("invalid color source {}", source),
                };
                rt.write(x_pos, y, color);
                if x_pos + 1 <= self.config.window.right {
                    rt.write(x_pos + 1, y, color);
                }
            } else {
                rt.write(x_pos, y, self.border_color);
            }
        }
    }

    fn draw_blank(&self, x: u16, y: u16) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..8u16 {
            let x_pos = x + 7 - i;
            if x_pos < self.config.window.right {
                rt.write(x_pos, y, 0);
            }
        }
    }

    fn draw_border(&self, x: u16, y: u16) {
        if y < self.config.screen_size.height {
            let mut rt = self.rt.borrow_mut();
            for i in 0..8u16 {
                let x_pos = x + 7 - i;
                if x_pos < self.config.screen_size.width {
                    rt.write(x_pos, y, self.border_color);
                }
            }
        }
    }

    fn draw_char_text(&self, x: u16, y: u16, data: u8, color_1: u8) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..8u16 {
            let x_pos = x + 7 - i;
            if x_pos <= self.config.window.right {
                let color = if bit::test(data, i as u8) {
                    color_1
                } else {
                    self.background_color[0]
                };
                rt.write(x_pos, y, color);
            } else {
                rt.write(x_pos, y, self.border_color);
            }
        }
    }

    fn draw_char_ecm(&self, x: u16, y: u16, data: u8, color_1: u8, color_0_src: u8) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..8u16 {
            let x_pos = x + 7 - i;
            if x_pos <= self.config.window.right {
                let color = if bit::test(data, i as u8) {
                    color_1
                } else {
                    match color_0_src {
                        0 => self.background_color[0],
                        1 => self.background_color[1],
                        2 => self.background_color[2],
                        3 => self.background_color[3],
                        _ => panic!("invalid color source {}", color_0_src),
                    }
                };
                rt.write(x_pos, y, color);
            } else {
                rt.write(x_pos, y, self.border_color);
            }
        }
    }

    fn draw_char_mctext(&self, x: u16, y: u16, data: u8, color_1: u8) {
        let mut rt = self.rt.borrow_mut();
        for i in 0..4u16 {
            let x_pos = x + 6 - (i << 1);
            if x_pos <= self.config.window.right {
                let source = (data >> ((i as u8) << 1)) & 0x03;
                let color = match source {
                    0 => self.background_color[0],
                    1 => self.background_color[1],
                    2 => self.background_color[2],
                    3 => color_1 & 0x07,
                    _ => panic!("invalid color source {}", source),
                };
                rt.write(x_pos, y, color);
                if x_pos + 1 <= self.config.window.right {
                    rt.write(x_pos + 1, y, color);
                }
            } else {
                rt.write(x_pos, y, self.border_color);
            }
        }
    }

    fn draw_sprites(&mut self, raster: u16) {
        for i in 0..8 {
            let n = 7 - i;
            if self.sprites[n].enabled {
                if self.is_sprite(n, raster) {
                    for j in 0..3 {
                        let sp_data = self.fetch_sprite_pixels(n, self.sprite_mc[n]);
                        if !self.sprites[n].multicolor {
                            self.draw_sprite(
                                24 + self.sprites[n].x + (j << 3),
                                raster,
                                sp_data,
                                self.sprites[n].color,
                            );
                        } else {
                            self.draw_sprite_mc(
                                24 + self.sprites[n].x + (j << 3),
                                raster,
                                n,
                                sp_data,
                            );
                        }
                        self.sprite_mc[n] += 1;
                    }
                }
            }
        }
    }

    fn draw_sprite(&self, x: u16, y: u16, data: u8, color: u8) {
        let mut rt = self.rt.borrow_mut();
        let y_trans = y - self.config.screen.top;
        let x_trans = x;
        for i in 0..8u16 {
            if bit::test(data, i as u8) {
                rt.write(x_trans + 7 - i, y_trans, color);
            }
        }
    }

    fn draw_sprite_mc(&self, x: u16, y: u16, n: usize, data: u8) {
        let mut rt = self.rt.borrow_mut();
        let y_trans = y - self.config.screen.top;
        let x_trans = x;
        for i in 0..4u16 {
            let source = (data >> (i as u8 * 2)) & 0x03;
            let color = match source {
                0 => 0,
                1 => self.sprite_multicolor[0],
                2 => self.sprites[n].color,
                3 => self.sprite_multicolor[1],
                _ => panic!("invalid sprite color source {}", source),
            };
            if color != 0 {
                rt.write(x_trans + 7 - (i * 2), y_trans, color);
                rt.write(x_trans + 6 - (i * 2), y_trans, color);
            }
        }
    }

    // -- Memory Ops

    #[inline(always)]
    fn fetch_bitmap_color(&self, vc: u16) -> u8 {
        let address = self.video_matrix | vc;
        self.mem.borrow().vic_read(address)
    }

    #[inline(always)]
    fn fetch_bitmap_pixels(&self, vc: u16, rc: u8) -> u8 {
        let address = self.char_base & 0x2000 | (vc << 3) | rc as u16;
        self.mem.borrow().vic_read(address)
    }

    #[inline(always)]
    fn fetch_char_code(&self, vc: u16) -> u8 {
        let address = self.video_matrix | vc;
        self.mem.borrow().vic_read(address)
    }

    #[inline(always)]
    fn fetch_char_color(&self, vc: u16) -> u8 {
        self.color_ram.borrow().read(vc)
    }

    #[inline(always)]
    fn fetch_char_pixels(&self, ch: u8, rc: u8) -> u8 {
        let address = self.char_base | ((ch as u16) << 3) | rc as u16;
        self.mem.borrow().vic_read(address)
    }

    #[inline(always)]
    fn fetch_sprite_pointers(&mut self) {
        let mem = self.mem.borrow();
        for i in 0..8u16 {
            let address = self.video_matrix | 0x03f8 | i;
            self.sprite_ptrs[i as usize] = (mem.vic_read(address) as u16) << 6;
        }
    }

    #[inline(always)]
    fn fetch_sprite_pixels(&self, n: usize, mc: u8) -> u8 {
        let address = self.sprite_ptrs[n] | (mc as u16);
        self.mem.borrow().vic_read(address)
    }

    // -- Raster Queries

    #[inline(always)]
    fn is_sprite(&self, n: usize, y: u16) -> bool {
        let sprite = &self.sprites[n];
        if y >= (sprite.y as u16) && y < (sprite.y as u16 + 21) {
            true
        } else {
            false
        }
    }

    // -- Device I/O

    pub fn read(&mut self, reg: u8) -> u8 {
        let value = match Reg::from(reg) {
            Reg::M0X => (self.sprites[0].x & 0x00ff) as u8,
            Reg::M0Y => self.sprites[0].y,
            Reg::M1X => (self.sprites[1].x & 0x00ff) as u8,
            Reg::M1Y => self.sprites[1].y,
            Reg::M2X => (self.sprites[2].x & 0x00ff) as u8,
            Reg::M2Y => self.sprites[2].y,
            Reg::M3X => (self.sprites[3].x & 0x00ff) as u8,
            Reg::M3Y => self.sprites[3].y,
            Reg::M4X => (self.sprites[4].x & 0x00ff) as u8,
            Reg::M4Y => self.sprites[4].y,
            Reg::M5X => (self.sprites[5].x & 0x00ff) as u8,
            Reg::M5Y => self.sprites[5].y,
            Reg::M6X => (self.sprites[6].x & 0x00ff) as u8,
            Reg::M6Y => self.sprites[6].y,
            Reg::M7X => (self.sprites[7].x & 0x00ff) as u8,
            Reg::M7Y => self.sprites[7].y,
            Reg::MX8 => {
                let m0x8 = bit::get_u16(self.sprites[0].x, 8) << 0;
                let m1x8 = bit::get_u16(self.sprites[1].x, 8) << 1;
                let m2x8 = bit::get_u16(self.sprites[2].x, 8) << 2;
                let m3x8 = bit::get_u16(self.sprites[3].x, 8) << 3;
                let m4x8 = bit::get_u16(self.sprites[4].x, 8) << 4;
                let m5x8 = bit::get_u16(self.sprites[5].x, 8) << 5;
                let m6x8 = bit::get_u16(self.sprites[6].x, 8) << 6;
                let m7x8 = bit::get_u16(self.sprites[7].x, 8) << 7;
                m0x8 | m1x8 | m2x8 | m3x8 | m4x8 | m5x8 | m6x8 | m7x8
            }
            Reg::CR1 => {
                let rst8 = bit::get_u16(self.raster, 8) << 7;
                let ecm = bit::get(self.mode.value(), 2) << 6;
                let bmm = bit::get(self.mode.value(), 1) << 5;
                let den = bit::value(4, self.den);
                let rsel = bit::value(3, self.rsel);
                let yscroll = self.scroll_y & 0x07;
                rst8 | ecm | bmm | den | rsel | yscroll
            }
            Reg::RASTER => (self.raster & 0x00ff) as u8,
            Reg::LPX => self.light_pen_pos[0],
            Reg::LPY => self.light_pen_pos[1],
            Reg::ME => {
                let m0e = bit::value(0, self.sprites[0].enabled);
                let m1e = bit::value(1, self.sprites[1].enabled);
                let m2e = bit::value(2, self.sprites[2].enabled);
                let m3e = bit::value(3, self.sprites[3].enabled);
                let m4e = bit::value(4, self.sprites[4].enabled);
                let m5e = bit::value(5, self.sprites[5].enabled);
                let m6e = bit::value(6, self.sprites[6].enabled);
                let m7e = bit::value(7, self.sprites[7].enabled);
                m0e | m1e | m2e | m3e | m4e | m5e | m6e | m7e
            }
            Reg::CR2 => {
                let res = 1 << 5;
                let mcm = bit::get(self.mode.value(), 0) << 4;
                let csel = bit::value(3, self.csel);
                let yscroll = self.scroll_x & 0x07;
                res | mcm | csel | yscroll | 0xc0
            }
            Reg::MYE => {
                let m0ye = bit::value(0, self.sprites[0].expand_y);
                let m1ye = bit::value(1, self.sprites[1].expand_y);
                let m2ye = bit::value(2, self.sprites[2].expand_y);
                let m3ye = bit::value(3, self.sprites[3].expand_y);
                let m4ye = bit::value(4, self.sprites[4].expand_y);
                let m5ye = bit::value(5, self.sprites[5].expand_y);
                let m6ye = bit::value(6, self.sprites[6].expand_y);
                let m7ye = bit::value(7, self.sprites[7].expand_y);
                m0ye | m1ye | m2ye | m3ye | m4ye | m5ye | m6ye | m7ye
            }
            Reg::MEMPTR => {
                let vm = (((self.video_matrix & 0x3c00) >> 10) as u8) << 4;
                let cb = (((self.char_base & 0x3800) >> 11) as u8) << 1;
                vm | cb | 0x01
            }
            Reg::IRR => {
                let result = bit::set(self.int_data, 7, (self.int_mask & self.int_data) != 0);
                result | 0x70
            }
            Reg::IMR => self.int_mask | 0xf0,
            Reg::MDP => {
                let m0dp = bit::value(0, self.sprites[0].priority);
                let m1dp = bit::value(1, self.sprites[1].priority);
                let m2dp = bit::value(2, self.sprites[2].priority);
                let m3dp = bit::value(3, self.sprites[3].priority);
                let m4dp = bit::value(4, self.sprites[4].priority);
                let m5dp = bit::value(5, self.sprites[5].priority);
                let m6dp = bit::value(6, self.sprites[6].priority);
                let m7dp = bit::value(7, self.sprites[7].priority);
                m0dp | m1dp | m2dp | m3dp | m4dp | m5dp | m6dp | m7dp
            }
            Reg::MMC => {
                let m0mc = bit::value(0, self.sprites[0].multicolor);
                let m1mc = bit::value(1, self.sprites[1].multicolor);
                let m2mc = bit::value(2, self.sprites[2].multicolor);
                let m3mc = bit::value(3, self.sprites[3].multicolor);
                let m4mc = bit::value(4, self.sprites[4].multicolor);
                let m5mc = bit::value(5, self.sprites[5].multicolor);
                let m6mc = bit::value(6, self.sprites[6].multicolor);
                let m7mc = bit::value(7, self.sprites[7].multicolor);
                m0mc | m1mc | m2mc | m3mc | m4mc | m5mc | m6mc | m7mc
            }
            Reg::MXE => {
                let m0xe = bit::value(0, self.sprites[0].expand_x);
                let m1xe = bit::value(1, self.sprites[1].expand_x);
                let m2xe = bit::value(2, self.sprites[2].expand_x);
                let m3xe = bit::value(3, self.sprites[3].expand_x);
                let m4xe = bit::value(4, self.sprites[4].expand_x);
                let m5xe = bit::value(5, self.sprites[5].expand_x);
                let m6xe = bit::value(6, self.sprites[6].expand_x);
                let m7xe = bit::value(7, self.sprites[7].expand_x);
                m0xe | m1xe | m2xe | m3xe | m4xe | m5xe | m6xe | m7xe
            }
            Reg::MM => 0xff, // DEFERRED collision
            Reg::MD => 0xff, // DEFERRED collision
            Reg::EC => self.border_color | 0xf0,
            Reg::B0C => self.background_color[0] | 0xf0,
            Reg::B1C => self.background_color[1] | 0xf0,
            Reg::B2C => self.background_color[2] | 0xf0,
            Reg::B3C => self.background_color[3] | 0xf0,
            Reg::MM0 => self.sprite_multicolor[0] | 0xf0,
            Reg::MM1 => self.sprite_multicolor[1] | 0xf0,
            Reg::M0C => self.sprites[0].color | 0xf0,
            Reg::M1C => self.sprites[1].color | 0xf0,
            Reg::M2C => self.sprites[2].color | 0xf0,
            Reg::M3C => self.sprites[3].color | 0xf0,
            Reg::M4C => self.sprites[4].color | 0xf0,
            Reg::M5C => self.sprites[5].color | 0xf0,
            Reg::M6C => self.sprites[6].color | 0xf0,
            Reg::M7C => self.sprites[7].color | 0xf0,
            Reg::IGNORE => 0xff,
        };
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "vic::reg", "Read 0x{:02x} = 0x{:02x}", reg, value);
        }
        value
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "vic::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        match Reg::from(reg) {
            Reg::M0X => self.sprites[0].x = (self.sprites[0].x & 0xff00) | (value as u16),
            Reg::M0Y => self.sprites[0].y = value,
            Reg::M1X => self.sprites[1].x = (self.sprites[1].x & 0xff00) | (value as u16),
            Reg::M1Y => self.sprites[1].y = value,
            Reg::M2X => self.sprites[2].x = (self.sprites[2].x & 0xff00) | (value as u16),
            Reg::M2Y => self.sprites[2].y = value,
            Reg::M3X => self.sprites[3].x = (self.sprites[3].x & 0xff00) | (value as u16),
            Reg::M3Y => self.sprites[3].y = value,
            Reg::M4X => self.sprites[4].x = (self.sprites[4].x & 0xff00) | (value as u16),
            Reg::M4Y => self.sprites[4].y = value,
            Reg::M5X => self.sprites[5].x = (self.sprites[5].x & 0xff00) | (value as u16),
            Reg::M5Y => self.sprites[5].y = value,
            Reg::M6X => self.sprites[6].x = (self.sprites[6].x & 0xff00) | (value as u16),
            Reg::M6Y => self.sprites[6].y = value,
            Reg::M7X => self.sprites[7].x = (self.sprites[7].x & 0xff00) | (value as u16),
            Reg::M7Y => self.sprites[7].y = value,
            Reg::MX8 => {
                self.sprites[0].x = bit::set_u16(self.sprites[0].x, 8, bit::test(value, 0));
                self.sprites[1].x = bit::set_u16(self.sprites[1].x, 8, bit::test(value, 1));
                self.sprites[2].x = bit::set_u16(self.sprites[2].x, 8, bit::test(value, 2));
                self.sprites[3].x = bit::set_u16(self.sprites[3].x, 8, bit::test(value, 3));
                self.sprites[4].x = bit::set_u16(self.sprites[4].x, 8, bit::test(value, 4));
                self.sprites[5].x = bit::set_u16(self.sprites[5].x, 8, bit::test(value, 5));
                self.sprites[6].x = bit::set_u16(self.sprites[6].x, 8, bit::test(value, 6));
                self.sprites[7].x = bit::set_u16(self.sprites[7].x, 8, bit::test(value, 7));
            }
            Reg::CR1 => {
                self.raster_compare = bit::set_u16(self.raster_compare, 8, bit::test(value, 7));
                let mode = bit::set(self.mode.value(), 2, bit::test(value, 6));
                let mode2 = bit::set(mode, 1, bit::test(value, 5));
                self.mode = Mode::from(mode2);
                self.den = bit::test(value, 4);
                self.rsel = bit::test(value, 3);
                self.scroll_y = value & 0x07;
                self.update_display_dims();
            }
            Reg::RASTER => self.raster_compare = (self.raster_compare & 0xff00) | (value as u16),
            Reg::LPX => self.light_pen_pos[0] = value,
            Reg::LPY => self.light_pen_pos[1] = value,
            Reg::ME => {
                self.sprites[0].enabled = bit::test(value, 0);
                self.sprites[1].enabled = bit::test(value, 1);
                self.sprites[2].enabled = bit::test(value, 2);
                self.sprites[3].enabled = bit::test(value, 3);
                self.sprites[4].enabled = bit::test(value, 4);
                self.sprites[5].enabled = bit::test(value, 5);
                self.sprites[6].enabled = bit::test(value, 6);
                self.sprites[7].enabled = bit::test(value, 7);
            }
            Reg::CR2 => {
                let mode = bit::set(self.mode.value(), 0, bit::test(value, 4));
                self.mode = Mode::from(mode);
                self.csel = bit::test(value, 3);
                self.scroll_x = value & 0x07;
                self.update_display_dims();
            }
            Reg::MYE => {
                self.sprites[0].expand_y = bit::test(value, 0);
                self.sprites[1].expand_y = bit::test(value, 1);
                self.sprites[2].expand_y = bit::test(value, 2);
                self.sprites[3].expand_y = bit::test(value, 3);
                self.sprites[4].expand_y = bit::test(value, 4);
                self.sprites[5].expand_y = bit::test(value, 5);
                self.sprites[6].expand_y = bit::test(value, 6);
                self.sprites[7].expand_y = bit::test(value, 7);
            }
            Reg::MEMPTR => {
                self.video_matrix = (((value & 0xf0) >> 4) as u16) << 10;
                self.char_base = (((value & 0x0f) >> 1) as u16) << 11;
            }
            Reg::IRR => {
                self.int_data &= !value;
                if (self.int_mask & self.int_data) == 0 {
                    self.cpu_io.borrow_mut().irq.clear(interrupt::Source::Vic);
                }
            }
            Reg::IMR => {
                self.int_mask = value & 0x0f;
                if (self.int_mask & self.int_data) != 0 {
                    self.cpu_io.borrow_mut().irq.set(interrupt::Source::Vic);
                }
            }
            Reg::MDP => {
                self.sprites[0].priority = bit::test(value, 0);
                self.sprites[1].priority = bit::test(value, 1);
                self.sprites[2].priority = bit::test(value, 2);
                self.sprites[3].priority = bit::test(value, 3);
                self.sprites[4].priority = bit::test(value, 4);
                self.sprites[5].priority = bit::test(value, 5);
                self.sprites[6].priority = bit::test(value, 6);
                self.sprites[7].priority = bit::test(value, 7);
            }
            Reg::MMC => {
                self.sprites[0].multicolor = bit::test(value, 0);
                self.sprites[1].multicolor = bit::test(value, 1);
                self.sprites[2].multicolor = bit::test(value, 2);
                self.sprites[3].multicolor = bit::test(value, 3);
                self.sprites[4].multicolor = bit::test(value, 4);
                self.sprites[5].multicolor = bit::test(value, 5);
                self.sprites[6].multicolor = bit::test(value, 6);
                self.sprites[7].multicolor = bit::test(value, 7);
            }
            Reg::MXE => {
                self.sprites[0].expand_x = bit::test(value, 0);
                self.sprites[1].expand_x = bit::test(value, 1);
                self.sprites[2].expand_x = bit::test(value, 2);
                self.sprites[3].expand_x = bit::test(value, 3);
                self.sprites[4].expand_x = bit::test(value, 4);
                self.sprites[5].expand_x = bit::test(value, 5);
                self.sprites[6].expand_x = bit::test(value, 6);
                self.sprites[7].expand_x = bit::test(value, 7);
            }
            Reg::MM => {}
            Reg::MD => {}
            Reg::EC => self.border_color = value & 0x0f,
            Reg::B0C => self.background_color[0] = value & 0x0f,
            Reg::B1C => self.background_color[1] = value & 0x0f,
            Reg::B2C => self.background_color[2] = value & 0x0f,
            Reg::B3C => self.background_color[3] = value & 0x0f,
            Reg::MM0 => self.sprite_multicolor[0] = value & 0x0f,
            Reg::MM1 => self.sprite_multicolor[1] = value & 0x0f,
            Reg::M0C => self.sprites[0].color = value & 0x0f,
            Reg::M1C => self.sprites[1].color = value & 0x0f,
            Reg::M2C => self.sprites[2].color = value & 0x0f,
            Reg::M3C => self.sprites[3].color = value & 0x0f,
            Reg::M4C => self.sprites[4].color = value & 0x0f,
            Reg::M5C => self.sprites[5].color = value & 0x0f,
            Reg::M6C => self.sprites[6].color = value & 0x0f,
            Reg::M7C => self.sprites[7].color = value & 0x0f,
            Reg::IGNORE => {}
        }
    }
}
