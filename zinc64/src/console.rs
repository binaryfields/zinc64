// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::{Error, Write};

use crate::util::{circular_buffer, CircularBuffer};

pub struct Console {
    // Configuration
    pub cols: u32,
    pub rows: u32,
    // Runtime state
    buffer: CircularBuffer<u8>,
    buffer_pos_snapshot: (usize, usize),
    screen_pos: usize,
    screen_pos_snapshot: usize,
}

impl Console {
    pub fn new(cols: u32, rows: u32, buffer_size: usize) -> Self {
        Console {
            rows,
            cols,
            buffer: CircularBuffer::new(buffer_size),
            buffer_pos_snapshot: (0, 0),
            screen_pos: 0,
            screen_pos_snapshot: 0,
        }
    }

    pub fn advance(&mut self) {
        if self.buffer.remaining(self.screen_pos) > ((self.rows - 1) * self.cols) as usize {
            for _ in 0..self.cols {
                self.screen_pos = self.buffer.advance(self.screen_pos);
            }
        }
    }

    pub fn print(&mut self, text: &[u8]) {
        let mut col = self.buffer.remaining(self.screen_pos) as u32 % self.cols;
        for ch in text {
            if *ch == '\n' as u8 {
                while col < self.cols {
                    self.buffer.push(' ' as u8);
                    col += 1;
                }
            } else {
                self.buffer.push(*ch);
                col += 1;
            }
            if col == self.cols {
                col = 0;
                self.advance();
            }
        }
    }

    pub fn restore_pos(&mut self) {
        self.buffer.restore_pos(self.buffer_pos_snapshot);
        self.screen_pos = self.screen_pos_snapshot;
    }

    pub fn save_pos(&mut self) {
        self.buffer_pos_snapshot = self.buffer.snapshot_pos();
        self.screen_pos_snapshot = self.screen_pos;
    }

    pub fn screen_data(&self) -> circular_buffer::Iter<u8> {
        self.buffer.iter_from(self.screen_pos)
    }
}

impl Write for Console {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.print(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
