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

use resid;
use video::vic;

pub struct Model {
    pub cpu_freq: u32,
    pub cycles_per_frame: u16,
    pub memory_size: usize,
    pub refresh_rate: f32,
    pub sid_model: resid::ChipModel,
    pub vic_model: vic::ChipModel,
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
            cpu_freq: 1_022_727,
            cycles_per_frame: 17095,
            memory_size: 65536,
            refresh_rate: 59.826,
            sid_model: resid::ChipModel::Mos6581,
            vic_model: vic::ChipModel::Mos6567,
        }
    }

    fn c64_pal() -> Model {
        Model {
            cpu_freq: 985_248,
            cycles_per_frame: 19656,
            memory_size: 65536,
            refresh_rate: 50.125,
            sid_model: resid::ChipModel::Mos6581,
            vic_model: vic::ChipModel::Mos6569,
        }
    }
}
