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

#[derive(Copy, Clone, Debug)]
pub struct Dimension {
    pub width: u16,
    pub height: u16,
}

impl Dimension {
    pub fn new(width: u16, height: u16) -> Dimension {
        Dimension {
            width,
            height,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

impl Rect {
    #[allow(dead_code)]
    pub fn new(left: u16, right: u16, top: u16, bottom: u16) -> Rect {
        Rect {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn new_with_dim(left: u16, top: u16, size: Dimension) -> Rect {
        Rect {
            left,
            right: left + size.width - 1,
            top,
            bottom: top + size.height - 1,
        }
    }

    #[inline]
    pub fn contains(&self, x: u16, y: u16) -> bool {
        y >= self.top && y <= self.bottom && x >= self.left && x <= self.right
    }

    pub fn offset(&self, dx: i16, dy: i16) -> Rect {
        Rect {
            left: (self.left as i16 + dx) as u16,
            right: (self.right as i16 + dx) as u16,
            top: (self.top as i16 + dy) as u16,
            bottom: (self.bottom as i16 + dy) as u16,
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn size(&self) -> Dimension {
        Dimension::new(self.right - self.left + 1, self.bottom - self.top + 1)
    }
}
