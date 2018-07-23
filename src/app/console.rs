// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64::system::C64;

pub struct ConsoleApp {
    c64: C64,
}

impl ConsoleApp {
    pub fn new(c64: C64) -> ConsoleApp {
        ConsoleApp { c64 }
    }

    pub fn run(&mut self) {
        loop {
            self.c64.run_frame();
            {
                let rt = self.c64.get_frame_buffer();
                rt.borrow_mut().set_sync(false);
            }
            if self.c64.is_cpu_jam() {
                let cpu = self.c64.get_cpu();
                warn!(target: "main", "CPU JAM detected at 0x{:x}", cpu.get_pc());
                break;
            }
        }
    }
}
