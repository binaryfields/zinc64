/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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
use std::cmp;
use std::rc::Rc;

use bit_field::BitField;
use core::{Chip, FrameBuffer, IrqControl, IrqLine, Pin, Ram, VicModel};
use log::LogLevel;

use super::VicMemory;
use super::rect::{Dimension, Rect};

// SPEC: The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64

// TODO vic:
// 1 display/idle states cycle 58
// 2 rsel/csel
// 3 scroll_x/y
// 4 sprites

#[derive(Copy, Clone)]
pub enum IrqSource {
    Vic = 2,
}

impl IrqSource {
    pub fn value(&self) -> usize {
        *self as usize
    }
}

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

pub struct Spec {
    pub raster_lines: u16,
    pub cycles_per_raster: u16,
    pub viewport: Rect,
    pub viewport_size: Dimension,
}

/*
          | Video  | # of  | Visible | Cycles/ |  Visible
   Type   | system | lines |  lines  |  line   | pixels/line
 ---------+--------+-------+---------+---------+------------
 6567R56A | NTSC-M |  262  |   234   |   64    |    411
  6567R8  | NTSC-M |  263  |   235   |   65    |    418
   6569   |  PAL-B |  312  |   284   |   63    |    403

          | First  |  Last  |              |   First    |   Last
          | vblank | vblank | First X coo. |  visible   |  visible
   Type   |  line  |  line  |  of a line   |   X coo.   |   X coo.
 ---------+--------+--------+--------------+------------+-----------
 6567R56A |   13   |   40   |  412 ($19c)  | 488 ($1e8) | 388 ($184)
  6567R8  |   13   |   40   |  412 ($19c)  | 489 ($1e9) | 396 ($18c)
   6569   |  300   |   15   |  404 ($194)  | 480 ($1e0) | 380 ($17c)
*/

impl Spec {
    pub fn new(chip_model: VicModel) -> Spec {
        match chip_model {
            VicModel::Mos6567 => Spec::ntsc(),
            VicModel::Mos6569 => Spec::pal(),
        }
    }

    fn ntsc() -> Spec {
        let viewport_size = Dimension::new(403, 284);
        Spec {
            raster_lines: 278,
            cycles_per_raster: 65,
            viewport: Rect::new_with_dim(76 - 4, 16, viewport_size),
            viewport_size,
        }
    }

    fn pal() -> Spec {
        let viewport_size = Dimension::new(403, 284);
        Spec {
            raster_lines: 312,
            cycles_per_raster: 63,
            viewport: Rect::new_with_dim(76 - 4, 16, viewport_size),
            viewport_size,
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

pub struct Vic {
    // Dependencies
    spec: Spec,
    ba_line: Rc<RefCell<Pin>>,
    color_ram: Rc<RefCell<Ram>>,
    irq_line: Rc<RefCell<IrqLine>>,
    frame_buffer: Rc<RefCell<FrameBuffer>>,
    mem: Rc<RefCell<VicMemory>>,
    // Control
    mode: Mode,
    csel: bool,
    den: bool,
    rsel: bool,
    raster_compare: u16,
    x_scroll: u8,
    y_scroll: u8,
    // Interrupts
    interrupt_control: IrqControl,
    // Memory Pointers
    char_base: u16,
    video_matrix: u16,
    // Sprite and Color Data
    background_color: [u8; 4],
    border_color: u8,
    light_pen_pos: [u8; 2],
    sprite_multicolor: [u8; 2],
    sprites: [Sprite; 8],
    // Registers
    cycle: u16,
    raster: u16,
    rc: u8,
    vc_base: u16,
    vc: u16,
    vmli: usize,
    // Runtime State
    border_flip: bool,
    border_vflip: bool,
    display_on: bool,
    display_state: bool,
    c_data: u8,
    c_color: u8,
    g_data: u8,
    is_bad_line: bool,
    sprite_ptrs: [u16; 8],
    #[allow(dead_code)] sprite_mc: [u8; 8],
    #[allow(dead_code)] sprite_mcbase: [u8; 8],
    vm_color_line: [u8; 40],
    vm_data_line: [u8; 40],
}

impl Vic {
    pub fn new(
        chip_model: VicModel,
        ba_line: Rc<RefCell<Pin>>,
        color_ram: Rc<RefCell<Ram>>,
        irq_line: Rc<RefCell<IrqLine>>,
        frame_buffer: Rc<RefCell<FrameBuffer>>,
        mem: Rc<RefCell<VicMemory>>,
    ) -> Vic {
        info!(target: "video", "Initializing VIC");
        let spec = Spec::new(chip_model);
        let vic = Vic {
            // Dependencies
            spec,
            ba_line,
            color_ram,
            irq_line,
            mem,
            frame_buffer,
            // Control
            mode: Mode::Text,
            csel: false,
            den: false,
            rsel: false,
            raster_compare: 0x00,
            x_scroll: 0,
            y_scroll: 0,
            // Interrupts
            interrupt_control: IrqControl::new(),
            // Memory Pointers
            char_base: 0,
            video_matrix: 0,
            // Sprite and Color Data
            background_color: [0; 4],
            border_color: 0,
            light_pen_pos: [0; 2],
            sprites: [Sprite::new(); 8],
            sprite_multicolor: [0; 2],
            // Registers
            cycle: 1,
            raster: 0,
            rc: 0,
            vc_base: 0,
            vc: 0,
            vmli: 0,
            // Runtime State
            border_flip: false,
            border_vflip: false,
            display_on: false,
            display_state: false,
            c_data: 0,
            c_color: 0,
            g_data: 0,
            is_bad_line: false,
            sprite_ptrs: [0; 8],
            sprite_mc: [0; 8],
            sprite_mcbase: [0; 8],
            vm_color_line: [0; 40],
            vm_data_line: [0; 40],
        };
        vic
    }

    #[inline]
    fn trigger_irq(&mut self, source: usize) {
        self.interrupt_control.set_event(source);
        if self.interrupt_control.is_triggered() {
            if log_enabled!(LogLevel::Trace) {
                trace!(target: "vic::reg", "Irq data = {:02x}, mask = {:02x}, source: {}",
                       self.interrupt_control.get_data(),
                       self.interrupt_control.get_mask(),
                       source
                );
            }
            self.irq_line
                .borrow_mut()
                .set_low(IrqSource::Vic.value(), true);
        }
    }

    #[inline]
    fn set_ba(&mut self, is_bad_line: bool) {
        self.ba_line.borrow_mut().set_active(!is_bad_line);
    }

    // -- Coordinates Mapping

    #[inline]
    fn get_coords(&self) -> (u16, u16) {
        ((self.cycle - 1) << 3, self.raster)
    }

    #[inline]
    fn get_viewport_coords(&self) -> Option<(u16, u16)> {
        let (x, y) = self.get_coords();
        if self.spec.viewport.contains(x, y) {
            Some((x - self.spec.viewport.left, y - self.spec.viewport.top))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn map_sprite_to_screen(&self, x: u16) -> u16 {
        let x_trans = match x {
            0x000...0x193 => x + 0x64,
            0x194...0x1ff => x - 0x194,
            _ => panic!("invalid sprite coords {}", x),
        };
        x_trans
    }

    #[allow(dead_code)]
    #[inline]
    fn map_screen_to_viewport(&self, x: u16, y: u16) -> Option<(u16, u16)> {
        if self.spec.viewport.contains(x, y) {
            Some((x - self.spec.viewport.left, y - self.spec.viewport.top))
        } else {
            None
        }
    }

    // -- Graphics Ops

    #[inline]
    fn draw(&self) {
        // "The sequencer outputs the graphics data in every raster line in the area of
        //  the display column as long as the vertical border flip-flop is reset"
        if !self.border_vflip {
            self.draw_graphics();
        } else {
            self.draw_border();
        }
    }

    #[inline]
    fn draw_border(&self) {
        if let Some((x_start, y)) = self.get_viewport_coords() {
            let mut rt = self.frame_buffer.borrow_mut();
            for x in x_start..cmp::min(x_start + 8, self.spec.viewport_size.width) {
                rt.write(x, y, self.border_color);
            }
        }
    }

    #[inline]
    fn draw_graphics(&self) {
        if let Some((x, y)) = self.get_viewport_coords() {
            match self.mode {
                Mode::Text => {
                    self.draw_text(x, y, self.g_data, self.c_color);
                }
                Mode::McText => {
                    if self.c_color.get_bit(3) {
                        self.draw_text_mc(x, y, self.g_data, self.c_color);
                    } else {
                        self.draw_text(x, y, self.g_data, self.c_color);
                    }
                }
                Mode::Bitmap => {
                    let color_1 = self.c_data >> 4;
                    let color_0 = self.c_data & 0x0f;
                    self.draw_bitmap(x, y, self.g_data, color_1, color_0);
                }
                Mode::McBitmap => {
                    let color_01 = self.c_data >> 4;
                    let color_10 = self.c_data & 0x0f;
                    let color_11 = self.c_color;
                    self.draw_bitmap_mc(x, y, self.g_data, color_01, color_10, color_11);
                }
                Mode::EcmText => {
                    self.draw_text_ecm(x, y, self.g_data, self.c_color, self.c_data >> 6);
                }
                Mode::InvalidBitmap1 | Mode::InvalidBitmap2 => {
                    self.draw_blank(x, y);
                }
                _ => panic!("unsupported graphics mode {}", self.mode.value()),
            }
        }
    }

    #[inline]
    fn draw_blank(&self, x_start: u16, y: u16) {
        let mut rt = self.frame_buffer.borrow_mut();
        for x in x_start..x_start + 8 {
            rt.write(x, y, 0);
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Color from bits 0-3 of c-data    |
     | "1": Color from bits 4-7 of c-data    |
     +---------------------------------------+
    */

    #[inline]
    fn draw_bitmap(&self, x_start: u16, y: u16, pixels: u8, color_1: u8, color_0: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        let mut data = pixels;
        for x in x_start..x_start + 8 {
            let color = if data.get_bit(7) { color_1 } else { color_0 };
            rt.write(x, y, color);
            data = data << 1;
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      |
     | "01": Color from bits 4-7 of c-data   |
     | "10": Color from bits 0-3 of c-data   |
     | "11": Color from bits 8-11 of c-data  |
     +---------------------------------------+
    */

    #[inline]
    fn draw_bitmap_mc(
        &self,
        x_start: u16,
        y: u16,
        pixels: u8,
        color_01: u8,
        color_10: u8,
        color_11: u8,
    ) {
        let mut rt = self.frame_buffer.borrow_mut();
        let mut data = pixels;
        let mut x = x_start;
        let x_end = x_start + 8;
        while x < x_end {
            let color = match data >> 6 {
                0 => self.background_color[0],
                1 => color_01,
                2 => color_10,
                3 => color_11,
                _ => panic!("invalid color source {}", data >> 6),
            };
            rt.write(x, y, color);
            rt.write(x + 1, y, color);
            data = data << 2;
            x += 2;
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    #[inline]
    fn draw_text(&self, x_start: u16, y: u16, pixels: u8, color_1: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        let mut data = pixels;
        for x in x_start..x_start + 8 {
            let color = if data.get_bit(7) {
                color_1
            } else {
                self.background_color[0]
            };
            rt.write(x, y, color);
            data = data << 1;
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Depending on bits 6/7 of c-data  |
     |      00: Background color 0 ($d021)   |
     |      01: Background color 1 ($d022)   |
     |      10: Background color 2 ($d023)   |
     |      11: Background color 3 ($d024)   |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    #[inline]
    fn draw_text_ecm(&self, x_start: u16, y: u16, pixels: u8, color_1: u8, color_0_src: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        let mut data = pixels;
        for x in x_start..x_start + 8 {
            let color = if data.get_bit(7) {
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
            rt.write(x, y, color);
            data = data << 1;
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       | MC flag = 0
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-10 of c-data   |
     +---------------------------------------+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      | MC flag = 1
     | "01": Background color 1 ($d022)      |
     | "10": Background color 2 ($d023)      |
     | "11": Color from bits 8-10 of c-data  |
     +---------------------------------------+
    */

    #[inline]
    fn draw_text_mc(&self, x_start: u16, y: u16, pixels: u8, color_1: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        let mut data = pixels;
        let mut x = x_start;
        let x_end = x_start + 8;
        while x < x_end {
            let color = match data >> 6 {
                0 => self.background_color[0],
                1 => self.background_color[1],
                2 => self.background_color[2],
                3 => color_1 & 0x07,
                _ => panic!("invalid color source {}", data >> 6),
            };
            rt.write(x, y, color);
            rt.write(x + 1, y, color);
            data = data << 2;
            x += 2;
        }
    }

    // -- Sprite Ops

    #[inline]
    fn draw_sprites(&mut self, raster: u16) {
        for i in 0..8 {
            let n = 7 - i;
            if self.sprites[n].enabled {
                if self.is_sprite(n, raster) {
                    for j in 0..3 {
                        let sp_data = self.fetch_sprite_pixels(n, self.sprite_mc[n]);
                        if !self.sprites[n].multicolor {
                            self.draw_sprite(
                                self.map_sprite_to_screen(self.sprites[n].x) + (j << 3),
                                raster,
                                sp_data,
                                self.sprites[n].color,
                            );
                        } else {
                            self.draw_sprite_mc(
                                self.map_sprite_to_screen(self.sprites[n].x) + (j << 3),
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

    #[inline]
    fn draw_sprite(&self, x: u16, y: u16, data: u8, color: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        if let Some((x_trans, y_trans)) = self.map_screen_to_viewport(x, y) {
            for i in 0..8u16 {
                if data.get_bit(i as usize) {
                    rt.write(x_trans + 7 - i, y_trans, color);
                }
            }
        }
    }

    #[inline]
    fn draw_sprite_mc(&self, x: u16, y: u16, n: usize, data: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
        if let Some((x_trans, y_trans)) = self.map_screen_to_viewport(x, y) {
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
    }

    #[inline]
    fn fetch_sprite_pixels(&self, n: usize, mc: u8) -> u8 {
        let address = self.sprite_ptrs[n] | (mc as u16);
        self.mem.borrow().read(address)
    }

    #[inline]
    fn is_sprite(&self, n: usize, y: u16) -> bool {
        let sprite = &self.sprites[n];
        if y >= (sprite.y as u16) && y < (sprite.y as u16 + 21) {
            true
        } else {
            false
        }
    }

    // -- Memory Ops

    #[inline]
    fn c_access(&mut self) {
        if self.is_bad_line {
            let address = self.video_matrix | self.vc;
            self.vm_data_line[self.vmli] = self.mem.borrow().read(address);
            self.vm_color_line[self.vmli] = self.color_ram.borrow().read(self.vc) & 0x0f;
            // TODO vic: no access unless ba down for 3 cycles
        }
    }

    #[inline]
    fn g_access(&mut self) {
        self.g_data = match self.mode {
            Mode::Text | Mode::McText => {
                let address =
                    self.char_base | ((self.vm_data_line[self.vmli] as u16) << 3) | self.rc as u16;
                self.mem.borrow().read(address)
            }
            Mode::EcmText => {
                let address = self.char_base | (((self.vm_data_line[self.vmli] & 0x3f) as u16) << 3)
                    | self.rc as u16;
                self.mem.borrow().read(address)
            }
            Mode::Bitmap | Mode::McBitmap => {
                let address = self.char_base & 0x2000 | (self.vc << 3) | self.rc as u16;
                self.mem.borrow().read(address)
            }
            Mode::InvalidBitmap1 | Mode::InvalidBitmap2 => 0,
            _ => panic!("unsupported graphics mode {}", self.mode.value()),
        };
        self.c_data = self.vm_data_line[self.vmli];
        self.c_color = self.vm_color_line[self.vmli];
        // "4. VC and VMLI are incremented after each g-access in display state."
        self.vc += 1;
        self.vmli += 1;
    }

    #[inline]
    fn p_access(&mut self, n: usize) {
        let address = self.video_matrix | 0x03f8 | n as u16;
        self.sprite_ptrs[n] = (self.mem.borrow().read(address) as u16) << 6;
    }

    #[allow(dead_code)]
    #[inline]
    fn s_access(&self, n: usize, mc: u8) -> u8 {
        let address = self.sprite_ptrs[n] | (mc as u16);
        self.mem.borrow().read(address)
    }

    // -- Raster Queries

    /*
     "A Bad Line Condition is given at any arbitrary clock cycle, if at the
      negative edge of ø0 at the beginning of the cycle RASTER >= $30 and RASTER
      <= $f7 and the lower three bits of RASTER are equal to YSCROLL and if the
      DEN bit was set during an arbitrary cycle of raster line $30."
    */

    #[inline]
    fn update_bad_line(&mut self) {
        self.is_bad_line = match self.raster {
            0x30...0xf7 => (self.raster & 0x07) as u8 == self.y_scroll && self.display_on,
            _ => false,
        };
    }

    /*
           |   CSEL=0   |   CSEL=1
     ------+------------+-----------
     Left  |  31 ($1f)  |  24 ($18)
     Right | 335 ($14f) | 344 ($158)

            |   RSEL=0  |  RSEL=1
     -------+-----------+----------
     Top    |  55 ($37) |  51 ($33)
     Bottom | 247 ($f7) | 251 ($fb)
    */

    #[inline]
    fn update_border_vflip(&mut self) {
        self.border_vflip = if self.rsel {
            match self.raster {
                51 if self.den => false,
                251 => true,
                _ => self.border_vflip,
            }
        } else {
            match self.raster {
                55 if self.den => false,
                247 => true,
                _ => self.border_vflip,
            }
        }
    }
}

/*
6569, Bad Line, no sprites:

Cycl-# 6                   1 1 1 1 1 1 1 1 1 1 2 2 2 2 2 2 2 2 2 2 3 3 3 3 3 3 3 3 3 3 4 4 4 4 4 4 4 4 4 4 5 5 5 5 5 5 5 5 5 5 6 6 6 6
       3 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 1
        _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
    ø0 _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _
       __
   IRQ   ________________________________________________________________________________________________________________________________
       ________________________                                                                                      ____________________
    BA                         ______________________________________________________________________________________
        _ _ _ _ _ _ _ _ _ _ _ _ _ _ _                                                                                 _ _ _ _ _ _ _ _ _ _
   AEC _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _________________________________________________________________________________ _ _ _ _ _ _ _ _ _

   VIC i 3 i 4 i 5 i 6 i 7 i r r r r rcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcgcg i i 0 i 1 i 2 i 3
  6510  x x x x x x x x x x x x X X X                                                                                 x x x x x x x x x x

Graph.                      |===========01020304050607080910111213141516171819202122232425262728293031323334353637383940=========

X coo. \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
       1111111111111111111111111110000000000000000000000000000000000000000000000000000000000000000111111111111111111111111111111111111111
       89999aaaabbbbccccddddeeeeff0000111122223333444455556666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555666677778888999
       c048c048c048c048c048c048c04048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048c048

*/

impl Chip for Vic {
    fn clock(&mut self) {
        match self.cycle {
            1 => {
                if self.raster == self.raster_compare && self.raster != 0 {
                    self.trigger_irq(0);
                }
                self.update_bad_line();
                self.set_ba(false);
                self.p_access(3);
            }
            2 => {
                // TODO vic: cycle 2 logic
                if self.raster == self.raster_compare && self.raster == 0 {
                    self.trigger_irq(0);
                }
                self.set_ba(false);
            }
            3 => {
                self.set_ba(false);
                self.p_access(4);
            }
            4 => {
                self.set_ba(false);
            }
            5 => {
                self.set_ba(false);
                self.p_access(5);
            }
            6 => {
                self.set_ba(false);
            }
            7 => {
                self.set_ba(false);
                self.p_access(6);
            }
            8 => {
                self.set_ba(false);
            }
            9 => {
                self.set_ba(false);
                self.p_access(7);
            }
            10 => {
                self.set_ba(false);
                self.draw_border();
            }
            11 => {
                self.set_ba(false);
                self.draw_border();
            }
            12...13 => {
                // "3. If there is a Bad Line Condition in cycles 12-54, BA is set low and the
                //     c-accesses are started. Once started, one c-access is done in the second
                //     phase of every clock cycle in the range 15-54."
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.draw_border();
            }
            14 => {
                // "2. In the first phase of cycle 14 of each line, VC is loaded from VCBASE
                //     (VCBASE->VC) and VMLI is cleared. If there is a Bad Line Condition in
                //     this phase, RC is also reset to zero."
                self.vc = self.vc_base;
                self.vmli = 0;
                if self.is_bad_line {
                    self.rc = 0;
                }
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.draw_border();
            }
            15 => {
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.c_access();
                self.draw_border();
            }
            // Display Column
            16 => {
                self.update_border_vflip();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
                self.draw();
            }
            17...54 => {
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
                self.draw();
            }
            55 => {
                self.set_ba(false);
                self.g_access();
                self.draw();
            }
            // Display Column End
            56 => {
                self.set_ba(false);
                self.draw_border();
            }
            57 => {
                self.set_ba(false);
                self.draw_border();
            }
            58 => {
                // TODO vic: cycle 58 display logic
                // "5. In the first phase of cycle 58, the VIC checks if RC=7. If so, the video
                //    logic goes to idle state and VCBASE is loaded from VC (VC->VCBASE). If
                //    the video logic is in display state afterwards (this is always the case
                //    if there is a Bad Line Condition), RC is incremented."
                if self.rc == 7 {
                    self.vc_base = self.vc;
                }
                self.rc += 1;
                self.set_ba(false);
                self.p_access(0);
                self.draw_border();
            }
            59 => {
                self.set_ba(false);
                self.draw_border();
            }
            60 => {
                self.set_ba(false);
                self.p_access(1);
                self.draw_border();
            }
            61 => {
                self.set_ba(false);
            }
            62 => {
                self.set_ba(false);
                self.p_access(2);
            }
            63 => {
                self.set_ba(false);
                self.update_border_vflip();
                for i in 0..8 {
                    if self.sprites[i].y as u16 == self.raster {
                        self.sprite_mc[i] = 0;
                    }
                }
                let raster = self.raster;
                self.draw_sprites(raster);
            }
            _ => panic!("invalid cycle"),
        }
        if self.raster == 0x30 && self.den {
            self.display_on = true;
        }
        // Update counters/vsync
        self.cycle += 1;
        if self.cycle > self.spec.cycles_per_raster {
            self.cycle = 1;
            self.raster += 1;
            if self.raster >= self.spec.raster_lines {
                self.raster = 0;
                // 1. VCBASE is reset to zero in raster line 0.
                self.vc_base = 0;
                let mut rt = self.frame_buffer.borrow_mut();
                rt.set_sync(true);
            }
        }
    }

    fn clock_delta(&mut self, delta: u32) {
        for _i in 0..delta {
            self.clock();
        }
    }

    fn process_vsync(&mut self) {}

    fn reset(&mut self) {
        // Control
        self.mode = Mode::Text;
        self.csel = true;
        self.den = true;
        self.rsel = true;
        self.raster_compare = 0;
        self.x_scroll = 0;
        self.y_scroll = 3;
        // Interrupts
        self.interrupt_control.reset();
        // Memory Pointers
        self.char_base = 0x1000;
        self.video_matrix = 0x0400;
        // Sprite and Color Data
        self.background_color = [0x06, 0, 0, 0];
        self.border_color = 0x0e;
        self.light_pen_pos = [0; 2];
        self.sprite_multicolor = [0; 2];
        for i in 0..8 {
            self.sprites[i].reset();
        }
        // Registers
        self.cycle = 1;
        self.raster = 0x0100;
        self.rc = 0;
        self.vc_base = 0;
        self.vc = 0;
        self.vmli = 0;
        // Runtime State
        self.border_flip = false;
        self.border_vflip = false;
        self.display_on = false;
        self.display_state = false;
        self.c_data = 0;
        self.c_color = 0;
        self.g_data = 0;
        self.is_bad_line = false;
        // TODO vic: reset sprite data
        for i in 0..self.vm_data_line.len() {
            self.vm_color_line[i] = 0;
            self.vm_data_line[i] = 0;
        }
    }

    // I/O

    fn read(&mut self, reg: u8) -> u8 {
        let value = match reg {
            // Reg::M0X - Reg::M7X
            0x00 | 0x02 | 0x04 | 0x06 | 0x08 | 0x0a | 0x0c | 0x0e => {
                (self.sprites[(reg >> 1) as usize].x & 0x00ff) as u8
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                self.sprites[((reg - 1) >> 1) as usize].y
            }
            // Reg::MX8
            0x10 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].x.get_bit(8));
                }
                result
            }
            // Reg::CR1
            0x11 => {
                let mut result = 0;
                result
                    .set_bit(7, self.raster.get_bit(8))
                    .set_bit(6, self.mode.value().get_bit(2))
                    .set_bit(5, self.mode.value().get_bit(1))
                    .set_bit(4, self.den)
                    .set_bit(3, self.rsel);
                result | (self.y_scroll & 0x07)
            }
            // Reg::RASTER
            0x12 => (self.raster & 0x00ff) as u8,
            // Reg::LPX
            0x13 => self.light_pen_pos[0],
            // Reg::LPY
            0x14 => self.light_pen_pos[1],
            // Reg::ME
            0x15 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].enabled);
                }
                result
            }
            // Reg::CR2
            0x16 => {
                let mut result = 0;
                result
                    .set_bit(5, true)
                    .set_bit(4, self.mode.value().get_bit(0))
                    .set_bit(3, self.csel);
                result | (self.x_scroll & 0x07) | 0xc0
            }
            // Reg::MYE
            0x17 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].expand_y);
                }
                result
            }
            // Reg::MEMPTR
            0x18 => {
                let vm = (((self.video_matrix & 0x3c00) >> 10) as u8) << 4;
                let cb = (((self.char_base & 0x3800) >> 11) as u8) << 1;
                vm | cb | 0x01
            }
            // Reg::IRR
            0x19 => self.interrupt_control.get_data() | 0x70,
            // Reg::IMR
            0x1a => self.interrupt_control.get_mask() | 0xf0,
            // Reg::MDP
            0x1b => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].priority);
                }
                result
            }
            // Reg::MMC
            0x1c => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].multicolor);
                }
                result
            }
            // Reg::MXE
            0x1d => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].expand_x);
                }
                result
            }
            // Reg::MM
            0x1e => 0xff, // DEFERRED collision
            // Reg::MD
            0x1f => 0xff, // DEFERRED collision
            // Reg::EC
            0x20 => self.border_color | 0xf0,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.background_color[(reg - 0x21) as usize] | 0xf0,
            // Reg::MM0 - Reg::MM1
            0x25...0x26 => self.sprite_multicolor[(reg - 0x25) as usize] | 0xf0,
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprites[(reg - 0x27) as usize].color | 0xf0,
            _ => 0xff,
        };
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "vic::reg", "Read 0x{:02x} = 0x{:02x}", reg, value);
        }
        value
    }

    fn write(&mut self, reg: u8, value: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "vic::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        match reg {
            // Reg::M0X - Reg::M7X
            0x00 | 0x02 | 0x04 | 0x06 | 0x08 | 0x0a | 0x0c | 0x0e => {
                let n = (reg >> 1) as usize;
                self.sprites[n].x = (self.sprites[n].x & 0xff00) | (value as u16)
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                let n = ((reg - 1) >> 1) as usize;
                self.sprites[n].y = value;
            }
            // Reg::MX8
            0x10 => for i in 0..8 as usize {
                self.sprites[i].x.set_bit(8, value.get_bit(i));
            },
            // Reg::CR1
            0x11 => {
                self.raster_compare.set_bit(8, value.get_bit(7));
                let mut mode = self.mode.value();
                mode.set_bit(2, value.get_bit(6))
                    .set_bit(1, value.get_bit(5));
                self.mode = Mode::from(mode);
                self.den = value.get_bit(4);
                self.rsel = value.get_bit(3);
                self.y_scroll = value & 0x07;
                self.update_bad_line();
                if self.raster == self.raster_compare {
                    self.trigger_irq(0);
                }
            }
            // Reg::RASTER
            0x12 => {
                let new_value = (self.raster_compare & 0xff00) | (value as u16);
                if self.raster_compare != new_value && self.raster == new_value {
                    self.trigger_irq(0);
                }
                self.raster_compare = new_value;
            }
            // Reg::LPX
            0x13 => self.light_pen_pos[0] = value,
            // Reg::LPY
            0x14 => self.light_pen_pos[1] = value,
            // Reg::ME
            0x15 => for i in 0..8 as usize {
                self.sprites[i].enabled = value.get_bit(i);
            },
            // Reg::CR2
            0x16 => {
                let mut mode = self.mode.value();
                mode.set_bit(0, value.get_bit(4));
                self.mode = Mode::from(mode);
                self.csel = value.get_bit(3);
                self.x_scroll = value & 0x07;
            }
            // Reg::MYE
            0x17 => for i in 0..8 as usize {
                self.sprites[i].expand_y = value.get_bit(i);
            },
            // Reg::MEMPTR
            0x18 => {
                self.video_matrix = (((value & 0xf0) >> 4) as u16) << 10;
                self.char_base = (((value & 0x0f) >> 1) as u16) << 11;
            }
            // Reg::IRR
            0x19 => {
                self.interrupt_control.clear_events(value);
                if !self.interrupt_control.is_triggered() || value == 0xe2 {
                    // FIXME
                    self.irq_line
                        .borrow_mut()
                        .set_low(IrqSource::Vic.value(), false);
                }
            }
            // Reg::IMR
            0x1a => {
                self.interrupt_control.set_mask(value & 0x0f);
                self.irq_line.borrow_mut().set_low(
                    IrqSource::Vic.value(),
                    self.interrupt_control.is_triggered(),
                );
            }
            // Reg::MDP
            0x1b => for i in 0..8 as usize {
                self.sprites[i].priority = value.get_bit(i);
            },
            // Reg::MMC
            0x1c => for i in 0..8 as usize {
                self.sprites[i].multicolor = value.get_bit(i);
            },
            // Reg::MXE
            0x1d => for i in 0..8 as usize {
                self.sprites[i].expand_x = value.get_bit(i);
            },
            // Reg::MM
            0x1e => {}
            // Reg::MD
            0x1f => {}
            // Reg::EC
            0x20 => self.border_color = value & 0x0f,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.background_color[reg as usize - 0x21] = value & 0x0f,
            // Reg::MM0  - Reg::MM1
            0x25...0x26 => self.sprite_multicolor[reg as usize - 0x25] = value & 0x0f,
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprites[reg as usize - 0x27].color = value & 0x0f,
            _ => {}
        }
    }
}
