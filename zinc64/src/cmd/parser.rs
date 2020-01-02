// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::result::Result;

use super::Cmd;

pub struct Parser {
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
                //"exit" | "x" => self.parse_exit(&mut tokens),
                "help" | "?" => self.parse_help(&mut tokens),
                _ => Err(format!("Invalid command {}", input)),
            }
        } else {
            Err(format!("Invalid command {}", input))
        }
    }

    /*
    fn parse_exit(&self, tokens: &mut dyn Iterator<Item=&str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Exit)
    }
    */

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
