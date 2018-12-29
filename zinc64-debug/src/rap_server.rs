// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::ffi::CStr;
use std::io;
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::u16;
use std::u8;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use super::{Command, CommandResult};

// SPEC: https://github.com/radare/radare2/blob/master/doc/rap
// SPEC: https://github.com/radare/radare2/blob/master/libr/io/p/io_rap.c
// SPEC: https://github.com/radare/radare2/blob/master/libr/socket/rap_server.c

const RAP_RMT_MAX: u32 = 4096;

const REG_PROFILE: &str = "
=PC	pc
=SP	sp
gpr	a	.8	0	0
gpr	x	.8	1	0
gpr	y	.8	2	0
gpr	p	.8	3	0
gpr	C	.1	.24	0
gpr	Z	.1	.25	0
gpr	I	.1	.26	0
gpr	D	.1	.27	0
gpr	V	.1	.30	0
gpr	N	.1	.31	0
gpr	sp	.8	4	0
gpr	pc	.16	5	0
";

#[derive(Clone, Copy)]
enum RapCmd {
    Registers,
    RegisterProfile,
}

enum RapOp {
    Open = 1,
    Read = 2,
    Write = 3,
    Seek = 4,
    Close = 5,
    // System = 6,
    Cmd = 7,
    Reply = 0x80,
}

impl RapOp {
    pub fn from(op: u8) -> Result<RapOp, String> {
        match op {
            1 => Ok(RapOp::Open),
            2 => Ok(RapOp::Read),
            3 => Ok(RapOp::Write),
            4 => Ok(RapOp::Seek),
            5 => Ok(RapOp::Close),
            7 => Ok(RapOp::Cmd),
            _ => Err(format!("Invalid op {}", op)),
        }
    }
}

pub struct RapServer {
    command_tx: Sender<Command>,
}

impl RapServer {
    pub fn new(command_tx: Sender<Command>) -> Self {
        Self { command_tx }
    }

    pub fn start(&self, addr: SocketAddr) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let mut conn = Connection::build(self.command_tx.clone(), &stream).unwrap();
                match conn.handle() {
                    Ok(_) => info!(target: "debugger", "Connection closed"),
                    Err(error) => {
                        error!(target: "debugger", "Connection failed, error - {}", error)
                    }
                }
            }
        }
        Ok(())
    }
}

struct Connection {
    command_parser: CommandParser,
    // I/O
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    command_tx: Sender<Command>,
    response_rx: Receiver<CommandResult>,
    response_tx: Sender<CommandResult>,
    // Runtime State
    offset: u16,
    running: bool,
}

impl Connection {
    pub fn build(command_tx: Sender<Command>, stream: &TcpStream) -> io::Result<Self> {
        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream.try_clone()?);
        let (response_tx, response_rx) = mpsc::channel::<CommandResult>();
        let conn = Self {
            command_parser: CommandParser::new(),
            reader,
            writer,
            command_tx,
            response_rx,
            response_tx,
            offset: 0,
            running: true,
        };
        Ok(conn)
    }

    pub fn handle(&mut self) -> io::Result<()> {
        while self.running {
            let opcode = self.reader.read_u8()?;
            let op = RapOp::from(opcode).map_err(|e| Error::new(ErrorKind::Other, e))?;
            match op {
                RapOp::Open => self.handle_open(),
                RapOp::Read => self.handle_read(),
                RapOp::Write => self.handle_write(),
                RapOp::Seek => self.handle_seek(),
                RapOp::Close => self.handle_close(),
                RapOp::Cmd => self.handle_cmd(),
                _ => Ok(()),
            }?;
        }
        Ok(())
    }

    fn handle_cmd(&mut self) -> io::Result<()> {
        let len = self.reader.read_u32::<BigEndian>()?;
        let mut data = vec![0; len as usize];
        self.reader.read_exact(&mut data)?;
        let c_str = CStr::from_bytes_with_nul(&data).unwrap();
        let input = str::from_utf8(c_str.to_bytes())
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        let input = input.to_string();
        info!(target: "rap", "Cmd '{}'", input);
        let result = match self.command_parser.parse(&input) {
            Ok(command) => self.execute_cmd(command),
            Err(error) => Ok(error),
        }?;
        info!(target: "rap", "Cmd result len {}", result.len());
        self.writer
            .write_u8(RapOp::Cmd as u8 | RapOp::Reply as u8)?;
        self.writer
            .write_u32::<BigEndian>((result.len() + 1) as u32)?;
        if !result.is_empty() {
            self.writer.write_all(&result.as_bytes())?;
            self.writer.write_u8(0)?;
        }
        self.writer.flush()
    }

    fn handle_close(&mut self) -> io::Result<()> {
        let _fd = self.reader.read_u32::<BigEndian>()?;
        match self.execute_emu(Command::Detach)? {
            CommandResult::Unit => Ok(()),
            result => Err(self.invalid_response(&result)),
        }?;
        self.running = false;
        self.writer
            .write_u8(RapOp::Close as u8 | RapOp::Reply as u8)?;
        self.writer.write_u32::<BigEndian>(0)?;
        self.writer.flush()
    }

    fn handle_open(&mut self) -> io::Result<()> {
        let _rw_flag = self.reader.read_u8()?;
        let len = self.reader.read_u8()?;
        let mut data = vec![0; len as usize];
        self.reader.read_exact(&mut data)?;
        let tx = self.response_tx.clone();
        match self.execute_emu(Command::Attach(tx))? {
            CommandResult::Unit => Ok(()),
            result => Err(self.invalid_response(&result)),
        }?;
        self.writer
            .write_u8(RapOp::Open as u8 | RapOp::Reply as u8)?;
        self.writer.write_u32::<BigEndian>(1000)?;
        self.writer.flush()
    }

    fn handle_read(&mut self) -> io::Result<()> {
        let mut len = self.reader.read_u32::<BigEndian>()?;
        if len > RAP_RMT_MAX {
            len = RAP_RMT_MAX;
        }
        info!(target: "rap", "Read 0x{:04x} {}", self.offset, len);
        let start = self.offset;
        let end = self.offset.wrapping_add(len as u16);
        let command = Command::MemRead(start, end);
        let data = match self.execute_emu(command)? {
            CommandResult::Buffer(data) => Ok(data),
            result => Err(self.invalid_response(&result)),
        }?;
        self.writer
            .write_u8(RapOp::Read as u8 | RapOp::Reply as u8)?;
        self.writer.write_u32::<BigEndian>(data.len() as u32)?;
        self.writer.write_all(&data)?;
        self.writer.flush()
    }

    fn handle_seek(&mut self) -> io::Result<()> {
        let whence = self.reader.read_u8()?;
        let offset = self.reader.read_u64::<BigEndian>()? as u16;
        info!(target: "rap", "Seek {} 0x{:04x}", whence, offset);
        self.offset = match whence {
            0 => offset,
            1 => self.offset.wrapping_add(offset),
            2 => (0xffff_u16).wrapping_add(offset),
            _ => self.offset,
        };
        self.writer
            .write_u8(RapOp::Seek as u8 | RapOp::Reply as u8)?;
        self.writer.write_u64::<BigEndian>(self.offset as u64)?;
        self.writer.flush()
    }

    fn handle_write(&mut self) -> io::Result<()> {
        let mut len = self.reader.read_u32::<BigEndian>()?;
        if len > RAP_RMT_MAX {
            len = RAP_RMT_MAX;
        }
        info!(target: "rap", "Write 0x{:04x} {}", self.offset, len);
        let mut data = vec![0; len as usize];
        self.reader.read_exact(&mut data)?;
        let command = Command::MemWrite(self.offset, data);
        match self.execute_emu(command)? {
            CommandResult::Unit => Ok(()),
            result => Err(self.invalid_response(&result)),
        }?;
        self.writer
            .write_u8(RapOp::Write as u8 | RapOp::Reply as u8)?;
        self.writer.write_u32::<BigEndian>(len)?;
        self.writer.flush()
    }

    fn invalid_response(&self, _result: &CommandResult) -> Error {
        Error::new(ErrorKind::Other, "Invalid debugger result")
    }

    // -- Commands

    fn cmd_registers(&mut self) -> io::Result<String> {
        match self.execute_emu(Command::RegRead)? {
            CommandResult::Registers(regs) => {
                let mut buffer = String::new();
                buffer.push_str(format!("a = 0x{:02x}\n", regs.a).as_str());
                buffer.push_str(format!("x = 0x{:02x}\n", regs.x).as_str());
                buffer.push_str(format!("y = 0x{:02x}\n", regs.y).as_str());
                buffer.push_str(format!("p = 0x{:02x}\n", regs.p).as_str());
                buffer.push_str(format!("sp = 0x{:02x}\n", regs.sp).as_str());
                buffer.push_str(format!("pc = 0x{:04x}\n", regs.pc).as_str());
                Ok(buffer)
            }
            other => Err(self.invalid_response(&other)),
        }
    }

    fn cmd_register_profile(&mut self) -> io::Result<String> {
        Ok(REG_PROFILE.to_string())
    }

    // -- Execution

    fn execute_cmd(&mut self, command: RapCmd) -> io::Result<String> {
        match command {
            RapCmd::Registers => self.cmd_registers(),
            RapCmd::RegisterProfile => self.cmd_register_profile(),
        }
    }

    fn execute_emu(&mut self, command: Command) -> io::Result<CommandResult> {
        self.command_tx.send(command).unwrap();
        match self.response_rx.recv() {
            Ok(CommandResult::Error(error)) => Err(Error::new(ErrorKind::Other, error)),
            Ok(result) => Ok(result),
            Err(error) => Err(Error::new(ErrorKind::Other, error)),
        }
    }
}

struct CommandParser {
    radix: u32,
}

impl CommandParser {
    pub fn new() -> Self {
        Self { radix: 16 }
    }

    pub fn parse(&self, input: &str) -> Result<RapCmd, String> {
        let mut tokens = input.split_whitespace();
        if let Some(command) = tokens.next() {
            match command.to_lowercase().as_str() {
                "dr" => self.parse_registers(&mut tokens),
                "drp" => self.parse_register_profile(&mut tokens),
                _ => Err(format!("Invalid command {}", input)),
            }
        } else {
            Err(format!("Invalid command {}", input))
        }
    }

    fn parse_registers(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<RapCmd, String> {
        self.ensure_eos(tokens)?;
        Ok(RapCmd::Registers)
    }

    fn parse_register_profile(
        &self,
        tokens: &mut dyn Iterator<Item = &str>,
    ) -> Result<RapCmd, String> {
        self.ensure_eos(tokens)?;
        Ok(RapCmd::RegisterProfile)
    }

    // Helpers

    fn ensure_eos(&self, tokens: &mut dyn Iterator<Item = &str>) -> Result<(), String> {
        match tokens.next() {
            Some(token) => Err(format!("Invalid token {}", token)),
            None => Ok(()),
        }
    }

    #[allow(dead_code)]
    fn parse_num(&self, input: Option<&str>) -> Result<u16, String> {
        if let Some(value) = input {
            u16::from_str_radix(value, self.radix).map_err(|_| format!("invalid number {}", value))
        } else {
            Err("missing argument".to_string())
        }
    }

    #[allow(dead_code)]
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
