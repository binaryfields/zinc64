// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]

use cgmath::Vector2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub p1: Vector2<f32>,
    pub p2: Vector2<f32>,
}

impl Rect {
    pub fn new(origin: Vector2<f32>, size: Vector2<f32>) -> Self {
        Rect {
            p1: origin,
            p2: origin + size,
        }
    }

    pub fn from_points(p1: Vector2<f32>, p2: Vector2<f32>) -> Self {
        Rect { p1, p2 }
    }

    pub fn origin(&self) -> Vector2<f32> {
        self.p1
    }

    pub fn size(&self) -> Vector2<f32> {
        self.p2 - self.p1
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RectI {
    pub p1: Vector2<i32>,
    pub p2: Vector2<i32>,
}

impl RectI {
    pub fn new(origin: Vector2<i32>, size: Vector2<i32>) -> Self {
        RectI {
            p1: origin,
            p2: origin + size,
        }
    }

    pub fn from_points(p1: Vector2<i32>, p2: Vector2<i32>) -> Self {
        RectI { p1, p2 }
    }

    pub fn origin(&self) -> Vector2<i32> {
        self.p1
    }

    pub fn size(&self) -> Vector2<i32> {
        self.p2 - self.p1
    }
}
