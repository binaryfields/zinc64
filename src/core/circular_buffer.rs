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

pub struct CircularBuffer {
    buffer: Vec<i16>,
    capacity: usize,
    count: usize,
    head: usize,
    tail: usize,
}

impl CircularBuffer {
    pub fn new(capacity: usize) -> CircularBuffer {
        CircularBuffer {
            buffer: vec![0; capacity],
            capacity,
            count: 0,
            head: 0,
            tail: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn pop(&mut self) -> i16 {
        if self.count == 0 {
            0
        } else {
            let value = self.buffer[self.head];
            self.count -= 1;
            self.head += 1;
            if self.head == self.buffer.len() {
                self.head = 0;
            }
            value
        }
    }

    #[inline]
    pub fn push(&mut self, value: i16) -> bool {
        if self.count == self.capacity {
            false
        } else {
            self.buffer[self.tail] = value;
            self.count += 1;
            self.tail += 1;
            if self.tail == self.buffer.len() {
                self.tail = 0;
            }
            true
        }
    }

    pub fn reset(&mut self) {
        for i in 0..self.buffer.len() {
            self.buffer[i] = 0;
        }
        self.count = 0;
        self.head = 0;
        self.tail = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len() {
        let mut buffer = CircularBuffer::new(4);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(3, buffer.len());
    }

    #[test]
    fn len_with_overflow() {
        let mut buffer = CircularBuffer::new(4);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4);
        buffer.pop();
        buffer.pop();
        buffer.push(5);
        buffer.push(6);
        assert_eq!(4, buffer.len());
    }

    #[test]
    fn push_and_pop() {
        let mut buffer = CircularBuffer::new(4);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(1, buffer.pop());
        assert_eq!(2, buffer.pop());
        assert_eq!(3, buffer.pop());
    }

    #[test]
    fn push_overflow() {
        let mut buffer = CircularBuffer::new(2);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(3, buffer.pop());
        assert_eq!(2, buffer.pop());
    }
}