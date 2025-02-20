// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::Write;
use std::result::Result;

use super::Handler;
use crate::session::Session;

pub struct ResetCommand<'a> {
    session: &'a mut Session,
    hard: bool,
}

impl<'a> ResetCommand<'a> {
    pub fn new(session: &'a mut Session, hard: bool) -> Self {
        Self { session, hard }
    }

    pub fn help() -> &'static str {
        "reset [<type>]"
    }
}

impl<'a> Handler for ResetCommand<'a> {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        self.session.reset(self.hard);
        out.write("Reset system.\n".as_bytes())
            .map_err(|err| format!("{}", err))?;
        Ok(())
    }
}
