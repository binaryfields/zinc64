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

pub struct Model {
    pub color_ram: usize,
    pub cpu_freq: u32,
    pub cycles_per_frame: u16,
    pub frame_buffer_size: (usize, usize),
    pub memory_size: usize,
    pub refresh_rate: f32,
    pub sid_model: SidModel,
    pub vic_model: VicModel,
}

impl Model {
    pub fn from(model: &str) -> Model {
        match model {
            "ntsc" => Model::c64_ntsc(),
            "pal" => Model::c64_pal(),
            "c64-ntsc" => Model::c64_ntsc(),
            "c64-pal" => Model::c64_pal(),
            _ => panic!("invalid model {}", model),
        }
    }

    fn c64_ntsc() -> Model {
        Model {
            color_ram: 1024,
            cpu_freq: 1_022_727,
            cycles_per_frame: 17095,
            frame_buffer_size: (403, 250),
            memory_size: 65536,
            refresh_rate: 59.826,
            sid_model: SidModel::Mos6581,
            vic_model: VicModel::Mos6567,
        }
    }

    fn c64_pal() -> Model {
        Model {
            color_ram: 1024,
            cpu_freq: 985_248,
            cycles_per_frame: 19656,
            frame_buffer_size: (403, 284),
            memory_size: 65536,
            refresh_rate: 50.125,
            sid_model: SidModel::Mos6581,
            vic_model: VicModel::Mos6569,
        }
    }
}
