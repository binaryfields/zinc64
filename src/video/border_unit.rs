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

use super::spec::Spec;

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
    spec: Spec,
    pub config: Config,
    main_flop: bool,
    vertical_flop: bool,
}

impl BorderUnit {
    pub fn new(spec: Spec) -> Self {
        BorderUnit {
            spec,
            config: Config::new(),
            main_flop: false,
            vertical_flop: false,
        }
    }

    #[inline]
    fn map_sprite_to_screen(&self, x: u16) -> u16 {
        match self.spec.first_x_coord {
            0x194 => {
                match x {
                    0x000...0x193 => x + 0x64, // 0x1f7 - 0x193
                    0x194...0x1f7 => x - 0x194,
                    _ => panic!("invalid sprite coords {}", x),
                }
            },
            0x19c => {
                match x {
                    0x000...0x19b => x + 0x64, // 0x1ff - 0x19b
                    0x19c...0x1ff => x - 0x19c,
                    _ => panic!("invalid sprite coords {}", x),
                }
            },
            _ => panic!("invalid sprite coords {}", x),
        }
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.main_flop || self.vertical_flop
    }

    #[inline]
    pub fn output(&self) -> u8 {
        self.config.border_color
    }

    pub fn reset(&mut self) {
        self.config.reset();
        self.main_flop = false;
        self.vertical_flop = false;
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
    pub fn update_main_flop(&mut self, x: u16, y: u16, den: bool) {
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
        if self.config.csel {
            if x == self.map_sprite_to_screen(0x18) {
                self.update_vertical_flop(y, den);
                if !self.vertical_flop {
                    self.main_flop = false;
                }
            } else if x == self.map_sprite_to_screen(0x158) {
                self.main_flop = true;
            }
        } else {
            if x == self.map_sprite_to_screen(0x1f) {
                self.update_vertical_flop(y, den);
                if !self.vertical_flop {
                    self.main_flop = false;
                }
            } else if x == self.map_sprite_to_screen(0x14f) {
                self.main_flop = true;
            }
        }
    }

    #[inline]
    pub fn update_vertical_flop(&mut self, y: u16, den: bool) {
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
                self.vertical_flop = false;
            } else if y == 251 {
                self.vertical_flop = true;
            }
        } else {
            if y == 55 && den {
                self.vertical_flop = false;
            } else if y == 247 {
                self.vertical_flop = true;
            }
        }
    }
}
