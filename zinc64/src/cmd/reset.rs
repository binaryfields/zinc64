// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::Write;
use std::result::Result;

use zinc64_system::C64;

use super::Handler;

pub struct ResetCommand<'a> {
    c64: &'a mut C64,
    hard: bool,
}

impl<'a> ResetCommand<'a> {
    pub fn new(c64: &'a mut C64, hard: bool) -> Self {
        Self { c64, hard }
    }

    pub fn help() -> &'static str {
        "reset [<type>]"
    }
}

impl<'a> Handler for ResetCommand<'a> {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        self.c64.reset(self.hard);
        out.write("Reset system.\n".as_bytes())
            .map_err(|err| format!("{}", err))?;
        Ok(())
    }
}
