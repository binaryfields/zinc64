// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub struct CircularBuffer<T: Copy + Default> {
    buffer: Vec<T>,
    capacity: usize,
    count: usize,
    head: usize,
    tail: usize,
}

impl<T: Copy + Default> CircularBuffer<T> {
    pub fn new(capacity: usize) -> CircularBuffer<T> {
        CircularBuffer {
            buffer: vec![T::default(); capacity],
            capacity,
            count: 0,
            head: 0,
            tail: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn pop(&mut self) -> T {
        if self.count == 0 {
            T::default()
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

    pub fn push(&mut self, value: T) -> bool {
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
        for value in self.buffer.iter_mut() {
            *value = T::default();
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
        assert_eq!(true, buffer.push(1));
        assert_eq!(true, buffer.push(2));
        assert_eq!(false, buffer.push(3));
    }
}
