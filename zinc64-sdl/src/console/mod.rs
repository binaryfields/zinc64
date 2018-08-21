// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64::system::C64;

pub struct ConsoleApp {
    c64: C64,
}

impl ConsoleApp {
    pub fn new(c64: C64) -> Self {
        Self { c64 }
    }

    pub fn run(&mut self) {
        loop {
            self.c64.run_frame();
            self.c64.reset_vsync();
            if self.c64.is_cpu_jam() {
                warn!(target: "main", "CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
                break;
            }
        }
    }
}
