// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::VicModel;

#[derive(Clone, Copy)]
pub struct Spec {
    pub raster_lines: u16,
    pub cycles_per_raster: u16,
    pub first_x_coord: u16,
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
        Spec {
            raster_lines: 263,
            cycles_per_raster: 65,
            first_x_coord: 0x19c,
        }
    }

    fn pal() -> Spec {
        Spec {
            raster_lines: 312,
            cycles_per_raster: 63,
            first_x_coord: 0x194,
        }
    }
}
