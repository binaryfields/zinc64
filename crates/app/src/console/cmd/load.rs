// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::Write;
use std::path::Path;
use std::result::Result;

use super::Handler;
use crate::session::Session;

pub struct LoadCommand<'a> {
    session: &'a mut Session,
    path: String,
}

impl<'a> LoadCommand<'a> {
    pub fn new(session: &'a mut Session, path: String) -> Self {
        Self { session, path }
    }

    pub fn help() -> &'static str {
        "load <image_path>"
    }
}

impl<'a> Handler for LoadCommand<'a> {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        self.session.open_image(Path::new(&self.path))?;
        out.write("Loaded image.\n".as_bytes())
            .map_err(|err| format!("{}", err))?;
        Ok(())
    }
}
