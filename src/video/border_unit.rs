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

pub struct Config {
    pub border_color: u8,
    pub csel: bool,
    pub rsel: bool,
}

impl Config {
    pub fn new() -> Self {
        Config {
            border_color: 0x0e,
            csel: true,
            rsel: true,
        }
    }

    pub fn reset(&mut self) {
        self.border_color = 0x0e;
        self.csel = true;
        self.rsel = true;
    }
}

pub struct BorderUnit {
    pub config: Config,
    border_mff: bool,
    border_vff: bool,
}

impl BorderUnit {
    pub fn new() -> Self {
        BorderUnit {
            config: Config::new(),
            border_mff: false,
            border_vff: false,
        }
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
    pub fn is_enabled(&self) -> bool {
        self.border_mff || self.border_vff
    }

    pub fn reset(&mut self) {
        self.config.reset();
        self.border_mff = false;
        self.border_vff = false;
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
    pub fn update_main_ff(&mut self, x: u16, y: u16, den: bool) {
        /*
        Section: 3.9. The border unit
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
        if self.config.csel {
            if x == Self::map_sprite_to_screen(0x18 - 4) {
                self.update_vertical_ff(y, den);
                if !self.border_vff {
                    self.border_mff = false;
                }
            } else if x == Self::map_sprite_to_screen(0x158 - 4) {
                self.border_mff = true;
            }
        } else {
            if x == Self::map_sprite_to_screen(0x1f - 4) {
                self.update_vertical_ff(y, den);
                if !self.border_vff {
                    self.border_mff = false;
                }
            } else if x == Self::map_sprite_to_screen(0x14f - 4) {
                self.border_mff = true;
            }
        }
    }

    #[inline]
    pub fn update_vertical_ff(&mut self, y: u16, den: bool) {
        /*
        Section: 3.9. The border unit
        2. If the Y coordinate reaches the bottom comparison value in cycle 63, the
           vertical border flip flop is set.
        3. If the Y coordinate reaches the top comparison value in cycle 63 and the
           DEN bit in register $d011 is set, the vertical border flip flop is
           reset.
        */
        if self.config.rsel {
            if y == 51 && den {
                self.border_vff = false;
            } else if y == 251 {
                self.border_vff = true;
            }
        } else {
            if y == 55 && den {
                self.border_vff = false;
            } else if y == 247 {
                self.border_vff = true;
            }
        }
    }
}
