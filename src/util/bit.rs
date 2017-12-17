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

#[inline(always)]
pub fn get(value: u8, pos: u8) -> u8 {
    if (value & (1 << pos)) != 0 {
        1
    } else {
        0
    }
}

#[inline(always)]
pub fn set(value: u8, pos: u8, enabled: bool) -> u8 {
    if enabled {
        value | (1 << pos)
    } else {
        value & !(1 << pos)
    }
}

#[inline(always)]
pub fn test(value: u8, pos: u8) -> bool {
    value & (1 << pos) != 0
}

#[inline(always)]
pub fn value(pos: u8, enabled: bool) -> u8 {
    if enabled {
        1 << pos
    } else {
        0
    }
}

#[inline(always)]
pub fn get_u16(value: u16, pos: u8) -> u8 {
    if (value & (1 << pos)) != 0 {
        1
    } else {
        0
    }
}

#[inline(always)]
pub fn set_u16(value: u16, pos: u8, enabled: bool) -> u16 {
    if enabled {
        value | ((1 << pos) as u16)
    } else {
        value & !((1 << pos) as u16)
    }
}
