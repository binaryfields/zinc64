// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use zinc64_emu::system::C64;
use zinc64_loader::Loaders;

use crate::util::FileReader;
use std::{fs, io};

pub enum Cmd {
    Load(String),
    Ls(Option<String>),
    Reset(bool),
    Exit,
    Help(Option<String>),
}

pub struct CmdHandler {
    parser: Parser,
}

impl CmdHandler {
    pub fn new() -> Self {
        CmdHandler {
            parser: Parser::new(),
        }
    }

    pub fn handle(&mut self, input: &str, c64: &mut C64) -> Result<String, String> {
        let command = self.parser.parse(input)?;
        self.execute(command, c64)
    }

    fn execute(&self, command: Cmd, c64: &mut C64) -> Result<String, String> {
        match command {
            Cmd::Load(path) => self.cmd_load(c64, &path),
            Cmd::Ls(path) => self.cmd_ls(path).map_err(|e| e.to_string()),
            Cmd::Reset(hard) => self.cmd_reset(c64, hard),
            Cmd::Help(command) => Help::help(command),
            _ => Err("Invalid command".to_string()),
        }
    }

    fn cmd_load(&self, c64: &mut C64, path: &String) -> Result<String, String> {
        let path = Path::new(path);
        let ext = path.extension().map(|s| s.to_str().unwrap());
        let loader = Loaders::from_ext(ext)?;
        let file = File::open(path).map_err(|err| format!("{}", err))?;
        let mut reader = FileReader(BufReader::new(file));
        let mut autostart = loader.autostart(&mut reader)?;
        autostart.execute(c64);
        Ok("Loaded image.\n".to_string())
    }

    fn cmd_ls(&self, path: Option<String>) -> io::Result<String> {
        let path = path.unwrap_or("./".to_string());
        let dir = Path::new(&path);
        let mut buffer = String::new();
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let path = format!("{}", entry.unwrap().path().display());
                if !path.is_empty() {
                    buffer.push_str(path.as_ref());
                    buffer.push('\n');
                }
            }
        }
        Ok(buffer)
    }

    fn cmd_reset(&self, c64: &mut C64, hard: bool) -> Result<String, String> {
        c64.reset(hard);
        Ok("Reset system.\n".to_string())
    }
}

struct Parser {
    radix: u32,
}

impl Parser {
    pub fn new() -> Self {
        Self { radix: 16 }
    }

    #[allow(unused)]
    pub fn get_radix(&self) -> u32 {
        self.radix
    }

    #[allow(unused)]
    pub fn set_radix(&mut self, radix: u32) {
        self.radix = radix;
    }

    pub fn parse(&self, input: &str) -> Result<Cmd, String> {
        let mut tokens = input.split_whitespace();
        if let Some(command) = tokens.next() {
            match command.to_lowercase().as_str() {
                "load" => self.parse_load(&mut tokens),
                "ls" => self.parse_ls(&mut tokens),
                "reset" => self.parse_reset(&mut tokens),
                "exit" | "x" => self.parse_exit(&mut tokens),
                "help" | "?" => self.parse_help(&mut tokens),
                _ => Err(format!("Invalid command {}", input)),
            }
        } else {
            Err(format!("Invalid command {}", input))
        }
    }

    fn parse_exit(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Exit)
    }

    fn parse_help(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<Cmd, String> {
        let command = tokens.next().map(|s| s.to_string());
        self.ensure_eos(tokens)?;
        Ok(Cmd::Help(command))
    }

    fn parse_load(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<Cmd, String> {
        let path = match tokens.next() {
            Some(value) => Ok(String::from(value)),
            None => Err("missing argument".to_string()),
        }?;
        Ok(Cmd::Load(path))
    }

    fn parse_ls(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<Cmd, String> {
        let command = tokens.next().map(|s| s.to_string());
        self.ensure_eos(tokens)?;
        Ok(Cmd::Ls(command))
    }

    fn parse_reset(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<Cmd, String> {
        let mode = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Reset(mode.unwrap_or(0) == 1))
    }

    // -- Helpers

    fn ensure_eos(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<(), String> {
        match tokens.next() {
            Some(token) => Err(format!("Unexpected token {}", token)),
            None => Ok(()),
        }
    }

    #[allow(unused)]
    fn parse_num(&self, input: Option<&str>) -> Result<u16, String> {
        if let Some(value) = input {
            u16::from_str_radix(value, self.radix).map_err(|_| format!("Invalid number {}", value))
        } else {
            Err("missing argument".to_string())
        }
    }

    fn parse_num_maybe(&self, input: Option<&str>) -> Result<Option<u16>, String> {
        if let Some(value) = input {
            let result = u16::from_str_radix(value, self.radix)
                .map_err(|_| format!("Invalid number {}", value))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

struct Help;

impl Help {
    pub fn help(command: Option<String>) -> Result<String, String> {
        if let Some(command) = command {
            match command.trim().to_lowercase().as_str() {
                "load" => Help::help_cmd("load <image_path>", ""),
                "ls" => Help::help_cmd("ls [<dir>]", ""),
                "reset" => Help::help_cmd("reset [<type>]", ""),
                "exit" | "x" => Help::help_cmd("exit", "x"),
                _ => Err(format!("Invalid command {}", command)),
            }
        } else {
            Help::help_star()
        }
    }

    fn help_cmd(syntax: &str, short: &str) -> Result<String, String> {
        let abbr_line = if short.is_empty() {
            String::new()
        } else {
            format!("Shortname: {}\n", short)
        };
        let result = format!("Syntax: {}\n{}", syntax, abbr_line);
        Ok(result)
    }

    fn help_star() -> Result<String, String> {
        let mut buffer = String::new();
        buffer.push_str("load\n");
        buffer.push_str("ls\n");
        buffer.push_str("reset\n");
        buffer.push_str("exit (x)\n");
        buffer.push_str("help (?)\n");
        Ok(buffer)
    }
}
