// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

pub trait Tape {
    fn read_pulse(&mut self) -> Option<u32>;
    fn seek(&mut self, pos: usize) -> bool;
}
