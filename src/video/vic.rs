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
use super::border_unit::BorderUnit;
use super::gfx_sequencer::{GfxSequencer, Mode};
use super::spec::Spec;
use super::sprite_sequencer::{SpriteSequencer, Mode as SpriteMode};

// SPEC: The MOS 6567/6569 video controller (VIC-II) and its application in the Commodore 64

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

// TODO vic:
// 1 display/idle states cycle 58
// 3 scroll_x

#[derive(Copy, Clone)]
pub enum IrqSource {
    Vic = 2,
}

impl IrqSource {
    pub fn value(&self) -> usize {
        *self as usize
    }
}

pub struct Vic {
    // Dependencies
    spec: Spec,
    ba_line: Rc<RefCell<Pin>>,
    color_ram: Rc<RefCell<Ram>>,
    frame_buffer: Rc<RefCell<FrameBuffer>>,
    irq_line: Rc<RefCell<IrqLine>>,
    mem: Rc<RefCell<VicMemory>>,
    // Functional Units
    border_unit: BorderUnit,
    gfx_seq: GfxSequencer,
    interrupt_control: IrqControl,
    sprites: [SpriteSequencer; 8],
    // Configuration
    char_base: u16,
    den: bool,
    raster_compare: u16,
    x_scroll: u8,
    y_scroll: u8,
    video_matrix: u16,
    // Registers
    mc: [u8; 8],
    mc_base: [u8; 8],
    raster_cycle: u16,
    raster_y: u16,
    rc: u8,
    vc: u16,
    vc_base: u16,
    vmli: usize,
    // Runtime State
    display_on: bool,
    display_state: bool,
    is_bad_line: bool,
    sprite_ptrs: [u16; 8],
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
        let sprites = [
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
            SpriteSequencer::new(),
        ];
        let vic = Vic {
            // Dependencies
            spec,
            ba_line,
            color_ram,
            frame_buffer,
            irq_line,
            mem,
            // Functional Units
            border_unit: BorderUnit::new(),
            gfx_seq: GfxSequencer::new(),
            interrupt_control: IrqControl::new(),
            sprites,
            // Configuration
            char_base: 0,
            den: false,
            raster_compare: 0x00,
            x_scroll: 0,
            y_scroll: 0,
            video_matrix: 0,
            // Registers
            mc: [0; 8],
            mc_base: [0; 8],
            raster_cycle: 1,
            raster_y: 0,
            rc: 0,
            vc_base: 0,
            vc: 0,
            vmli: 0,
            // Runtime State
            display_on: false,
            display_state: false,
            is_bad_line: false,
            sprite_ptrs: [0; 8],
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
        let x_start = (self.raster_cycle - 1) << 3;
        for x in x_start..x_start + 8 {
            self.border_unit.update_main_flop(x, self.raster_y, self.den);
            for sprite in self.sprites.iter_mut() {
                sprite.clock(x);
            }
            if !self.border_unit.is_enabled() {
                self.gfx_seq.clock();
                let mut i = 0;
                let sprite_output = loop {
                    if i >= 8 {
                        break None;
                    }
                    let sprite = &self.sprites[i];
                    if sprite.display && sprite.output().is_some() {
                        break sprite.output();
                    }
                    i += 1;
                };
                let pixel = if let Some(output) = sprite_output {
                    output
                } else {
                    self.gfx_seq.output()
                };
                let mut rt = self.frame_buffer.borrow_mut();
                rt.write(x, self.raster_y, pixel);
            } else {
                let mut rt = self.frame_buffer.borrow_mut();
                rt.write(x, self.raster_y, self.border_unit.output());
            }
        }
    }

    #[inline]
    fn draw(&mut self) {
        let mut rt = self.frame_buffer.borrow_mut();
        let x_start = (self.raster_cycle - 1) << 3;
        for x in x_start..x_start + 8 {
            for sprite in self.sprites.iter_mut() {
                sprite.clock(x);
            }
            if !self.border_unit.is_enabled() {
                self.gfx_seq.clock();
                let mut i = 0;
                let sprite_output = loop {
                    if i >= 8 {
                        break None;
                    }
                    let sprite = &self.sprites[i];
                    if sprite.display && sprite.output().is_some() {
                        break sprite.output();
                    }
                    i += 1;
                };
                let pixel = if let Some(output) = sprite_output {
                    output
                } else {
                    self.gfx_seq.output()
                };
                rt.write(x, self.raster_y, pixel);
            } else {
                rt.write(x, self.raster_y, self.border_unit.output());
            }
            // FIXME vic/sprite: mux logic
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
        Section: 3.5. Bad Lines
         A Bad Line Condition is given at any arbitrary clock cycle, if at the
         negative edge of ø0 at the beginning of the cycle RASTER >= $30 and RASTER
         <= $f7 and the lower three bits of RASTER are equal to YSCROLL and if the
         DEN bit was set during an arbitrary cycle of raster line $30.
        */
        self.is_bad_line = if self.display_on {
            if self.raster_y >= 0x30 && self.raster_y <= 0xf7 {
                (self.raster_y & 0x07) as u8 == self.y_scroll
            } else {
                false
            }
        } else {
            false
        };
    }

    #[inline]
    fn update_display_on(&mut self) {
        /*
        Section: 3.10. Display Enable
        A Bad Line Condition can only occur if the DEN bit has been set for at
        least one cycle somewhere in raster line $30 (see section 3.5.).
        */
        if self.raster_y == 0x30 && self.den {
            self.display_on = true;
        }
    }

    #[inline]
    fn update_sprite_display(&mut self) {
        /*
        Section: 3.8. Sprites
        4. In the first phase of cycle 58, the MC of every sprite is loaded from
           its belonging MCBASE (MCBASE->MC) and it is checked if the DMA for the
           sprite is turned on and the Y coordinate of the sprite matches the lower
           8 bits of RASTER. If this is the case, the display of the sprite is
           turned on.
        */
        for sprite in self.sprites.iter_mut() {
            if sprite.config.y == (self.raster_y as u8) {
                sprite.display = sprite.dma;
            }
        }
    }

    #[inline]
    fn update_sprite_dma_on(&mut self) {
        /*
        Section: 3.8. Sprites
        3. In the first phases of cycle 55 and 56, the VIC checks for every sprite
           if the corresponding MxE bit in register $d015 is set and the Y
           coordinate of the sprite (odd registers $d001-$d00f) match the lower 8
           bits of RASTER. If this is the case and the DMA for the sprite is still
           off, the DMA is switched on, MCBASE is cleared, and if the MxYE bit is
           set the expansion flip flip is reset.
        */
        for n in 0..8 {
            let sprite = &mut self.sprites[n];
            if sprite.config.enabled && sprite.config.y == (self.raster_y as u8) {
                if !sprite.dma {
                    sprite.dma = true;
                    self.mc_base[n] = 0;
                    if sprite.config.expand_y {
                        sprite.expansion_flop = false;
                    }
                }
            }
        }
    }

    #[inline]
    fn update_sprite_dma_off(&mut self) {
        /*
        Section: 3.8. Sprites
        8. In the first phase of cycle 16, it is checked if the expansion flip flop
           is set. If so, MCBASE is incremented by 1. After that, the VIC checks if
           MCBASE is equal to 63 and turns of the DMA and the display of the sprite
           if it is.
        */
        for i in 0..8 {
            if self.sprites[i].expansion_flop {
                self.mc_base[i] += 1;
                if self.mc_base[i] == 63 {
                    let mut sprite = &mut self.sprites[i];
                    sprite.dma = false;
                    sprite.display = false;
                }
            }
        }
    }

    #[inline]
    fn update_sprite_expansion_ff(&mut self) {
        /*
        Section: 3.8. Sprites
        2. If the MxYE bit is set in the first phase of cycle 55, the expansion
           flip flop is inverted.
        */
        for sprite in self.sprites.iter_mut() {
            if sprite.config.expand_y {
                sprite.expansion_flop = !sprite.expansion_flop;
            }
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
        let g_data = match self.gfx_seq.config.mode {
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
            _ => panic!("unsupported graphics mode {}", self.gfx_seq.config.mode.value()),
        };
        let c_data = self.vm_data_line[self.vmli];
        let c_color = self.vm_color_line[self.vmli];
        self.gfx_seq.set_data(c_data, c_color, g_data);
        /*
        Section: 3.7.2. VC and RC
        4. VC and VMLI are incremented after each g-access in display state.
        */
        self.vc += 1;
        self.vmli += 1;
    }

    #[inline]
    fn p_access(&mut self, n: usize) {
        let address = self.video_matrix | 0x03f8 | n as u16;
        self.sprite_ptrs[n] = (self.mem.borrow().read(address) as u16) << 6;
    }

    #[inline]
    fn s_access(&mut self, n: usize, byte: usize) {
        /*
        Section: 3.8. Sprites
        5. If the DMA for a sprite is turned on, three s-accesses are done in
           sequence in the corresponding cycles assigned to the sprite (see the
           diagrams in section 3.6.3.). The p-accesses are always done, even if the
           sprite is turned off. The read data of the first access is stored in the
           upper 8 bits of the shift register, that of the second one in the middle
           8 bits and that of the third one in the lower 8 bits. MC is incremented
           by one after each s-access.
        */
        let address = self.sprite_ptrs[n] | (self.mc[n] as u16);
        let data = self.mem.borrow().read(address);
        self.sprites[n].set_data(byte, data);
        self.mc[n] += 1;
    }
}

impl Chip for Vic {
    fn clock(&mut self) {
        self.update_display_on();
        match self.raster_cycle {
            1 => {
                if self.raster_y == self.raster_compare && self.raster_y != 0 {
                    self.trigger_irq(0);
                }
                self.update_bad_line();
                self.p_access(3);
                if self.sprites[3].dma {
                    self.s_access(3, 0);
                }
                let sprite_dma = self.sprites[3].dma | self.sprites[4].dma;
                self.set_ba(sprite_dma);
            }
            2 => {
                // TODO vic: clock cycle 2 logic
                if self.raster_y == self.raster_compare && self.raster_y == 0 {
                    self.trigger_irq(0);
                }
                if self.sprites[3].dma {
                    self.s_access(3, 1);
                    self.s_access(3, 2);
                }
                let sprite_dma = self.sprites[4].dma | self.sprites[5].dma;
                self.set_ba(sprite_dma);
            }
            3 => {
                self.p_access(4);
                if self.sprites[4].dma {
                    self.s_access(4, 0);
                }
                let sprite_dma = self.sprites[4].dma | self.sprites[5].dma;
                self.set_ba(sprite_dma);
            }
            4 => {
                if self.sprites[4].dma {
                    self.s_access(4, 1);
                    self.s_access(4, 2);
                }
                let sprite_dma = self.sprites[5].dma | self.sprites[6].dma;
                self.set_ba(sprite_dma);
            }
            5 => {
                self.p_access(5);
                if self.sprites[5].dma {
                    self.s_access(5, 0);
                }
                let sprite_dma = self.sprites[5].dma | self.sprites[6].dma;
                self.set_ba(sprite_dma);
            }
            6 => {
                if self.sprites[5].dma {
                    self.s_access(5, 1);
                    self.s_access(5, 2);
                }
                let sprite_dma = self.sprites[6].dma | self.sprites[7].dma;
                self.set_ba(sprite_dma);
            }
            7 => {
                self.p_access(6);
                if self.sprites[6].dma {
                    self.s_access(6, 0);
                }
                let sprite_dma = self.sprites[6].dma | self.sprites[7].dma;
                self.set_ba(sprite_dma);
            }
            8 => {
                if self.sprites[6].dma {
                    self.s_access(6, 1);
                    self.s_access(6, 2);
                }
                let sprite_dma = self.sprites[7].dma;
                self.set_ba(sprite_dma);
            }
            9 => {
                self.p_access(7);
                if self.sprites[7].dma {
                    self.s_access(7, 0);
                }
                let sprite_dma = self.sprites[7].dma;
                self.set_ba(sprite_dma);
            }
            10 => {
                if self.sprites[7].dma {
                    self.s_access(7, 1);
                    self.s_access(7, 2);
                }
                self.draw_border();
                self.set_ba(false);
            }
            11 => {
                self.draw_border();
                self.set_ba(false);
            }
            12...13 => {
                /*
                Section: 3.7.2. VC and RC
                3. If there is a Bad Line Condition in cycles 12-54, BA is set low and the
                   c-accesses are started. Once started, one c-access is done in the second
                   phase of every clock cycle in the range 15-54.
                */
                self.draw_border();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
            }
            14 => {
                /*
                Section: 3.7.2. VC and RC
                2. In the first phase of cycle 14 of each line, VC is loaded from VCBASE
                   (VCBASE->VC) and VMLI is cleared. If there is a Bad Line Condition in
                   this phase, RC is also reset to zero.
                */
                self.vc = self.vc_base;
                self.vmli = 0;
                if self.is_bad_line {
                    self.rc = 0;
                }
                self.draw_border();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
            }
            15 => {
                /*
                Section: 3.8. Sprites
                7. In the first phase of cycle 15, it is checked if the expansion flip flop
                   is set. If so, MCBASE is incremented by 2.
                */
                for i in 0..8 {
                    if self.sprites[i].expansion_flop {
                        self.mc_base[i] += 2;
                    }
                }
                self.c_access();
                self.draw_border();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
            }
            16 => {
                self.update_sprite_dma_off();
                self.g_access();
                self.c_access();
                self.draw_border();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
            }
            17...54 => {
                self.g_access();
                self.c_access();
                self.draw();
                let is_bad_line = self.is_bad_line;
                self.set_ba(is_bad_line);
            }
            55 => {
                self.update_sprite_dma_on();
                self.update_sprite_expansion_ff();
                self.g_access();
                self.draw();
                let sprite_dma = self.sprites[0].dma;
                self.set_ba(sprite_dma);
            }
            56 => {
                self.update_sprite_dma_on();
                self.draw_border();
                let sprite_dma = self.sprites[0].dma;
                self.set_ba(sprite_dma);
            }
            57 => {
                self.draw_border();
                let sprite_dma = self.sprites[0].dma | self.sprites[1].dma;
                self.set_ba(sprite_dma);
            }
            58 => {
                // TODO vic: clock cycle 58 display logic
                /*
                Section: 3.7.2. VC and RC
                5. In the first phase of cycle 58, the VIC checks if RC=7. If so, the video
                   logic goes to idle state and VCBASE is loaded from VC (VC->VCBASE). If
                   the video logic is in display state afterwards (this is always the case
                   if there is a Bad Line Condition), RC is incremented.
                */
                if self.rc == 7 {
                    self.vc_base = self.vc;
                }
                self.rc += 1;
                /*
                Section: 3.8. Sprites
                4. In the first phase of cycle 58, the MC of every sprite is loaded from
                   its belonging MCBASE (MCBASE->MC) ...
                */
                for i in 0..8 {
                    self.mc[i] = self.mc_base[i];
                }
                self.update_sprite_display();
                self.p_access(0);
                if self.sprites[0].dma {
                    self.s_access(0, 0);
                }
                self.draw_border();
                let sprite_dma = self.sprites[0].dma | self.sprites[1].dma;
                self.set_ba(sprite_dma);
            }
            59 => {
                if self.sprites[0].dma {
                    self.s_access(0, 1);
                    self.s_access(0, 2);
                }
                self.draw_border();
                let sprite_dma = self.sprites[1].dma | self.sprites[2].dma;
                self.set_ba(sprite_dma);
            }
            60 => {
                self.p_access(1);
                if self.sprites[1].dma {
                    self.s_access(1, 0);
                }
                self.draw_border();
                let sprite_dma = self.sprites[1].dma | self.sprites[2].dma;
                self.set_ba(sprite_dma);
            }
            61 => {
                if self.sprites[1].dma {
                    self.s_access(1, 1);
                    self.s_access(1, 2);
                }
                let sprite_dma = self.sprites[2].dma | self.sprites[3].dma;
                self.set_ba(sprite_dma);
            }
            62 => {
                self.p_access(2);
                if self.sprites[2].dma {
                    self.s_access(2, 0);
                }
                let sprite_dma = self.sprites[2].dma | self.sprites[3].dma;
                self.set_ba(sprite_dma);
            }
            63 => {
                if self.sprites[2].dma {
                    self.s_access(2, 1);
                    self.s_access(2, 2);
                }
                let sprite_dma = self.sprites[3].dma | self.sprites[4].dma;
                self.set_ba(sprite_dma);
                self.border_unit.update_vertical_flop(self.raster_y, self.den);
            }
            _ => panic!("invalid cycle"),
        }
        // Update counters/vsync
        self.raster_cycle += 1;
        if self.raster_cycle > self.spec.cycles_per_raster {
            self.raster_cycle = 1;
            self.raster_y += 1;
            if self.raster_y >= self.spec.raster_lines {
                self.raster_y = 0;
                // TODO vic: check display on reset
                // self.display_on = false;
                /*
                Section: 3.7.2. VC and RC
                1. Once somewhere outside of the range of raster lines $30-$f7 (i.e.
                   outside of the Bad Line range), VCBASE is reset to zero. This is
                   presumably done in raster line 0, the exact moment cannot be determined
                   and is irrelevant.
                */
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
        self.border_unit.reset();
        self.gfx_seq.reset();
        self.interrupt_control.reset();
        for sprite in self.sprites.iter_mut() {
            sprite.reset();
        }
        // Configuration
        self.char_base = 0x1000;
        self.den = true;
        self.raster_compare = 0;
        self.x_scroll = 0;
        self.y_scroll = 3;
        self.video_matrix = 0x0400;
        // Registers
        for i in 0..self.mc.len() {
            self.mc[i] = 0;
        }
        for i in 0..self.mc_base.len() {
            self.mc_base[i] = 0;
        }
        self.raster_cycle = 1;
        self.raster_y = 0x0100;
        self.rc = 0;
        self.vc = 0;
        self.vc_base = 0;
        self.vmli = 0;
        // Runtime State
        self.display_on = false;
        self.display_state = false;
        self.is_bad_line = false;
        for i in 0..self.sprite_ptrs.len() {
            self.sprite_ptrs[i] = 0;
        }
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
                (self.sprites[(reg >> 1) as usize].config.x & 0x00ff) as u8
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                self.sprites[((reg - 1) >> 1) as usize].config.y
            }
            // Reg::MX8
            0x10 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].config.x.get_bit(8));
                }
                result
            }
            // Reg::CR1
            0x11 => {
                let mut result = 0;
                result
                    .set_bit(7, self.raster_y.get_bit(8))
                    .set_bit(6, self.gfx_seq.config.mode.value().get_bit(2))
                    .set_bit(5, self.gfx_seq.config.mode.value().get_bit(1))
                    .set_bit(4, self.den)
                    .set_bit(3, self.border_unit.config.rsel);
                result | (self.y_scroll & 0x07)
            }
            // Reg::RASTER
            0x12 => (self.raster_y & 0x00ff) as u8,
            // Reg::LPX
            0x13 => 0,
            // Reg::LPY
            0x14 => 0,
            // Reg::ME
            0x15 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].config.enabled);
                }
                result
            }
            // Reg::CR2
            0x16 => {
                let mut result = 0;
                result
                    .set_bit(5, true)
                    .set_bit(4, self.gfx_seq.config.mode.value().get_bit(0))
                    .set_bit(3, self.border_unit.config.csel);
                result | (self.x_scroll & 0x07) | 0xc0
            }
            // Reg::MYE
            0x17 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].config.expand_y);
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
                    result.set_bit(i, self.sprites[i].config.data_priority);
                }
                result
            }
            // Reg::MMC
            0x1c => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].config.mode == SpriteMode::Multicolor);
                }
                result
            }
            // Reg::MXE
            0x1d => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprites[i].config.expand_x);
                }
                result
            }
            // Reg::MM
            0x1e => 0xff, // DEFERRED collision
            // Reg::MD
            0x1f => 0xff, // DEFERRED collision
            // Reg::EC
            0x20 => self.border_unit.config.border_color | 0xf0,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.gfx_seq.config.bg_color[(reg - 0x21) as usize] | 0xf0,
            // Reg::MM0 - Reg::MM1
            0x25...0x26 => self.sprites[0].config.multicolor[(reg - 0x25) as usize] | 0xf0,
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprites[(reg - 0x27) as usize].config.color | 0xf0,
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
                self.sprites[n].config.x = (self.sprites[n].config.x & 0xff00) | (value as u16);
                self.sprites[n].config.x_screen = Self::map_sprite_to_screen(self.sprites[n].config.x);
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                let n = ((reg - 1) >> 1) as usize;
                self.sprites[n].config.y = value;
            }
            // Reg::MX8
            0x10 => for i in 0..8 as usize {
                self.sprites[i].config.x.set_bit(8, value.get_bit(i));
                self.sprites[i].config.x_screen = Self::map_sprite_to_screen(self.sprites[i].config.x);
            },
            // Reg::CR1
            0x11 => {
                self.raster_compare.set_bit(8, value.get_bit(7));
                let mut mode = self.gfx_seq.config.mode.value();
                mode.set_bit(2, value.get_bit(6))
                    .set_bit(1, value.get_bit(5));
                self.gfx_seq.config.mode = Mode::from(mode);
                self.den = value.get_bit(4);
                self.border_unit.config.rsel = value.get_bit(3);
                self.y_scroll = value & 0x07;
                self.update_bad_line();
                if self.raster_y == self.raster_compare {
                    self.trigger_irq(0);
                }
            }
            // Reg::RASTER
            0x12 => {
                let new_value = (self.raster_compare & 0xff00) | (value as u16);
                if self.raster_compare != new_value && self.raster_y == new_value {
                    self.trigger_irq(0);
                }
                self.raster_compare = new_value;
            }
            // Reg::LPX
            0x13 => {},
            // Reg::LPY
            0x14 => {},
            // Reg::ME
            0x15 => for i in 0..8 as usize {
                self.sprites[i].config.enabled = value.get_bit(i);
            },
            // Reg::CR2
            0x16 => {
                let mut mode = self.gfx_seq.config.mode.value();
                mode.set_bit(0, value.get_bit(4));
                self.gfx_seq.config.mode = Mode::from(mode);
                self.border_unit.config.csel = value.get_bit(3);
                self.x_scroll = value & 0x07;
            }
            // Reg::MYE
            0x17 => for i in 0..8 as usize {
                self.sprites[i].config.expand_y = value.get_bit(i);
                /*
                Section: 3.8. Sprites
                1. The expansion flip flip is set as long as the bit in MxYE in register
                   $d017 corresponding to the sprite is cleared.
                */
                self.sprites[i].expansion_flop = !self.sprites[i].config.expand_y;
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
                self.sprites[i].config.data_priority = value.get_bit(i);
            },
            // Reg::MMC
            0x1c => for i in 0..8 as usize {
                self.sprites[i].config.mode = if !value.get_bit(i) {
                    SpriteMode::Standard
                } else {
                    SpriteMode::Multicolor
                };
            },
            // Reg::MXE
            0x1d => for i in 0..8 as usize {
                self.sprites[i].config.expand_x = value.get_bit(i);
            },
            // Reg::MM
            0x1e => {}
            // Reg::MD
            0x1f => {}
            // Reg::EC
            0x20 => self.border_unit.config.border_color = value & 0x0f,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.gfx_seq.config.bg_color[reg as usize - 0x21] = value & 0x0f,
            // Reg::MM0  - Reg::MM1
            0x25...0x26 => for i in 0..8 as usize {
                self.sprites[i].config.multicolor[reg as usize - 0x25] = value & 0x0f;
            },
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprites[reg as usize - 0x27].config.color = value & 0x0f,
            _ => {}
        }
    }
}
