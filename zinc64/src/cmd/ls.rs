// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::result::Result;

use super::Handler;

pub struct LsCommand {
    path: Option<String>,
}

impl LsCommand {
    pub fn new(path: Option<String>) -> Self {
        Self { path }
    }

    pub fn help() -> &'static str {
        "ls [<dir>]"
    }
}

impl Handler for LsCommand {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        let path = self.path.as_ref().map(|s| s.as_str()).unwrap_or("./");
        let dir = Path::new(path);
        if dir.is_dir() {
            let entries = fs::read_dir(dir).map_err(|err| format!("{}", err))?;
            for entry in entries {
                let path = format!("{}", entry.unwrap().path().display());
                if !path.is_empty() {
                    out.write(path.as_ref()).map_err(|err| format!("{}", err))?;
                    out.write("\n".as_bytes())
                        .map_err(|err| format!("{}", err))?;
                }
            }
        }
        Ok(())
    }
}
