// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

mod help;
mod load;
mod ls;
mod parser;
mod reset;

use std::io::Write;
use std::result::Result;

use zinc64_emu::system::C64;

use self::help::HelpCommand;
use self::load::LoadCommand;
use self::ls::LsCommand;
use self::parser::Parser;
use self::reset::ResetCommand;

pub enum Cmd {
    Help(Option<String>),
    Load(String),
    Ls(Option<String>),
    Reset(bool),
}

trait Handler {
    fn run(&mut self, out: &mut dyn Write) -> Result<(), String>;
}

pub struct Executor {
    parser: Parser,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            parser: Parser::new(),
        }
    }

    pub fn execute(&self, input: &str, c64: &mut C64, out: &mut dyn Write) -> Result<(), String> {
        let command = self.parser.parse(input)?;
        let mut handler: Box<dyn Handler> = match command {
            Cmd::Load(path) => Box::new(LoadCommand::new(c64, path)),
            Cmd::Ls(path) => Box::new(LsCommand::new(path)),
            Cmd::Reset(hard) => Box::new(ResetCommand::new(c64, hard)),
            Cmd::Help(command) => Box::new(HelpCommand::new(command)),
        };
        handler.run(out)
    }
}
