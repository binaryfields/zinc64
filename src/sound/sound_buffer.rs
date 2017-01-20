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

pub struct SoundBuffer {
    buffer: Vec<i16>,
    head: usize,
    tail: usize,
}

impl SoundBuffer {
    pub fn new(capacity: usize) -> SoundBuffer {
        SoundBuffer {
            buffer: vec![0; capacity],
            head: 0,
            tail: 0,
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
        self.head = 0;
        self.tail = 0;
    }

    pub fn pop(&mut self) -> i16 {
        let value = self.buffer[self.head];
        self.head += 1;
        if self.head == self.buffer.len() {
            self.head = 0;
        }
        value
    }

    pub fn push(&mut self, value: i16) {
        self.buffer[self.tail] = value;
        self.tail += 1;
        if self.tail == self.buffer.len() {
            self.tail = 0;
        }
    }
}