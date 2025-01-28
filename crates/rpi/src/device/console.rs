// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::fmt;

use super::uart;

pub enum Output {
    None,
    Uart(uart::Uart),
}

pub struct Console {
    output: Output,
}

impl Console {
    pub const fn new() -> Self {
        Console {
            output: Output::None,
        }
    }

    pub fn set_output(&mut self, output: Output) {
        self.output = output;
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match &mut self.output {
            Output::Uart(uart) => uart.puts(s),
            _ => (),
        }
        Ok(())
    }
}
