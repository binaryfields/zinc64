// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Rect {
            x,
            y,
            w: width,
            h: height,
        }
    }

    pub fn new_with_origin(origin: (u32, u32), size: (u32, u32)) -> Self {
        Self::new(origin.0, origin.1, size.0, size.1)
    }
}
