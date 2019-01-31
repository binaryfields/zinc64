// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use alloc::prelude::*;
use zinc64_loader::{Reader, Result};
use zorio::io::{BufRead, Read};
use zorio::cursor::Cursor;

pub struct ImageReader<'a>(pub Cursor<&'a[u8]>);

impl <'a> Reader for ImageReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
            .map_err(|err| err.message.to_owned())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.0.read_to_end(buf)
            .map_err(|err| err.message.to_owned())
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.0.read_exact(buf)
            .map_err(|err| err.message.to_owned())
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}
