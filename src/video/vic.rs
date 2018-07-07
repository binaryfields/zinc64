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
use std::rc::Rc;

use bit_field::BitField;
use core::{Chip, FrameBuffer, IrqControl, IrqLine, Pin, Ram, VicModel};
use log::LogLevel;

use super::VicMemory;
use super::gfx_sequencer::{GfxSequencer, Mode};
use super::spec::Spec;

// SPEC: The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64

// TODO vic:
// 1 display/idle states cycle 58
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
    // Functional Units
    gfx_seq: GfxSequencer,
    interrupt_control: IrqControl,
    // Configuration
    char_base: u16,
    csel: bool,
    den: bool,
    rsel: bool,
    raster_compare: u16,
    x_scroll: u8,
    y_scroll: u8,
    video_matrix: u16,
    // Sprite and Color Data
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
    display_on: bool,
    display_state: bool,
    is_bad_line: bool,
    sprite_ptrs: [u16; 8],
    #[allow(dead_code)]
    sprite_mc: [u8; 8],
    #[allow(dead_code)]
    sprite_mcbase: [u8; 8],
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
            // Functional Units
            gfx_seq: GfxSequencer::new(),
            interrupt_control: IrqControl::new(),
            // Configuration
            char_base: 0,
            csel: false,
            den: false,
            rsel: false,
            raster_compare: 0x00,
            x_scroll: 0,
            y_scroll: 0,
            video_matrix: 0,
            // Sprite and Color Data
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
            display_on: false,
            display_state: false,
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
    fn map_sprite_to_screen(x: u16) -> u16 {
        match x {
            0x000...0x193 => x + 0x64,
            0x194...0x1ff => x - 0x194,
            _ => panic!("invalid sprite coords {}", x),
        }
    }

    #[inline]
    fn draw_border(&mut self) {
        let x_start = (self.cycle - 1) << 3;
        for x in x_start..x_start + 8 {
            self.update_border_main_ff(x);
            self.gfx_seq.clock();
            let mut rt = self.frame_buffer.borrow_mut();
            rt.write(x, self.raster, self.gfx_seq.output());
        }
    }

    #[inline]
    fn draw(&mut self) {
        let mut rt = self.frame_buffer.borrow_mut();
        let x_start = (self.cycle - 1) << 3;
        for x in x_start..x_start + 8 {
            self.gfx_seq.clock();
            rt.write(x, self.raster, self.gfx_seq.output());
        }
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

    #[inline]
    fn update_bad_line(&mut self) {
        /*
         "A Bad Line Condition is given at any arbitrary clock cycle, if at the
          negative edge of ø0 at the beginning of the cycle RASTER >= $30 and RASTER
          <= $f7 and the lower three bits of RASTER are equal to YSCROLL and if the
          DEN bit was set during an arbitrary cycle of raster line $30."
        */
        self.is_bad_line = match self.raster {
            0x30...0xf7 => {
                self.display_on && (self.raster & 0x07) as u8 == self.y_scroll
            },
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
    fn update_border_main_ff(&mut self, x: u16) {
        /*
        1. If the X coordinate reaches the right comparison value, the main border
           flip flop is set.
        4. If the X coordinate reaches the left comparison value and the Y
           coordinate reaches the bottom one, the vertical border flip flop is set.
        5. If the X coordinate reaches the left comparison value and the Y
           coordinate reaches the top one and the DEN bit in register $d011 is set,
           the vertical border flip flop is reset.
        6. If the X coordinate reaches the left comparison value and the vertical
           border flip flop is not set, the main flip flop is reset.
        */
        // TODO vic: border off by 4 pixels to get around gfx shift register issue
        if self.csel {
            if x == Vic::map_sprite_to_screen(0x18 - 4) {
                self.update_border_vertical_ff();
                if !self.gfx_seq.get_border_vertical_ff() {
                    self.gfx_seq.set_border_main_ff(false);
                }
            } else if x == Vic::map_sprite_to_screen(0x158 - 4) {
                self.gfx_seq.set_border_main_ff(true);
            }
        } else {
            if x == Vic::map_sprite_to_screen(0x1f - 4) {
                self.update_border_vertical_ff();
                if !self.gfx_seq.get_border_vertical_ff() {
                    self.gfx_seq.set_border_main_ff(false);
                }
            } else if x == Vic::map_sprite_to_screen(0x14f - 4) {
                self.gfx_seq.set_border_main_ff(true);
            }
        }
    }

    #[inline]
    fn update_border_vertical_ff(&mut self) {
        /*
            2. If the Y coordinate reaches the bottom comparison value in cycle 63, the
               vertical border flip flop is set.
            3. If the Y coordinate reaches the top comparison value in cycle 63 and the
               DEN bit in register $d011 is set, the vertical border flip flop is
               reset.
        */
        if self.rsel {
            if self.raster == 51 && self.den {
                self.gfx_seq.set_border_vertical_ff(false);
            } else if self.raster == 251 {
                self.gfx_seq.set_border_vertical_ff(true);
            }
        } else {
            if self.raster == 55 && self.den {
                self.gfx_seq.set_border_vertical_ff(false);
            } else if self.raster == 247 {
                self.gfx_seq.set_border_vertical_ff(true);
            }
        }
    }

    #[inline]
    fn update_display_on(&mut self) {
        if self.raster == 0x30 && self.den {
            self.display_on = true; // TODO vic: when is this reset
        }
    }

    // -- Memory Ops

    #[inline]
    fn c_access(&mut self) {
        if self.is_bad_line {
            let address = self.video_matrix | self.vc;
            self.vm_data_line[self.vmli] = self.mem.borrow().read(address);
            self.vm_color_line[self.vmli] = self.color_ram.borrow().read(self.vc) & 0x0f;
            // TODO vic: memory no access unless ba down for 3 cycles
        }
    }

    #[inline]
    fn g_access(&mut self) {
        let g_data = match self.gfx_seq.get_mode() {
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
            _ => panic!("unsupported graphics mode {}", self.gfx_seq.get_mode().value()),
        };
        let c_data = self.vm_data_line[self.vmli];
        let c_color = self.vm_color_line[self.vmli];
        self.gfx_seq.set_data(c_data, c_color, g_data);
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

    // -- Sprite Ops

    fn draw_sprites(&mut self, raster: u16) {
        for i in 0..8 {
            let n = 7 - i;
            if self.sprites[n].enabled {
                if self.is_sprite(n, raster) {
                    for j in 0..3 {
                        let sp_data = self.fetch_sprite_pixels(n, self.sprite_mc[n]);
                        if !self.sprites[n].multicolor {
                            self.draw_sprite(
                                Vic::map_sprite_to_screen(self.sprites[n].x) + (j << 3),
                                raster,
                                sp_data,
                                self.sprites[n].color,
                            );
                        } else {
                            self.draw_sprite_mc(
                                Vic::map_sprite_to_screen(self.sprites[n].x) + (j << 3),
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
        for i in 0..8u16 {
            if data.get_bit(i as usize) {
                rt.write(x + 7 - i, y, color);
            }
        }
    }

    #[inline]
    fn draw_sprite_mc(&self, x: u16, y: u16, n: usize, data: u8) {
        let mut rt = self.frame_buffer.borrow_mut();
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
                rt.write(x + 7 - (i * 2), y, color);
                rt.write(x + 6 - (i * 2), y, color);
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
        self.update_display_on();
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
                // TODO vic: clock cycle 2 logic
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
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
                self.draw_border();
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
                // TODO vic: clock cycle 58 display logic
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
                for i in 0..8 {
                    if self.sprites[i].y as u16 == self.raster {
                        self.sprite_mc[i] = 0;
                    }
                }
                let raster = self.raster;
                self.draw_sprites(raster);
                self.update_border_vertical_ff();
            }
            _ => panic!("invalid cycle"),
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
        // Functional Units
        self.gfx_seq.reset();
        self.interrupt_control.reset();
        // Configuration
        self.char_base = 0x1000;
        self.csel = true;
        self.den = true;
        self.rsel = true;
        self.raster_compare = 0;
        self.x_scroll = 0;
        self.y_scroll = 3;
        self.video_matrix = 0x0400;
        // Sprite and Color Data
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
        self.display_on = false;
        self.display_state = false;
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
                    .set_bit(6, self.gfx_seq.get_mode().value().get_bit(2))
                    .set_bit(5, self.gfx_seq.get_mode().value().get_bit(1))
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
                    .set_bit(4, self.gfx_seq.get_mode().value().get_bit(0))
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
            0x20 => self.gfx_seq.get_border_color() | 0xf0,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.gfx_seq.get_bg_color((reg - 0x21) as usize) | 0xf0,
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
                let mut mode = self.gfx_seq.get_mode().value();
                mode.set_bit(2, value.get_bit(6))
                    .set_bit(1, value.get_bit(5));
                self.gfx_seq.set_mode(Mode::from(mode));
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
                let mut mode = self.gfx_seq.get_mode().value();
                mode.set_bit(0, value.get_bit(4));
                self.gfx_seq.set_mode(Mode::from(mode));
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
                    // TODO vic: check interrupt reset logic
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
            0x20 => self.gfx_seq.set_border_color(value & 0x0f),
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.gfx_seq.set_bg_color(reg as usize - 0x21, value & 0x0f),
            // Reg::MM0  - Reg::MM1
            0x25...0x26 => self.sprite_multicolor[reg as usize - 0x25] = value & 0x0f,
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprites[reg as usize - 0x27].color = value & 0x0f,
            _ => {}
        }
    }
}
