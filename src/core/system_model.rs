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

#[derive(Clone, Copy)]
pub enum SidModel {
    Mos6581,
    Mos8580,
}

#[derive(Copy, Clone)]
pub enum VicModel {
    Mos6567, // NTSC
    Mos6569, // PAL
}

pub struct SystemModel {
    pub color_ram: usize,
    pub cpu_freq: u32,
    pub cycles_per_frame: u16,
    pub frame_buffer_size: (u32, u32),
    pub memory_size: usize,
    pub refresh_rate: f32,
    pub sid_model: SidModel,
    pub vic_model: VicModel,
    pub viewport_offset: (u32, u32),
    pub viewport_size: (u32, u32),
}

impl SystemModel {
    pub fn from(model: &str) -> SystemModel {
        match model {
            "ntsc" => SystemModel::c64_ntsc(),
            "pal" => SystemModel::c64_pal(),
            "c64-ntsc" => SystemModel::c64_ntsc(),
            "c64-pal" => SystemModel::c64_pal(),
            _ => panic!("invalid model {}", model),
        }
    }

    pub fn c64_ntsc() -> SystemModel {
        SystemModel {
            color_ram: 1024,
            cpu_freq: 1_022_727,
            cycles_per_frame: 17095,
            frame_buffer_size: (512, 263),
            memory_size: 65536,
            refresh_rate: 59.826,
            sid_model: SidModel::Mos6581,
            vic_model: VicModel::Mos6567,
            viewport_offset: (77, 16),
            viewport_size: (418, 235),
        }
    }

    pub fn c64_pal() -> SystemModel {
        SystemModel {
            color_ram: 1024,
            cpu_freq: 985_248,
            cycles_per_frame: 19656,
            frame_buffer_size: (504, 312),
            memory_size: 65536,
            refresh_rate: 50.125,
            sid_model: SidModel::Mos6581,
            vic_model: VicModel::Mos6569,
            viewport_offset: (76, 16),
            viewport_size: (403, 284),
        }
    }
}
