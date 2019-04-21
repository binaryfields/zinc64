// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::result::Result;

use bit_field::BitField;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::util::Font;

pub fn draw_char(
    canvas: &mut Canvas<Window>,
    font: &Font,
    ch: u8,
    x_start: u32,
    y_start: u32,
    fg_color: Color,
    bg_color: Color,
) -> Result<(), String> {
    let glyph = font.get_glyph(ch);
    for y in y_start..(y_start + font.get_height()) {
        let mut data = glyph[(y - y_start) as usize * font.get_width() as usize / 8];
        for x in x_start..(x_start + font.get_width()) {
            let color = if data.get_bit(7) {
                fg_color
            } else {
                bg_color
            };
            canvas.set_draw_color(color);
            canvas.draw_point(Point::new(x as i32, y as i32))?;
            data = data << 1;
        }
    }
    Ok(())
}
