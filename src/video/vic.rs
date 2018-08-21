// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bit_field::BitField;
use core::{Chip, IrqControl, IrqLine, Pin, Ram, VicModel, VideoOutput};
use log::LogLevel;

use super::border_unit::BorderUnit;
use super::gfx_sequencer::{GfxSequencer, Mode};
use super::mux_unit::MuxUnit;
use super::spec::Spec;
use super::sprite_sequencer::{Mode as SpriteMode, SpriteSequencer};
use super::VicMemory;

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

// TODO vic: fix ntsc support

#[derive(Copy, Clone)]
pub enum IrqSource {
    Vic = 2,
}

impl IrqSource {
    pub fn value(&self) -> usize {
        *self as usize
    }
}

struct RasterUnit {
    display_on: bool,
    display_state: bool,
    is_bad_line: bool,
    rc: u8,
    vc: u16,
    vc_base: u16,
    vmli: usize,
    vm_color_line: [u8; 40],
    vm_data_line: [u8; 40],
    // Sprites
    mc: [u8; 8],
    mc_base: [u8; 8],
    sprite_dma: [bool; 8],
    sprite_ptrs: [u16; 8],
    sprites_on: bool,
}

impl RasterUnit {
    pub fn new() -> Self {
        Self {
            rc: 0,
            vc_base: 0,
            vc: 0,
            vmli: 0,
            display_on: false,
            display_state: false,
            is_bad_line: false,
            vm_color_line: [0; 40],
            vm_data_line: [0; 40],
            mc: [0; 8],
            mc_base: [0; 8],
            sprite_dma: [false; 8],
            sprite_ptrs: [0; 8],
            sprites_on: false,
        }
    }

    pub fn reset(&mut self) {
        self.display_on = false;
        self.display_state = false;
        self.is_bad_line = false;
        self.rc = 0;
        self.vc = 0;
        self.vc_base = 0;
        self.vmli = 0;
        for value in self.vm_color_line.iter_mut() {
            *value = 0;
        }
        for value in self.vm_data_line.iter_mut() {
            *value = 0;
        }
        // Sprites
        for i in 0..self.mc.len() {
            self.mc[i] = 0;
        }
        for i in 0..self.mc_base.len() {
            self.mc_base[i] = 0;
        }
        for dma in self.sprite_dma.iter_mut() {
            *dma = false;
        }
        for ptr in self.sprite_ptrs.iter_mut() {
            *ptr = 0;
        }
        self.sprites_on = false;
    }
}

pub struct Vic {
    // Dependencies
    spec: Spec,
    color_ram: Rc<RefCell<Ram>>,
    mem: VicMemory,
    // Functional Units
    border_unit: BorderUnit,
    gfx_seq: GfxSequencer,
    irq_control: IrqControl,
    mux_unit: MuxUnit,
    raster_unit: RasterUnit,
    sprite_units: [SpriteSequencer; 8],
    // Configuration
    char_base: u16,
    den: bool,
    frame_buffer_width: usize,
    raster_compare: u16,
    x_scroll: u8,
    y_scroll: u8,
    video_matrix: u16,
    // Runtime State
    cycle: u16,
    y: u16,
    // I/O
    ba_line: Rc<RefCell<Pin>>,
    irq_line: Rc<RefCell<IrqLine>>,
    frame_buffer: Rc<RefCell<dyn VideoOutput>>,
    vsync_flag: Rc<Cell<bool>>,
}

impl Vic {
    pub fn new(
        chip_model: VicModel,
        color_ram: Rc<RefCell<Ram>>,
        mem: VicMemory,
        frame_buffer: Rc<RefCell<dyn VideoOutput>>,
        vsync_flag: Rc<Cell<bool>>,
        ba_line: Rc<RefCell<Pin>>,
        irq_line: Rc<RefCell<IrqLine>>,
    ) -> Vic {
        info!(target: "video", "Initializing VIC");
        let spec = Spec::new(chip_model);
        let frame_buffer_width = frame_buffer.borrow().get_dimension().0;
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
            color_ram,
            mem,
            // Functional Units
            border_unit: BorderUnit::new(),
            gfx_seq: GfxSequencer::new(),
            irq_control: IrqControl::new(),
            mux_unit: MuxUnit::new(),
            raster_unit: RasterUnit::new(),
            sprite_units: sprites,
            // Configuration
            char_base: 0,
            den: false,
            frame_buffer_width,
            raster_compare: 0x00,
            x_scroll: 0,
            y_scroll: 0,
            video_matrix: 0,
            // Runtime State
            cycle: 1,
            y: 0,
            // I/O
            frame_buffer,
            ba_line,
            irq_line,
            vsync_flag,
        };
        vic
    }

    fn draw(&mut self) {
        let x_start = (self.cycle << 3) - 12;
        let x_scroll_start = x_start + self.x_scroll as u16;
        let mut pixel_idx = self.y as usize * self.frame_buffer_width + x_start as usize;
        for x in x_start..x_start + 8 {
            if !self.border_unit.is_enabled() {
                if x == x_scroll_start {
                    self.gfx_seq.load_data();
                }
                self.gfx_seq.clock();
                self.mux_unit.feed_graphics(self.gfx_seq.output());
            } else {
                self.mux_unit.feed_border(self.border_unit.output());
            }
            if self.raster_unit.sprites_on {
                for sprite in self.sprite_units.iter_mut() {
                    sprite.clock(x);
                }
                let sprite_output = self.output_sprites();
                self.mux_unit.compute_collisions(&sprite_output);
                self.mux_unit.feed_sprites(&sprite_output);
                if self.mux_unit.mb_interrupt {
                    self.trigger_irq(1);
                }
                if self.mux_unit.mm_interrupt {
                    self.trigger_irq(2);
                }
            }
            let pixel = self.mux_unit.output();
            self.frame_buffer.borrow_mut().write(pixel_idx, pixel);
            pixel_idx += 1;
        }
    }

    fn draw_cycle_17_56(&mut self) {
        let x_start = (self.cycle << 3) - 12;
        let x_scroll_start = x_start + self.x_scroll as u16;
        let mut pixel_idx = self.y as usize * self.frame_buffer_width + x_start as usize;
        for x in x_start..x_start + 8 {
            self.border_unit.update_main_flop(x, self.y, self.den);
            if !self.border_unit.is_enabled() {
                if x == x_scroll_start {
                    self.gfx_seq.load_data();
                }
                self.gfx_seq.clock();
                self.mux_unit.feed_graphics(self.gfx_seq.output());
            } else {
                self.mux_unit.feed_border(self.border_unit.output());
            }
            if self.raster_unit.sprites_on {
                for sprite in self.sprite_units.iter_mut() {
                    sprite.clock(x);
                }
                let sprite_output = self.output_sprites();
                self.mux_unit.compute_collisions(&sprite_output);
                self.mux_unit.feed_sprites(&sprite_output);
                if self.mux_unit.mb_interrupt {
                    self.trigger_irq(1);
                }
                if self.mux_unit.mm_interrupt {
                    self.trigger_irq(2);
                }
            }
            let pixel = self.mux_unit.output();
            self.frame_buffer.borrow_mut().write(pixel_idx, pixel);
            pixel_idx += 1;
        }
    }

    fn draw_border(&mut self) {
        let x_start = (self.cycle << 3) - 12;
        let mut pixel_idx = self.y as usize * self.frame_buffer_width + x_start as usize;
        for x in x_start..x_start + 8 {
            self.border_unit.update_main_flop(x, self.y, self.den);
            self.mux_unit.feed_border(self.border_unit.output());
            if self.raster_unit.sprites_on {
                for sprite in self.sprite_units.iter_mut() {
                    sprite.clock(x);
                }
                let sprite_output = self.output_sprites();
                self.mux_unit.compute_collisions(&sprite_output);
                self.mux_unit.feed_sprites(&sprite_output);
                if self.mux_unit.mb_interrupt {
                    self.trigger_irq(1);
                }
                if self.mux_unit.mm_interrupt {
                    self.trigger_irq(2);
                }
            }
            let pixel = self.mux_unit.output();
            self.frame_buffer.borrow_mut().write(pixel_idx, pixel);
            pixel_idx += 1;
        }
    }

    fn map_sprite_to_screen(&self, x: u16) -> u16 {
        match self.spec.first_x_coord {
            0x194 => {
                match x {
                    0x000...0x193 => x + 0x64, // 0x1f7 - 0x193
                    0x194...0x1ff => x - 0x194,
                    _ => panic!("invalid sprite coords {}", x),
                }
            }
            0x19c => {
                match x {
                    0x000...0x19b => x + 0x64, // 0x1ff - 0x19b
                    0x19c...0x1ff => x - 0x19c,
                    _ => panic!("invalid sprite coords {}", x),
                }
            }
            _ => panic!("invalid sprite coords {}", x),
        }
    }

    fn output_sprites(&self) -> [Option<u8>; 8] {
        [
            self.sprite_units[0].output(),
            self.sprite_units[1].output(),
            self.sprite_units[2].output(),
            self.sprite_units[3].output(),
            self.sprite_units[4].output(),
            self.sprite_units[5].output(),
            self.sprite_units[6].output(),
            self.sprite_units[7].output(),
        ]
    }

    fn set_ba(&mut self, is_bad_line: bool) {
        self.ba_line.borrow_mut().set_active(!is_bad_line);
    }

    fn trigger_irq(&mut self, source: usize) {
        self.irq_control.set_event(source);
        if self.irq_control.is_triggered() {
            if log_enabled!(LogLevel::Trace) {
                trace!(target: "vic::reg", "Irq data = {:02x}, mask = {:02x}, source: {}",
                       self.irq_control.get_data(),
                       self.irq_control.get_mask(),
                       source
                );
            }
            self.irq_line
                .borrow_mut()
                .set_low(IrqSource::Vic.value(), true);
        }
    }

    fn update_bad_line(&mut self) {
        /*
        Section: 3.5. Bad Lines
         A Bad Line Condition is given at any arbitrary clock cycle, if at the
         negative edge of ø0 at the beginning of the cycle RASTER >= $30 and RASTER
         <= $f7 and the lower three bits of RASTER are equal to YSCROLL and if the
         DEN bit was set during an arbitrary cycle of raster line $30.
        */
        self.raster_unit.is_bad_line = if self.raster_unit.display_on {
            if self.y >= 0x30 && self.y <= 0xf7 {
                (self.y & 0x07) as u8 == self.y_scroll
            } else {
                false
            }
        } else {
            false
        };
    }

    fn update_display_on(&mut self) {
        /*
        Section: 3.10. Display Enable
        A Bad Line Condition can only occur if the DEN bit has been set for at
        least one cycle somewhere in raster line $30 (see section 3.5.).
        */
        if self.y == 0x30 && self.den {
            self.raster_unit.display_on = true;
        }
    }

    fn update_display_state(&mut self) {
        if self.raster_unit.is_bad_line {
            self.raster_unit.display_state = true;
        }
    }

    fn update_sprite_display(&mut self) {
        /*
        Section: 3.8. Sprites
        4. In the first phase of cycle 58, the MC of every sprite is loaded from
           its belonging MCBASE (MCBASE->MC) and it is checked if the DMA for the
           sprite is turned on and the Y coordinate of the sprite matches the lower
           8 bits of RASTER. If this is the case, the display of the sprite is
           turned on.
        */
        for (i, sprite) in self.sprite_units.iter_mut().enumerate() {
            if sprite.config.y == (self.y as u8) {
                sprite.display = self.raster_unit.sprite_dma[i];
            }
        }
        self.raster_unit.sprites_on = self.sprite_units.iter().any(|sp| sp.display);
    }

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
            let sprite = &mut self.sprite_units[n];
            if sprite.config.enabled && sprite.config.y == (self.y as u8) {
                if !self.raster_unit.sprite_dma[n] {
                    self.raster_unit.sprite_dma[n] = true;
                    self.raster_unit.mc_base[n] = 0;
                    if sprite.config.expand_y {
                        sprite.expansion_flop = false;
                    }
                }
            }
        }
    }

    fn update_sprite_dma_off(&mut self) {
        /*
        Section: 3.8. Sprites
        8. In the first phase of cycle 16, it is checked if the expansion flip flop
           is set. If so, MCBASE is incremented by 1. After that, the VIC checks if
           MCBASE is equal to 63 and turns of the DMA and the display of the sprite
           if it is.
        */
        for i in 0..8 {
            if self.sprite_units[i].expansion_flop {
                self.raster_unit.mc_base[i] += 1;
                if self.raster_unit.mc_base[i] == 63 {
                    self.raster_unit.sprite_dma[i] = false;
                    self.sprite_units[i].display = false;
                }
            }
        }
    }

    fn update_sprite_expansion_ff(&mut self) {
        /*
        Section: 3.8. Sprites
        2. If the MxYE bit is set in the first phase of cycle 55, the expansion
           flip flop is inverted.
        */
        for sprite in self.sprite_units.iter_mut() {
            if sprite.config.expand_y {
                sprite.expansion_flop = !sprite.expansion_flop;
            }
        }
    }

    // -- Memory Ops

    fn c_access(&mut self) {
        if self.raster_unit.is_bad_line {
            let address = self.video_matrix | self.raster_unit.vc;
            self.raster_unit.vm_data_line[self.raster_unit.vmli] = self.mem.read(address);
            self.raster_unit.vm_color_line[self.raster_unit.vmli] =
                self.color_ram.borrow().read(self.raster_unit.vc) & 0x0f;
            // DEFERRED vic: memory no access unless ba down for 3 cycles
        }
    }

    fn g_access(&mut self) {
        if self.raster_unit.display_state {
            let g_data = match self.gfx_seq.config.mode {
                Mode::Text | Mode::McText => {
                    let address = self.char_base
                        | ((self.raster_unit.vm_data_line[self.raster_unit.vmli] as u16) << 3)
                        | self.raster_unit.rc as u16;
                    self.mem.read(address)
                }
                Mode::EcmText => {
                    let address = self.char_base
                        | (((self.raster_unit.vm_data_line[self.raster_unit.vmli] & 0x3f) as u16)
                            << 3) | self.raster_unit.rc as u16;
                    self.mem.read(address)
                }
                Mode::Bitmap | Mode::McBitmap => {
                    let address = self.char_base & 0x2000
                        | (self.raster_unit.vc << 3)
                        | self.raster_unit.rc as u16;
                    self.mem.read(address)
                }
                Mode::InvalidBitmap1 | Mode::InvalidBitmap2 => 0,
                _ => panic!(
                    "unsupported graphics mode {}",
                    self.gfx_seq.config.mode.value()
                ),
            };
            let c_data = self.raster_unit.vm_data_line[self.raster_unit.vmli];
            let c_color = self.raster_unit.vm_color_line[self.raster_unit.vmli];
            self.gfx_seq.set_data(c_data, c_color, g_data);
            /*
            Section: 3.7.2. VC and RC
            4. VC and VMLI are incremented after each g-access in display state.
            */
            self.raster_unit.vc += 1;
            self.raster_unit.vmli += 1;
        } else {
            let g_data = self.mem.read(0x3fff);
            self.gfx_seq.set_data(0, 0, g_data);
        }
    }

    fn p_access(&mut self, n: usize) {
        let address = self.video_matrix | 0x03f8 | n as u16;
        self.raster_unit.sprite_ptrs[n] = (self.mem.read(address) as u16) << 6;
    }

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
        let address = self.raster_unit.sprite_ptrs[n] | (self.raster_unit.mc[n] as u16);
        let data = self.mem.read(address);
        self.sprite_units[n].set_data(byte, data);
        self.raster_unit.mc[n] += 1;
    }
}

impl Chip for Vic {
    fn clock(&mut self) {
        match self.cycle {
            1 => {
                /*
                Section: 3.12. VIC interrupts
                 Bit|Name| Trigger condition
                 ---+----+-----------------------------------------------------------------
                  0 | RST| Reaching a certain raster line. The line is specified by writing
                    |    | to register $d012 and bit 7 of $d011 and internally stored by
                    |    | the VIC for the raster compare. The test for reaching the
                    |    | interrupt raster line is done in cycle 0 of every line (for line
                    |    | 0, in cycle 1).
                */
                if self.y == self.raster_compare && self.y != 0 {
                    self.trigger_irq(0);
                }
                self.update_display_on();
                self.update_bad_line();
                let sprite_dma = self.raster_unit.sprite_dma[3] | self.raster_unit.sprite_dma[4];
                self.set_ba(sprite_dma);
                self.p_access(3);
                if self.raster_unit.sprite_dma[3] {
                    self.s_access(3, 0);
                }
            }
            2 => {
                if self.y == self.raster_compare && self.y == 0 {
                    self.trigger_irq(0);
                }
                let sprite_dma = self.raster_unit.sprite_dma[3]
                    | self.raster_unit.sprite_dma[4]
                    | self.raster_unit.sprite_dma[5];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[3] {
                    self.s_access(3, 1);
                    self.s_access(3, 2);
                }
            }
            3 => {
                let sprite_dma = self.raster_unit.sprite_dma[4] | self.raster_unit.sprite_dma[5];
                self.set_ba(sprite_dma);
                self.p_access(4);
                if self.raster_unit.sprite_dma[4] {
                    self.s_access(4, 0);
                }
            }
            4 => {
                let sprite_dma = self.raster_unit.sprite_dma[4]
                    | self.raster_unit.sprite_dma[5]
                    | self.raster_unit.sprite_dma[6];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[4] {
                    self.s_access(4, 1);
                    self.s_access(4, 2);
                }
            }
            5 => {
                let sprite_dma = self.raster_unit.sprite_dma[5] | self.raster_unit.sprite_dma[6];
                self.set_ba(sprite_dma);
                self.p_access(5);
                if self.raster_unit.sprite_dma[5] {
                    self.s_access(5, 0);
                }
            }
            6 => {
                let sprite_dma = self.raster_unit.sprite_dma[5]
                    | self.raster_unit.sprite_dma[6]
                    | self.raster_unit.sprite_dma[7];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[5] {
                    self.s_access(5, 1);
                    self.s_access(5, 2);
                }
            }
            7 => {
                let sprite_dma = self.raster_unit.sprite_dma[6] | self.raster_unit.sprite_dma[7];
                self.set_ba(sprite_dma);
                self.p_access(6);
                if self.raster_unit.sprite_dma[6] {
                    self.s_access(6, 0);
                }
            }
            8 => {
                let sprite_dma = self.raster_unit.sprite_dma[6] | self.raster_unit.sprite_dma[7];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[6] {
                    self.s_access(6, 1);
                    self.s_access(6, 2);
                }
            }
            9 => {
                let sprite_dma = self.raster_unit.sprite_dma[7];
                self.set_ba(sprite_dma);
                self.p_access(7);
                if self.raster_unit.sprite_dma[7] {
                    self.s_access(7, 0);
                }
            }
            10 => {
                let sprite_dma = self.raster_unit.sprite_dma[7];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[7] {
                    self.s_access(7, 1);
                    self.s_access(7, 2);
                }
            }
            11 => {
                self.draw_border();
                self.set_ba(false);
            }
            12...13 => {
                self.draw_border();
                /*
                Section: 3.7.2. VC and RC
                3. If there is a Bad Line Condition in cycles 12-54, BA is set low and the
                   c-accesses are started. Once started, one c-access is done in the second
                   phase of every clock cycle in the range 15-54.
                */
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
            }
            14 => {
                self.draw_border();
                /*
                Section: 3.7.2. VC and RC
                2. In the first phase of cycle 14 of each line, VC is loaded from VCBASE
                   (VCBASE->VC) and VMLI is cleared. If there is a Bad Line Condition in
                   this phase, RC is also reset to zero.
                */
                self.raster_unit.vc = self.raster_unit.vc_base;
                self.raster_unit.vmli = 0;
                if self.raster_unit.is_bad_line {
                    self.raster_unit.rc = 0;
                }
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
            }
            15 => {
                self.draw_border();
                /*
                Section: 3.8. Sprites
                7. In the first phase of cycle 15, it is checked if the expansion flip flop
                   is set. If so, MCBASE is incremented by 2.
                */
                for i in 0..8 {
                    if self.sprite_units[i].expansion_flop {
                        self.raster_unit.mc_base[i] += 2;
                    }
                }
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
                self.c_access();
            }
            16 => {
                self.draw_border();
                self.update_sprite_dma_off();
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
            }
            17 => {
                self.draw_cycle_17_56();
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
            }
            18...54 => {
                self.draw();
                let is_bad_line = self.raster_unit.is_bad_line;
                self.set_ba(is_bad_line);
                self.g_access();
                self.c_access();
            }
            55 => {
                self.draw_cycle_17_56();
                self.update_sprite_dma_on();
                self.update_sprite_expansion_ff();
                let sprite_dma = self.raster_unit.sprite_dma[0];
                self.set_ba(sprite_dma);
                self.g_access();
            }
            56 => {
                self.draw_cycle_17_56();
                self.update_sprite_dma_on();
                let sprite_dma = self.raster_unit.sprite_dma[0];
                self.set_ba(sprite_dma);
            }
            57 => {
                self.draw_border();
                let sprite_dma = self.raster_unit.sprite_dma[0] | self.raster_unit.sprite_dma[1];
                self.set_ba(sprite_dma);
            }
            58 => {
                self.draw_border();
                /*
                Section: 3.7.2. VC and RC
                5. In the first phase of cycle 58, the VIC checks if RC=7. If so, the video
                   logic goes to idle state and VCBASE is loaded from VC (VC->VCBASE). If
                   the video logic is in display state afterwards (this is always the case
                   if there is a Bad Line Condition), RC is incremented.
                */
                if self.raster_unit.rc == 7 {
                    self.raster_unit.vc_base = self.raster_unit.vc;
                    if !self.raster_unit.is_bad_line {
                        self.raster_unit.display_state = false;
                    }
                }
                self.update_display_state();
                if self.raster_unit.display_state {
                    self.raster_unit.rc += 1;
                }
                /*
                Section: 3.8. Sprites
                4. In the first phase of cycle 58, the MC of every sprite is loaded from
                   its belonging MCBASE (MCBASE->MC) ...
                */
                for i in 0..8 {
                    self.raster_unit.mc[i] = self.raster_unit.mc_base[i];
                }
                self.update_sprite_display();
                let sprite_dma = self.raster_unit.sprite_dma[0] | self.raster_unit.sprite_dma[1];
                self.set_ba(sprite_dma);
                self.p_access(0);
                if self.raster_unit.sprite_dma[0] {
                    self.s_access(0, 0);
                }
            }
            59 => {
                self.draw_border();
                let sprite_dma = self.raster_unit.sprite_dma[0]
                    | self.raster_unit.sprite_dma[1]
                    | self.raster_unit.sprite_dma[2];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[0] {
                    self.s_access(0, 1);
                    self.s_access(0, 2);
                }
            }
            60 => {
                self.draw_border();
                let sprite_dma = self.raster_unit.sprite_dma[1] | self.raster_unit.sprite_dma[2];
                self.set_ba(sprite_dma);
                self.p_access(1);
                if self.raster_unit.sprite_dma[1] {
                    self.s_access(1, 0);
                }
            }
            61 => {
                self.draw_border();
                let sprite_dma = self.raster_unit.sprite_dma[1]
                    | self.raster_unit.sprite_dma[2]
                    | self.raster_unit.sprite_dma[3];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[1] {
                    self.s_access(1, 1);
                    self.s_access(1, 2);
                }
            }
            62 => {
                let sprite_dma = self.raster_unit.sprite_dma[2] | self.raster_unit.sprite_dma[3];
                self.set_ba(sprite_dma);
                self.p_access(2);
                if self.raster_unit.sprite_dma[2] {
                    self.s_access(2, 0);
                }
            }
            63 => {
                let sprite_dma = self.raster_unit.sprite_dma[2]
                    | self.raster_unit.sprite_dma[3]
                    | self.raster_unit.sprite_dma[4];
                self.set_ba(sprite_dma);
                if self.raster_unit.sprite_dma[2] {
                    self.s_access(2, 1);
                    self.s_access(2, 2);
                }
                self.border_unit.update_vertical_flop(self.y, self.den);
            }
            64 => {}
            65 => {}
            _ => panic!("invalid cycle"),
        }
        self.update_display_state();
        // Update counters/vsync
        self.cycle += 1;
        if self.cycle > self.spec.cycles_per_raster {
            self.cycle = 1;
            self.y += 1;
            if self.y >= self.spec.raster_lines {
                self.y = 0;
                self.raster_unit.display_on = false;
                /*
                Section: 3.7.2. VC and RC
                1. Once somewhere outside of the range of raster lines $30-$f7 (i.e.
                   outside of the Bad Line range), VCBASE is reset to zero. This is
                   presumably done in raster line 0, the exact moment cannot be determined
                   and is irrelevant.
                */
                self.raster_unit.vc_base = 0;
                self.vsync_flag.set(true);
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
        self.irq_control.reset();
        self.mux_unit.reset();
        self.raster_unit.reset();
        for sprite_unit in self.sprite_units.iter_mut() {
            sprite_unit.reset();
        }
        // Configuration
        self.char_base = 0x1000;
        self.den = true;
        self.frame_buffer_width = self.frame_buffer.borrow().get_dimension().0;
        self.raster_compare = 0;
        self.x_scroll = 0;
        self.y_scroll = 3;
        self.video_matrix = 0x0400;
        // Runtime State
        self.cycle = 1;
        self.y = 0x0100;
    }

    // I/O

    fn read(&mut self, reg: u8) -> u8 {
        let value = match reg {
            // Reg::M0X - Reg::M7X
            0x00 | 0x02 | 0x04 | 0x06 | 0x08 | 0x0a | 0x0c | 0x0e => {
                (self.sprite_units[(reg >> 1) as usize].config.x & 0x00ff) as u8
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                self.sprite_units[((reg - 1) >> 1) as usize].config.y
            }
            // Reg::MX8
            0x10 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprite_units[i].config.x.get_bit(8));
                }
                result
            }
            // Reg::CR1
            0x11 => {
                let mut result = 0;
                result
                    .set_bit(7, self.y.get_bit(8))
                    .set_bit(6, self.gfx_seq.config.mode.value().get_bit(2))
                    .set_bit(5, self.gfx_seq.config.mode.value().get_bit(1))
                    .set_bit(4, self.den)
                    .set_bit(3, self.border_unit.config.rsel);
                result | (self.y_scroll & 0x07)
            }
            // Reg::RASTER
            0x12 => (self.y & 0x00ff) as u8,
            // Reg::LPX
            0x13 => 0,
            // Reg::LPY
            0x14 => 0,
            // Reg::ME
            0x15 => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprite_units[i].config.enabled);
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
                    result.set_bit(i, self.sprite_units[i].config.expand_y);
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
            0x19 => self.irq_control.get_data() | 0x70,
            // Reg::IMR
            0x1a => self.irq_control.get_mask() | 0xf0,
            // Reg::MDP
            0x1b => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.mux_unit.config.data_priority[i]);
                }
                result
            }
            // Reg::MMC
            0x1c => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(
                        i,
                        self.sprite_units[i].config.mode == SpriteMode::Multicolor,
                    );
                }
                result
            }
            // Reg::MXE
            0x1d => {
                let mut result = 0;
                for i in 0..8 {
                    result.set_bit(i, self.sprite_units[i].config.expand_x);
                }
                result
            }
            // Reg::MM
            0x1e => {
                let result = self.mux_unit.mm_collision;
                self.mux_unit.mm_collision = 0;
                result
            }
            // Reg::MD
            0x1f => {
                let result = self.mux_unit.mb_collision;
                self.mux_unit.mb_collision = 0;
                result
            }
            // Reg::EC
            0x20 => self.border_unit.config.border_color | 0xf0,
            // Reg::B0C - Reg::B3C
            0x21...0x24 => self.gfx_seq.config.bg_color[(reg - 0x21) as usize] | 0xf0,
            // Reg::MM0 - Reg::MM1
            0x25...0x26 => self.sprite_units[0].config.multicolor[(reg - 0x25) as usize] | 0xf0,
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprite_units[(reg - 0x27) as usize].config.color | 0xf0,
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
                self.sprite_units[n].config.x =
                    (self.sprite_units[n].config.x & 0xff00) | (value as u16);
                self.sprite_units[n].config.x_screen =
                    self.map_sprite_to_screen(self.sprite_units[n].config.x);
            }
            // Reg::M0Y - Reg::M7Y
            0x01 | 0x03 | 0x05 | 0x07 | 0x09 | 0x0b | 0x0d | 0x0f => {
                let n = ((reg - 1) >> 1) as usize;
                self.sprite_units[n].config.y = value;
            }
            // Reg::MX8
            0x10 => for i in 0..8 as usize {
                self.sprite_units[i].config.x.set_bit(8, value.get_bit(i));
                self.sprite_units[i].config.x_screen =
                    self.map_sprite_to_screen(self.sprite_units[i].config.x);
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
                self.update_display_on();
                self.update_bad_line();
                if self.y == self.raster_compare {
                    self.trigger_irq(0);
                }
            }
            // Reg::RASTER
            0x12 => {
                let new_value = (self.raster_compare & 0xff00) | (value as u16);
                if self.raster_compare != new_value && self.y == new_value {
                    self.trigger_irq(0);
                }
                self.raster_compare = new_value;
            }
            // Reg::LPX
            0x13 => {}
            // Reg::LPY
            0x14 => {}
            // Reg::ME
            0x15 => for i in 0..8 as usize {
                self.sprite_units[i].config.enabled = value.get_bit(i);
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
                self.sprite_units[i].config.expand_y = value.get_bit(i);
                /*
                Section: 3.8. Sprites
                1. The expansion flip flip is set as long as the bit in MxYE in register
                   $d017 corresponding to the sprite is cleared.
                */
                self.sprite_units[i].expansion_flop = !self.sprite_units[i].config.expand_y;
            },
            // Reg::MEMPTR
            0x18 => {
                self.video_matrix = (((value & 0xf0) >> 4) as u16) << 10;
                self.char_base = (((value & 0x0f) >> 1) as u16) << 11;
            }
            // Reg::IRR
            0x19 => {
                self.irq_control.clear_events(value & 0x0f);
                self.irq_line
                    .borrow_mut()
                    .set_low(IrqSource::Vic.value(), false);
            }
            // Reg::IMR
            0x1a => {
                self.irq_control.set_mask(value & 0x0f);
                self.irq_line
                    .borrow_mut()
                    .set_low(IrqSource::Vic.value(), self.irq_control.is_triggered());
            }
            // Reg::MDP
            0x1b => for i in 0..8 as usize {
                self.mux_unit.config.data_priority[i] = value.get_bit(i);
            },
            // Reg::MMC
            0x1c => for i in 0..8 as usize {
                self.sprite_units[i].config.mode = if !value.get_bit(i) {
                    SpriteMode::Standard
                } else {
                    SpriteMode::Multicolor
                };
            },
            // Reg::MXE
            0x1d => for i in 0..8 as usize {
                self.sprite_units[i].config.expand_x = value.get_bit(i);
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
                self.sprite_units[i].config.multicolor[reg as usize - 0x25] = value & 0x0f;
            },
            // Reg::M0C - Reg::M7C
            0x27...0x2e => self.sprite_units[reg as usize - 0x27].config.color = value & 0x0f,
            _ => {}
        }
    }
}
