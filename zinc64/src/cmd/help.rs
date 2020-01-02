// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io::Write;
use std::result::Result;

use super::load::LoadCommand;
use super::ls::LsCommand;
use super::reset::ResetCommand;
use super::Handler;

pub struct HelpCommand {
    command: Option<String>,
}

impl HelpCommand {
    pub fn new(command: Option<String>) -> Self {
        Self { command }
    }

    fn format(syntax: &str, short: &str) -> String {
        let abbr_line = if short.is_empty() {
            String::new()
        } else {
            format!("Shortname: {}\n", short)
        };
        format!("Syntax: {}\n{}", syntax, abbr_line)
    }

    fn help() -> String {
        let mut buffer = String::new();
        buffer.push_str("load\n");
        buffer.push_str("ls\n");
        buffer.push_str("reset\n");
        buffer.push_str("exit (x)\n");
        buffer.push_str("help (?)\n");
        buffer
    }
}

impl Handler for HelpCommand {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String> {
        if let Some(command) = self.command.as_ref() {
            let text = match command.trim().to_lowercase().as_str() {
                "load" => HelpCommand::format(LoadCommand::help(), ""),
                "ls" => HelpCommand::format(LsCommand::help(), ""),
                "reset" => HelpCommand::format(ResetCommand::help(), ""),
                _ => format!("Invalid command: {}", command),
            };
            out.write(text.as_bytes())
                .map_err(|err| format!("{}", err))?;
        } else {
            out.write(HelpCommand::help().as_bytes())
                .map_err(|err| format!("{}", err))?;
        }
        Ok(())
    }
}
