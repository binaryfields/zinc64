// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.
#![allow(unused)]

pub struct CircularBuffer<T: Copy + Default> {
    buffer: Vec<T>,
    head: usize,
    tail: usize,
}

impl<T: Copy + Default> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![T::default(); capacity],
            head: 0,
            tail: 0,
        }
    }

    pub fn advance(&self, pos: usize) -> usize {
        if pos + 1 == self.capacity() {
            0
        } else {
            pos + 1
        }
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn iter_from(&self, pos: usize) -> Iter<T> {
        Iter::new(self, pos)
    }

    pub fn len(&self) -> usize {
        self.remaining(self.head)
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.head == self.tail {
            None
        } else {
            let value = Some(self.buffer[self.head]);
            self.head = self.advance(self.head);
            value
        }
    }

    pub fn push(&mut self, value: T) {
        self.buffer[self.tail] = value;
        self.tail = self.advance(self.tail);
        if self.tail == self.head {
            self.head = self.advance(self.head);
        }
    }

    pub fn remaining(&self, pos: usize) -> usize {
        if self.tail == self.head {
            0
        } else if self.tail > pos {
            self.tail - pos
        } else {
            self.capacity() - pos + self.tail
        }
    }

    pub fn reset(&mut self) {
        for value in self.buffer.iter_mut() {
            *value = T::default();
        }
        self.head = 0;
        self.tail = 0;
    }

    pub fn restore_pos(&mut self, pos: (usize, usize)) {
        self.head = pos.0;
        self.tail = pos.1;
    }

    pub fn snapshot_pos(&self) -> (usize, usize) {
        (self.head, self.tail)
    }
}

pub struct Iter<'a, T: 'a + Copy + Default> {
    buffer: &'a CircularBuffer<T>,
    pos: usize,
}

impl<'a, T: 'a + Copy + Default> Iter<'a, T> {
    pub fn new(buffer: &'a CircularBuffer<T>, pos: usize) -> Self {
        Iter { buffer, pos }
    }
}

impl<'a, T: 'a + Copy + Default> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos != self.buffer.tail {
            let value = &self.buffer.buffer[self.pos];
            self.pos = self.buffer.advance(self.pos);
            Some(value)
        } else {
            None
        }
    }
}
