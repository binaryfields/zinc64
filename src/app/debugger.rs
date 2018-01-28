/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::fmt;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::u8;
use std::u16;

use super::command::{Command, CommandResult, Response, SystemState};

pub struct Debugger {
    command_tx: Sender<Command>,
}

impl Debugger {
    pub fn new(command_tx: Sender<Command>) -> Self {
        Self {
            command_tx,
        }
    }

    pub fn start(&self) -> io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:3000")?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut conn = Connection::new(self.command_tx.clone(), stream);
                    match conn.handle() {
                        Ok(_) => {
                            info!(target: "debugger", "Connection closed")
                        }
                        Err(error) => {
                            error!(target: "debugger", "Connection failed, error - {}", error)
                        }
                    }
                }
                Err(_) => {}
            }
        }
        Ok(())
    }
}

struct Connection {
    // Dependencies
    command_parser: CommandParser,
    // I/O
    command_tx: Sender<Command>,
    response_rx: Receiver<Response>,
    response_tx: Sender<Response>,
    stream: TcpStream,
    // Runtime State
    running: bool,
    sys_state: Option<SystemState>,
}

impl Connection {
    pub fn new(command_tx: Sender<Command>, stream: TcpStream) -> Self {
        let (response_tx, response_rx) = mpsc::channel::<Response>();
        Self {
            command_parser: CommandParser::new(),
            command_tx,
            response_rx,
            response_tx,
            stream,
            running: true,
            sys_state: None,
        }
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut writer = BufWriter::new(self.stream.try_clone()?);
        let tx = self.response_tx.clone();
        self.handle_command(Command::Attach(tx), &mut writer)?;
        while self.running {
            self.write_prompt(&mut writer)?;
            let mut input = String::new();
            reader.read_line(&mut input)?;
            self.handle_request(&input, &mut writer)?;
        }
        self.handle_command(Command::Detach, &mut writer)
    }

    fn handle_command(
        &mut self,
        command: Command,
        writer: &mut BufWriter<TcpStream>,
    ) -> io::Result<()> {
        self.command_tx.send(command).unwrap();
        match self.response_rx.recv() {
            Ok(Response(result, sys_state)) => {
                self.sys_state = Some(sys_state);
                write!(writer, "{}", result)
            }
            Err(error) => {
                write!(writer, "Error: {}", error)
            }
        }
    }

    fn handle_request(
        &mut self,
        input: &String,
        writer: &mut BufWriter<TcpStream>,
    ) -> io::Result<()> {
        match self.command_parser.parse(input) {
            Ok(command) => {
                match command {
                    Command::Exit => {
                        self.running = false;
                        Ok(())
                    }
                    Command::Radix(radix) => {
                        if let Some(radix) = radix {
                            self.command_parser.set_radix(radix as u32);
                        }
                        writeln!(writer, "Set radix to {}", self.command_parser.get_radix())?;
                        Ok(())
                    }
                    _ => self.handle_command(command, writer),
                }
            }
            Err(error) => writeln!(writer, "{}", error),
        }
    }

    fn write_prompt(&self, writer: &mut BufWriter<TcpStream>) -> io::Result<()> {
        if let Some(ref sys_state) = self.sys_state {
            write!(writer, "${:04x}> ", sys_state.pc)?;
        } else {
            write!(writer, "> ")?;
        }
        writer.flush()
    }
}

struct CommandParser {
    radix: u32,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            radix: 16,
        }
    }

    pub fn get_radix(&self) -> u32 {
        self.radix
    }

    pub fn set_radix(&mut self, radix: u32) {
        self.radix = radix;
    }

    pub fn parse(&self, input: &String) -> Result<Command, String> {
        let mut tokens = input.split_whitespace();
        if let Some(command) = tokens.next() {
            match command {
                // Machine
                "g" | "goto" => self.parse_goto(&mut tokens),
                "n" | "next" => self.parse_next(&mut tokens),
                "r" | "registers" => self.parse_registers(&mut tokens),
                "reset" => self.parse_reset(&mut tokens),
                "ret" | "return" => self.parse_return(&mut tokens),
                "sc" | "screen" => self.parse_screen(&mut tokens),
                "z" | "step" => self.parse_step(&mut tokens),
                "sw" | "stopwatch" => self.parse_stopwatch(&mut tokens),
                // Memory
                "c" | "compare" => self.parse_compare(&mut tokens),
                "d" | "disass" => self.parse_disassemble(&mut tokens),
                "f" | "fill" => self.parse_fill(&mut tokens),
                "h" | "hunt" => self.parse_hunt(&mut tokens),
                "m" | "mem" => self.parse_memory(&mut tokens),
                "mc" | "memchar" => self.parse_mem_char(&mut tokens),
                "i" | "petscii" => self.parse_mem_petscii(&mut tokens),
                "t" | "move" => self.parse_move(&mut tokens),
                // Monitor
                "x" | "exit" => self.parse_exit(&mut tokens),
                "quit" => self.parse_quit(&mut tokens),
                "radix" => self.parse_radix(&mut tokens),
                // Breakpoints
                "bk" | "break" => self.parse_bp_bk(&mut tokens),
                "en" | "enable" => self.parse_bp_enable(&mut tokens),
                "del" | "delete" => self.parse_bp_delete(&mut tokens),
                "dis" | "disable" => self.parse_bp_disable(&mut tokens),
                "ignore" => self.parse_bp_ignore(&mut tokens),
                "un" | "until" => self.parse_bp_until(&mut tokens),
                _ => Err(format!("Invalid command {}", input))
            }
        } else {
            Err(format!("Invalid command {}", input))
        }
    }

    // Machine

    fn parse_goto(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let address = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Goto(address))
    }

    fn parse_next(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Next(count.unwrap_or(1)))
    }

    fn parse_registers(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        self.ensure_eos(tokens)?;
        Ok(Command::Registers)
    }

    fn parse_reset(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let mode = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Reset(mode.unwrap_or(0) == 1))
    }

    fn parse_return(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        self.ensure_eos(tokens)?;
        Ok(Command::Return)
    }

    fn parse_screen(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        self.ensure_eos(tokens)?;
        Ok(Command::Screen)
    }

    fn parse_step(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Step(count.unwrap_or(1)))
    }

    fn parse_stopwatch(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let reset = if let Some(token) = tokens.next() {
            token == "reset"
        } else {
            false
        };
        Ok(Command::Stopwatch(reset))
    }

    // Memory

    fn parse_compare(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let target = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Compare(start, end, target))
    }

    fn parse_disassemble(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num_maybe(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Disassemble(start, end))
    }

    fn parse_fill(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let mut data: Vec<u8> = Vec::new();
        while let Some(token) = tokens.next() {
            if token.contains(',') {
                for value in token.split(',') {
                    data.push(self.parse_byte(value)?);
                }
            } else {
                data.push(self.parse_byte(token)?)
            }
        }
        if data.len() >= 1 {
            Ok(Command::Fill(start, end, data))
        } else {
            Err(format!("Missing data"))
        }
    }

    fn parse_hunt(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let mut data: Vec<u8> = Vec::new();
        while let Some(token) = tokens.next() {
            if token.contains(',') {
                for value in token.split(',') {
                    data.push(self.parse_byte(value)?);
                }
            } else {
                data.push(self.parse_byte(token)?)
            }
        }
        if data.len() >= 1 {
            Ok(Command::Hunt(start, end, data))
        } else {
            Err(format!("Missing data"))
        }
    }

    fn parse_mem_char(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let address = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::MemChar(address))
    }


    fn parse_mem_petscii(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::MemPetscii(start, end))
    }

    fn parse_memory(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Read(start, end))
    }

    fn parse_move(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let target = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Move(start, end, target))
    }

    // Monitor

    fn parse_exit(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        self.ensure_eos(tokens)?;
        Ok(Command::Exit)
    }

    fn parse_quit(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        self.ensure_eos(tokens)?;
        Ok(Command::Quit)
    }

    fn parse_radix(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let radix = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::Radix(radix))
    }

    // Breakpoints

    fn parse_bp_bk(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        match address {
            Some(address) => Ok(Command::BpSet(address)),
            None => Ok(Command::BpList),
        }
    }

    fn parse_bp_enable(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::BpEnable(index))
    }

    fn parse_bp_delete(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::BpDelete(index))
    }

    fn parse_bp_disable(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::BpDisable(index))
    }

    fn parse_bp_ignore(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let index = self.parse_num(tokens.next())?;
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Command::BpIgnore(index, count.unwrap_or(1)))
    }

    fn parse_bp_until(&self, tokens: &mut Iterator<Item=&str>) -> Result<Command, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        match address {
            Some(address) => Ok(Command::BpUntil(address)),
            None => Ok(Command::BpList),
        }
    }

    // Helpers

    fn ensure_eos(&self, tokens: &mut Iterator<Item=&str>) -> Result<(), String> {
        match tokens.next() {
            Some(token) => Err(format!("Invalid token {}", token)),
            None => Ok(()),
        }
    }

    fn parse_byte(&self, value: &str) -> Result<u8, String> {
        u8::from_str_radix(value, self.radix)
            .map_err(|_| format!("invalid number {}", value))
    }

    fn parse_num(&self, input: Option<&str>) -> Result<u16, String> {
        if let Some(value) = input {
            u16::from_str_radix(value, self.radix)
                .map_err(|_| format!("invalid number {}", value))
        } else {
            Err("missing argument".to_string())
        }
    }

    fn parse_num_maybe(&self, input: Option<&str>) -> Result<Option<u16>, String> {
        if let Some(value) = input {
            let result = u16::from_str_radix(value, self.radix)
                .map_err(|_| format!("invalid number {}", value))?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

impl fmt::Display for CommandResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CommandResult::Empty => {
                Ok(())
            }
            CommandResult::Error(ref error) => {
                writeln!(f, "Error: {}", error)
            }
            CommandResult::ExecutionState(ref cpu_state, ref instruction) => {
                let mut instr_bytes = String::new();
                for byte in instruction.instr_bytes.iter() {
                    instr_bytes.push_str(format!("{:02x} ", byte).as_str());
                }
                writeln!(
                    f,
                    "${:04x}: {:12} {:16} A:{:02x} X:{:02x} Y:{:02x} SP:{:02x} {}{}{}{}{}{}{}",
                    cpu_state.pc,
                    instr_bytes,
                    instruction.instr_text,
                    cpu_state.a,
                    cpu_state.x,
                    cpu_state.y,
                    cpu_state.sp,
                    if (cpu_state.p & CpuFlag::Negative as u8) != 0 { "N" } else { "n" },
                    if (cpu_state.p & CpuFlag::Overflow as u8) != 0 { "V" } else { "v" },
                    if (cpu_state.p & CpuFlag::Decimal as u8) != 0 { "B" } else { "b" },
                    if (cpu_state.p & CpuFlag::Decimal as u8) != 0 { "D" } else { "d" },
                    if (cpu_state.p & CpuFlag::IntDisable as u8) != 0 { "I" } else { "i" },
                    if (cpu_state.p & CpuFlag::Zero as u8) != 0 { "Z" } else { "z" },
                    if (cpu_state.p & CpuFlag::Carry as u8) != 0 { "C" } else { "c" }
                )
            }
            CommandResult::Memory(address, ref buffer) => {
                let mut address = address;
                let mut counter = 0;
                for value in buffer {
                    if counter % 12 == 0 {
                        write!(f, "${:04x} ", address)?;
                        address = address.wrapping_add(12);
                    }
                    if counter % 4 == 0 {
                        write!(f, " ")?;
                    }
                    write!(f, " {:02x}", value)?;
                    counter += 1;
                    if counter % 12 == 0 {
                        writeln!(f)?;
                    }
                }
                if buffer.len() % 12 != 0 {
                    writeln!(f)?;
                }
                Ok(())
            }
            CommandResult::Registers(ref cpu_state) => {
                writeln!(
                    f,
                    "PC   A  X  Y  SP 00 01 NV-BDIZC"
                )?;
                writeln!(
                    f,
                    "{:04x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {}{}1{}{}{}{}{}",
                    cpu_state.pc,
                    cpu_state.a,
                    cpu_state.x,
                    cpu_state.y,
                    cpu_state.sp,
                    cpu_state.port_00,
                    cpu_state.port_01,
                    if (cpu_state.p & CpuFlag::Negative as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::Overflow as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::Break as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::Decimal as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::IntDisable as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::Zero as u8) != 0 { "1" } else { "0" },
                    if (cpu_state.p & CpuFlag::Carry as u8) != 0 { "1" } else { "0" }
                )
            }
            CommandResult::Text(ref lines) => {
                for line in lines {
                    writeln!(f, "{}", line)?;
                }
                Ok(())
            }
            _ => panic!("Unsupported command result type"),
        }
    }
}

enum CpuFlag {
    Carry = 1 << 0,
    Zero = 1 << 1,
    IntDisable = 1 << 2,
    Decimal = 1 << 3,
    Break = 1 << 4,
    Overflow = 1 << 6,
    Negative = 1 << 7,
}
