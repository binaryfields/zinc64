// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use cgmath::{ElementWise, Vector4};

#[derive(Copy, Clone, Debug)]
pub struct Color(pub Vector4<f32>);

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color(Vector4::new(r, g, b, a))
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color::from_rgba(r, g, b, 255)
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        let multiplier = 1.0 / 255.0;
        Color::new(
            f32::from(r) * multiplier,
            f32::from(g) * multiplier,
            f32::from(b) * multiplier,
            f32::from(a) * multiplier,
        )
    }

    #[inline]
    pub fn r(&self) -> f32 {
        self.0.x
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.0.y
    }

    #[inline]
    pub fn b(&self) -> f32 {
        self.0.z
    }

    #[inline]
    pub fn a(&self) -> f32 {
        self.0.w
    }

    pub fn rgba(&self) -> u32 {
        let tmp = self
            .0
            .mul_element_wise(Vector4::new(255.0, 255.0, 255.0, 255.0));
        let color = tmp.cast::<u8>().unwrap();
        return (color.x as u32)
            | (color.y as u32) << 8
            | (color.z as u32) << 16
            | (color.w as u32) << 24;
    }

    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
}
