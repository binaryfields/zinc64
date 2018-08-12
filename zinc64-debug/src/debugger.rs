// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::io;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Error, ErrorKind, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::u16;
use std::u8;

use bit_field::BitField;
use byteorder::{BigEndian, ReadBytesExt};
use zinc64::cpu::Instruction;

use super::charset;
use super::{Command, CommandResult, RegData, RegOp};
use super::disassembler::Disassembler;

// SPEC: Vice -> Alt-H -> help -> [Enter]

const OPCODE_JSR: u8 = 0x20;
const OPCODE_RTI: u8 = 0x40;
const OPCODE_RTS: u8 = 0x60;

// TODO debugger: print triggered breakpoint

pub enum Cmd {
    // Breakpoint
    BpCondition(u16, String),
    BpDisable(Option<u16>),
    BpDelete(Option<u16>),
    BpEnable(Option<u16>),
    BpIgnore(u16, u16),
    BpList,
    BpSet(u16),
    BpUntil(u16),
    // Debugger
    Goto(Option<u16>),
    Next(u16),
    RegRead,
    RegWrite(Vec<RegOp>),
    Return,
    Step(u16),
    // Memory
    Compare(u16, u16, u16),
    Disassemble(Option<u16>, Option<u16>),
    Fill(u16, u16, Vec<u8>),
    Hunt(u16, u16, Vec<u8>),
    Memory(Option<u16>, Option<u16>),
    MemChar(Option<u16>),
    Move(u16, u16, u16),
    Petscii(u16, Option<u16>),
    // System
    Reset(bool),
    Screen,
    Stopwatch(bool),
    // Monitor
    Exit,
    Help(Option<String>),
    Quit,
    Radix(Option<u16>),
}

pub struct Debugger {
    command_tx: mpsc::Sender<Command>,
}

impl Debugger {
    pub fn new(command_tx: mpsc::Sender<Command>) -> Self {
        Self { command_tx }
    }

    pub fn start(&self, addr: SocketAddr) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut conn = Connection::new(self.command_tx.clone(), stream).unwrap();
                    match conn.handle() {
                        Ok(_) => info!(target: "debugger", "Connection closed"),
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
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    command_tx: mpsc::Sender<Command>,
    response_rx: mpsc::Receiver<CommandResult>,
    response_tx: mpsc::Sender<CommandResult>,
    // Runtime State
    regs: Option<RegData>,
    running: bool,
}

impl Connection {
    pub fn new(command_tx: mpsc::Sender<Command>, stream: TcpStream) -> io::Result<Self> {
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
            regs: None,
            running: true,
        };
        Ok(conn)
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let tx = self.response_tx.clone();
        self.execute_unit_cmd(Command::Attach(tx))?;
        while self.running {
            self.regs = Some(self.read_regs()?);
            self.write_prompt()?;
            let mut input = String::new();
            self.reader.read_line(&mut input)?;
            self.handle_request(&input)?;
        }
        self.command_tx.send(Command::Detach).unwrap();
        self.writer.flush()?;
        Ok(())
    }

    fn handle_command(&mut self, command: Cmd) -> io::Result<()> {
        let result = match command {
            // Breakpoint
            Cmd::BpCondition(index, condition) => self.cmd_bp_condition(index, condition),
            Cmd::BpDisable(index) => self.cmd_bp_disable(index),
            Cmd::BpDelete(index) => self.cmd_bp_delete(index),
            Cmd::BpEnable(index) => self.cmd_bp_enable(index),
            Cmd::BpIgnore(index, count) => self.cmd_bp_ignore(index, count),
            Cmd::BpList => self.cmd_bp_list(),
            Cmd::BpSet(address) => self.cmd_bp_set(address),
            Cmd::BpUntil(address) => self.cmd_bp_until(address),
            // Debugger
            Cmd::Goto(address) => self.cmd_goto(address),
            Cmd::Next(count) => self.cmd_next(count),
            Cmd::RegRead => self.cmd_reg_read(),
            Cmd::RegWrite(ops) => self.cmd_reg_write(ops),
            Cmd::Return => self.cmd_return(),
            Cmd::Step(count) => self.cmd_step(count),
            // Memory
            Cmd::Compare(start, end, target) => self.cmd_compare(start, end, target),
            Cmd::Disassemble(start, end) => self.cmd_disassemble(start, end),
            Cmd::Fill(start, end, data) => self.cmd_fill(start, end, data),
            Cmd::Hunt(start, end, data) => self.cmd_hunt(start, end, data),
            Cmd::Memory(start, end) => self.cmd_memory(start, end),
            Cmd::MemChar(address) => self.cmd_memchar(address),
            Cmd::Move(start, end, target) => self.cmd_move(start, end, target),
            Cmd::Petscii(start, end) => self.cmd_petscii(start, end),
            // System
            Cmd::Reset(hard) => self.cmd_reset(hard),
            Cmd::Screen => self.cmd_screen(),
            Cmd::Stopwatch(reset) => self.cmd_stopwatch(reset),
            // Monitor
            Cmd::Exit => self.cmd_exit(),
            Cmd::Quit => self.cmd_quit(),
            Cmd::Radix(radix) => self.cmd_radix(radix),
            Cmd::Help(command) => CommandHelp::help(command),
        };
        let output = result.unwrap_or_else(|e| format!("Error: {}\n", e));
        self.writer.write_all(output.as_bytes())
    }

    fn handle_request(&mut self, input: &String) -> io::Result<()> {
        match self.command_parser.parse(input) {
            Ok(command) => self.handle_command(command),
            Err(error) => self.writer.write_all(format!("{}\n", error).as_bytes()),
        }
    }

    fn read_mem(&mut self, start: u16, end: u16) -> io::Result<Vec<u8>> {
        match self.execute_emu(Command::MemRead(start, end))? {
            CommandResult::Buffer(data) => Ok(data),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn read_regs(&mut self) -> io::Result<RegData> {
        match self.execute_emu(Command::RegRead)? {
            CommandResult::Registers(regs) => Ok(regs),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn write_prompt(&mut self) -> io::Result<()> {
        let pc = self.regs.as_ref().map_or(0, |r| r.pc);
        write!(self.writer, "${:04x}> ", pc)?;
        self.writer.flush()
    }

    // -- Commands

    // -- Breakpoint

    fn cmd_bp_condition(&mut self, index: u16, condition: String) -> io::Result<String> {
        let command = Command::BpCondition(index, condition, self.command_parser.get_radix());
        self.execute_text_cmd(command)
    }

    fn cmd_bp_disable(&mut self, index: Option<u16>) -> io::Result<String> {
        if let Some(index) = index {
            self.execute_unit_cmd(Command::BpDisable(index))
        } else {
            self.execute_unit_cmd(Command::BpDisableAll)?;
            Ok(format!("Set all breakpoints to state: disabled\n"))
        }
    }

    fn cmd_bp_delete(&mut self, index: Option<u16>) -> io::Result<String> {
        if let Some(index) = index {
            self.execute_unit_cmd(Command::BpRemove(index))
        } else {
            self.execute_unit_cmd(Command::BpClear)?;
            Ok(format!("Deleted all breakpoints\n"))
        }
    }

    fn cmd_bp_enable(&mut self, index: Option<u16>) -> io::Result<String> {
        if let Some(index) = index {
            self.execute_unit_cmd(Command::BpEnable(index))
        } else {
            self.execute_unit_cmd(Command::BpEnableAll)?;
            Ok(format!("Set all breakpoints to state: enabled\n"))
        }
    }

    fn cmd_bp_ignore(&mut self, index: u16, count: u16) -> io::Result<String> {
        self.execute_unit_cmd(Command::BpIgnore(index, count))?;
        Ok(format!(
            "Ignoring the next {} hits of breakpoint {}\n",
            count, index
        ))
    }

    fn cmd_bp_list(&mut self) -> io::Result<String> {
        self.execute_text_cmd(Command::BpList)
    }

    fn cmd_bp_set(&mut self, address: u16) -> io::Result<String> {
        self.execute_text_cmd(Command::BpSet(address, false))
    }

    fn cmd_bp_until(&mut self, address: u16) -> io::Result<String> {
        self.execute_text_cmd(Command::BpSet(address, true))?;
        self.execute_unit_cmd(Command::Continue)?;
        let regs = self.read_regs()?;
        let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(10))?;
        let dis = Disassembler::new(mem.clone(), regs.pc);
        let (instr, instr_len) = dis.disassemble(regs.pc);
        Ok(self.format_instr(&regs, &instr, &mem[0..instr_len]))
    }

    // -- Debugger

    fn cmd_goto(&mut self, address: Option<u16>) -> io::Result<String> {
        if let Some(address) = address {
            self.execute_unit_cmd(Command::RegWrite(vec![RegOp::SetPC(address)]))?;
        }
        self.execute_unit_cmd(Command::Continue)?;
        let regs = self.read_regs()?;
        let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(10))?;
        let dis = Disassembler::new(mem.clone(), regs.pc);
        let (instr, instr_len) = dis.disassemble(regs.pc);
        Ok(self.format_instr(&regs, &instr, &mem[0..instr_len]))
    }

    fn cmd_next(&mut self, count: u16) -> io::Result<String> {
        let mut bp_hit = 0;
        for _i in 0..count {
            let regs = self.read_regs()?;
            let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(1))?;
            let opcode = mem[0];
            if opcode == OPCODE_JSR {
                let target = regs.pc.wrapping_add(3);
                loop {
                    let regs = self.read_regs()?;
                    if regs.pc == target {
                        break;
                    }
                    bp_hit = self.execute_num_cmd(Command::Step)?;
                    if bp_hit > 0 {
                        break;
                    }
                }
            } else {
                bp_hit = self.execute_num_cmd(Command::Step)?;
                if bp_hit > 0 {
                    break;
                }
            }
        }
        let mut buffer = String::new();
        if bp_hit > 0 {
            buffer.push_str(format!("Stopped on breakpoint\n").as_str());
        }
        let regs = self.read_regs()?;
        let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(10))?;
        let dis = Disassembler::new(mem.clone(), regs.pc);
        let (instr, instr_len) = dis.disassemble(regs.pc);
        buffer.push_str(
            self.format_instr(&regs, &instr, &mem[0..instr_len])
                .as_str(),
        );
        Ok(buffer)
    }

    fn cmd_reg_read(&mut self) -> io::Result<String> {
        let regs = self.read_regs()?;
        Ok(self.format_regs(regs))
    }

    fn cmd_reg_write(&mut self, ops: Vec<RegOp>) -> io::Result<String> {
        self.execute_unit_cmd(Command::RegWrite(ops))?;
        let regs = self.read_regs()?;
        Ok(self.format_regs(regs))
    }

    fn cmd_return(&mut self) -> io::Result<String> {
        let mut bp_hit = 0;
        loop {
            let regs = self.read_regs()?;
            let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(1))?;
            let opcode = mem[0];
            if opcode == OPCODE_RTS || opcode == OPCODE_RTI {
                break;
            }
            bp_hit = self.execute_num_cmd(Command::Step)?;
            if bp_hit > 0 {
                break;
            }
        }
        let mut buffer = String::new();
        if bp_hit > 0 {
            buffer.push_str(format!("Stopped on breakpoint\n").as_str());
        }
        let regs = self.read_regs()?;
        let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(10))?;
        let dis = Disassembler::new(mem.clone(), regs.pc);
        let (instr, instr_len) = dis.disassemble(regs.pc);
        buffer.push_str(
            self.format_instr(&regs, &instr, &mem[0..instr_len])
                .as_str(),
        );
        Ok(buffer)
    }

    fn cmd_step(&mut self, count: u16) -> io::Result<String> {
        let mut bp_hit = 0;
        for _i in 0..count {
            bp_hit = self.execute_num_cmd(Command::Step)?;
            if bp_hit > 0 {
                break;
            }
        }
        let mut buffer = String::new();
        if bp_hit > 0 {
            buffer.push_str(format!("Stopped on breakpoint\n").as_str());
        }
        let regs = self.read_regs()?;
        let mem = self.read_mem(regs.pc, regs.pc.wrapping_add(10))?;
        let dis = Disassembler::new(mem.clone(), regs.pc);
        let (instr, instr_len) = dis.disassemble(regs.pc);
        buffer.push_str(
            self.format_instr(&regs, &instr, &mem[0..instr_len])
                .as_str(),
        );
        Ok(buffer)
    }

    // -- Memory

    fn cmd_compare(&mut self, start: u16, end: u16, target: u16) -> io::Result<String> {
        let source_data = self.read_mem(start, end)?;
        let target_end = target.wrapping_add(target + source_data.len() as u16);
        let target_data = self.read_mem(target, target_end)?;
        let mut buffer = String::new();
        for i in 0..source_data.len() {
            let source = source_data[i];
            let dest = target_data[i];
            if source != dest {
                let s = format!(
                    "${:04x} ${:04x}: {:02x} {:02x}\n",
                    start.wrapping_add(i as u16),
                    target.wrapping_add(i as u16),
                    source,
                    dest
                );
                buffer.push_str(s.as_str());
            }
        }
        Ok(buffer)
    }

    fn cmd_disassemble(&mut self, start: Option<u16>, end: Option<u16>) -> io::Result<String> {
        let start = start.unwrap_or(self.regs.as_ref().map(|r| r.pc).unwrap_or(0));
        let end = end.unwrap_or(start + 96);
        let data = self.read_mem(start, end + 10)?;
        let dis = Disassembler::new(data, start);
        let mut buffer = String::new();
        let mut address = start;
        while address < end {
            let (instr, instr_len) = dis.disassemble(address);
            let mut instr_bytes = String::new();
            for i in 0..instr_len as u16 {
                let byte = dis.read_byte(address + i);
                instr_bytes.push_str(format!("{:02x} ", byte).as_str());
            }
            let instr_text = format!("{}", instr);
            buffer.push_str(
                format!("${:04x}  {:12} {}\n", address, instr_bytes, instr_text).as_str(),
            );
            address = address.wrapping_add(instr_len as u16);
        }
        Ok(buffer)
    }

    fn cmd_fill(&mut self, start: u16, end: u16, data: Vec<u8>) -> io::Result<String> {
        let mut address = start;
        while address < end {
            self.execute_unit_cmd(Command::MemWrite(address, data.clone()))?;
            address = address.wrapping_add(data.len() as u16);
        }
        Ok(String::new())
    }

    fn cmd_hunt(&mut self, start: u16, end: u16, search: Vec<u8>) -> io::Result<String> {
        let data = self.read_mem(start, end)?;
        let mut buffer = String::new();
        for i in 0..data.len() {
            let mut found = true;
            let value = data[i];
            for j in 0..search.len() {
                if value != search[j] {
                    found = false;
                    break;
                }
            }
            if found {
                buffer.push_str(format!("{:04x}\n", start.wrapping_add(i as u16)).as_str());
            }
        }
        Ok(buffer)
    }

    fn cmd_memory(&mut self, start: Option<u16>, end: Option<u16>) -> io::Result<String> {
        let start = start.unwrap_or(self.regs.as_ref().map(|r| r.pc).unwrap_or(0));
        let data = self.read_mem(start, end.unwrap_or(start + 96))?;
        let mut buffer = String::new();
        let mut address = start;
        let mut counter = 0;
        for value in data {
            if counter % 12 == 0 {
                buffer.push_str(format!("${:04x} ", address).as_str());
                address = address.wrapping_add(12);
            }
            if counter % 4 == 0 {
                buffer.push(' ');
            }
            buffer.push_str(format!(" {:02x}", value).as_str());
            counter += 1;
            if counter % 12 == 0 {
                buffer.push('\n');
            }
        }
        if counter % 12 != 0 {
            buffer.push('\n');
        }
        Ok(buffer)
    }

    fn cmd_memchar(&mut self, address: Option<u16>) -> io::Result<String> {
        let address = address.unwrap_or(self.regs.as_ref().map(|r| r.pc).unwrap_or(0));
        let data = self.read_mem(address, address.wrapping_add(8))?;
        let mut buffer = String::new();
        for value in data {
            let mut s = String::new();
            s.push_str(format!("${:04x} ", address).as_str());
            for i in 0..8 {
                let bit = if value.get_bit(7 - i) { "." } else { "*" };
                s.push_str(bit);
            }
            buffer.push_str(format!("{}\n", s).as_str());
        }
        Ok(buffer)
    }

    fn cmd_move(&mut self, start: u16, end: u16, target: u16) -> io::Result<String> {
        let data = self.read_mem(start, end)?;
        self.execute_unit_cmd(Command::MemWrite(target, data))
    }

    fn cmd_petscii(&mut self, start: u16, end: Option<u16>) -> io::Result<String> {
        let data = self.read_mem(start, end.unwrap_or(start.wrapping_add(400)))?;
        let mut buffer = String::new();
        let mut counter = 0;
        for value in data {
            let ascii = match charset::pet_to_ascii(value) {
                0 => 46,
                v => v,
            };
            buffer.push(char::from(ascii));
            counter += 1;
            if counter % 40 == 0 {
                buffer.push('\n');
            }
        }
        if counter % 40 != 0 {
            buffer.push('\n');
        }
        Ok(buffer)
    }

    // -- System

    fn cmd_quit(&mut self) -> io::Result<String> {
        self.running = false;
        self.execute_unit_cmd(Command::SysQuit)
    }

    fn cmd_reset(&mut self, hard: bool) -> io::Result<String> {
        self.execute_unit_cmd(Command::SysReset(hard))
    }

    fn cmd_screen(&mut self) -> io::Result<String> {
        let vm_base = self.execute_num_cmd(Command::SysScreen)?;
        let data = self.read_mem(vm_base, vm_base.wrapping_add(1000))?;
        let mut buffer = String::new();
        let mut counter = 0;
        buffer.push_str(format!("Displaying 40x25 screen at ${:04x}\n", vm_base).as_str());
        for value in data {
            let ascii = match charset::screen_code_to_ascii(value) {
                0 => 46,
                v => v,
            };
            buffer.push(char::from(ascii));
            counter += 1;
            if counter % 40 == 0 {
                buffer.push('\n');
            }
        }
        if counter % 40 != 0 {
            buffer.push('\n');
        }
        Ok(buffer)
    }

    fn cmd_stopwatch(&mut self, reset: bool) -> io::Result<String> {
        let result = self.execute_buffer_cmd(Command::SysStopwatch(reset))?;
        let mut rdr = Cursor::new(result);
        let clock = rdr.read_u64::<BigEndian>()?;
        Ok(format!("Clock: {}", clock))
    }

    // -- Monitor

    fn cmd_exit(&mut self) -> io::Result<String> {
        self.running = false;
        Ok(String::new())
    }

    fn cmd_radix(&mut self, radix: Option<u16>) -> io::Result<String> {
        if let Some(radix) = radix {
            self.command_parser.set_radix(radix as u32);
        }
        let result = format!("Set radix to {}\n", self.command_parser.get_radix());
        Ok(result)
    }

    // -- Helpers

    fn execute_emu(&mut self, command: Command) -> io::Result<CommandResult> {
        self.command_tx.send(command).unwrap();
        self.response_rx
            .recv()
            .map_err(|error| Error::new(ErrorKind::Other, error))
    }

    fn execute_buffer_cmd(&mut self, command: Command) -> io::Result<Vec<u8>> {
        match self.execute_emu(command)? {
            CommandResult::Buffer(buffer) => Ok(buffer),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn execute_num_cmd(&mut self, command: Command) -> io::Result<u16> {
        match self.execute_emu(command)? {
            CommandResult::Number(num) => Ok(num),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn execute_text_cmd(&mut self, command: Command) -> io::Result<String> {
        match self.execute_emu(command)? {
            CommandResult::Text(text) => Ok(text),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn execute_unit_cmd(&mut self, command: Command) -> io::Result<String> {
        match self.execute_emu(command)? {
            CommandResult::Unit => Ok(String::new()),
            CommandResult::Error(error) => Err(Error::new(ErrorKind::Other, error)),
            _ => Err(Error::new(ErrorKind::Other, "Invalid debugger result")),
        }
    }

    fn format_instr(&self, regs: &RegData, instr: &Instruction, instr_bytes: &[u8]) -> String {
        let mut buffer = String::new();
        let mut instr_bytes2 = String::new();
        for byte in instr_bytes {
            instr_bytes2.push_str(format!("{:02x} ", byte).as_str());
        }
        buffer.push_str(
            format!(
                "${:04x}: {:12} {:16} A:{:02x} X:{:02x} Y:{:02x} SP:{:02x} {}{}{}{}{}{}{}\n",
                regs.pc,
                instr_bytes2,
                format!("{}", instr),
                regs.a,
                regs.x,
                regs.y,
                regs.sp,
                if (regs.p & CpuFlag::Negative as u8) != 0 {
                    "N"
                } else {
                    "n"
                },
                if (regs.p & CpuFlag::Overflow as u8) != 0 {
                    "V"
                } else {
                    "v"
                },
                if (regs.p & CpuFlag::Decimal as u8) != 0 {
                    "B"
                } else {
                    "b"
                },
                if (regs.p & CpuFlag::Decimal as u8) != 0 {
                    "D"
                } else {
                    "d"
                },
                if (regs.p & CpuFlag::IntDisable as u8) != 0 {
                    "I"
                } else {
                    "i"
                },
                if (regs.p & CpuFlag::Zero as u8) != 0 {
                    "Z"
                } else {
                    "z"
                },
                if (regs.p & CpuFlag::Carry as u8) != 0 {
                    "C"
                } else {
                    "c"
                }
            ).as_str(),
        );
        buffer
    }

    fn format_regs(&self, regs: RegData) -> String {
        let mut buffer = String::new();
        buffer.push_str("PC   A  X  Y  SP 00 01 NV-BDIZC\n");
        buffer.push_str(
            format!(
                "{:04x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {}{}1{}{}{}{}{}\n",
                regs.pc,
                regs.a,
                regs.x,
                regs.y,
                regs.sp,
                regs.port_00,
                regs.port_01,
                if (regs.p & CpuFlag::Negative as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::Overflow as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::Break as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::Decimal as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::IntDisable as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::Zero as u8) != 0 {
                    "1"
                } else {
                    "0"
                },
                if (regs.p & CpuFlag::Carry as u8) != 0 {
                    "1"
                } else {
                    "0"
                }
            ).as_str(),
        );
        buffer
    }
}

struct CommandParser {
    radix: u32,
}

impl CommandParser {
    pub fn new() -> Self {
        Self { radix: 16 }
    }

    pub fn get_radix(&self) -> u32 {
        self.radix
    }

    pub fn set_radix(&mut self, radix: u32) {
        self.radix = radix;
    }

    pub fn parse(&self, input: &String) -> Result<Cmd, String> {
        let mut tokens = input.split_whitespace();
        if let Some(command) = tokens.next() {
            match command.to_lowercase().as_str() {
                // Breakpoint
                "break" | "bk" => self.parse_break(&mut tokens),
                "condition" | "cond" => self.parse_condition(&mut tokens),
                "enable" | "en" => self.parse_enable(&mut tokens),
                "delete" | "del" => self.parse_delete(&mut tokens),
                "disable" | "dis" => self.parse_disable(&mut tokens),
                "ignore" => self.parse_ignore(&mut tokens),
                "until" | "un" => self.parse_until(&mut tokens),
                // Debugger
                "goto" | "g" => self.parse_goto(&mut tokens),
                "next" | "n" => self.parse_next(&mut tokens),
                "registers" | "r" => self.parse_registers(&mut tokens),
                "return" | "ret" => self.parse_return(&mut tokens),
                "step" | "z" => self.parse_step(&mut tokens),
                // Memory
                "compare" | "c" => self.parse_compare(&mut tokens),
                "disass" | "d" => self.parse_disassemble(&mut tokens),
                "fill" | "f" => self.parse_fill(&mut tokens),
                "hunt" | "h" => self.parse_hunt(&mut tokens),
                "mem" | "m" => self.parse_memory(&mut tokens),
                "memchar" | "mc" => self.parse_mem_char(&mut tokens),
                "move" | "t" => self.parse_move(&mut tokens),
                "i" => self.parse_petscii(&mut tokens),
                // System
                "reset" => self.parse_reset(&mut tokens),
                "screen" | "sc" => self.parse_screen(&mut tokens),
                "stopwatch" | "sw" => self.parse_stopwatch(&mut tokens),
                // Monitor
                "exit" | "x" => self.parse_exit(&mut tokens),
                "help" | "?" => self.parse_help(&mut tokens),
                "quit" => self.parse_quit(&mut tokens),
                "radix" => self.parse_radix(&mut tokens),
                _ => Err(format!("Invalid command {}", input)),
            }
        } else {
            Err(format!("Invalid command {}", input))
        }
    }

    // -- Breakpoint

    fn parse_break(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        match address {
            Some(address) => Ok(Cmd::BpSet(address)),
            None => Ok(Cmd::BpList),
        }
    }

    fn parse_condition(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let index = self.parse_num(tokens.next())?;
        self.ensure_keyword("if", tokens)?;
        let expr = tokens.next();
        self.ensure_eos(tokens)?;
        if let Some(expr) = expr {
            Ok(Cmd::BpCondition(index, expr.to_string()))
        } else {
            Err(format!("Missing expression"))
        }
    }

    fn parse_delete(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::BpDelete(index))
    }

    fn parse_disable(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::BpDisable(index))
    }

    fn parse_enable(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let index = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::BpEnable(index))
    }

    fn parse_ignore(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let index = self.parse_num(tokens.next())?;
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::BpIgnore(index, count.unwrap_or(1)))
    }

    fn parse_until(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        match address {
            Some(address) => Ok(Cmd::BpUntil(address)),
            None => Ok(Cmd::BpList),
        }
    }

    // -- Debugger

    fn parse_goto(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Goto(address))
    }

    fn parse_next(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Next(count.unwrap_or(1)))
    }

    fn parse_registers(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let mut ops = Vec::new();
        while let Some(op) = self.parse_reg_op(tokens)? {
            ops.push(op);
            if self.parse_reg_sep(tokens)? == false {
                break;
            }
        }
        if ops.is_empty() {
            Ok(Cmd::RegRead)
        } else {
            Ok(Cmd::RegWrite(ops))
        }
    }

    fn parse_reg_op(&self, tokens: &mut Iterator<Item = &str>) -> Result<Option<RegOp>, String> {
        if let (Some(name), Some(op), Some(value)) = (tokens.next(), tokens.next(), tokens.next()) {
            match op.trim() {
                "=" => {
                    let reg = RegName::from(name.trim())?;
                    let op = match reg {
                        RegName::A => RegOp::SetA(self.parse_byte(value.trim())?),
                        RegName::X => RegOp::SetX(self.parse_byte(value.trim())?),
                        RegName::Y => RegOp::SetY(self.parse_byte(value.trim())?),
                        RegName::P => RegOp::SetP(self.parse_byte(value.trim())?),
                        RegName::SP => RegOp::SetSP(self.parse_byte(value.trim())?),
                        RegName::PC => RegOp::SetPC(self.parse_word(value.trim())?),
                    };
                    Ok(Some(op))
                }
                _ => Err(format!("invalid operator {}", op)),
            }
        } else {
            Ok(None)
        }
    }

    fn parse_reg_sep(&self, tokens: &mut Iterator<Item = &str>) -> Result<bool, String> {
        if let Some(sep) = tokens.next() {
            match sep.trim() {
                "," => Ok(true),
                _ => Err(format!("invalid token {}", sep)),
            }
        } else {
            Ok(false)
        }
    }

    fn parse_return(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Return)
    }

    fn parse_step(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let count = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Step(count.unwrap_or(1)))
    }

    // -- Memory

    fn parse_compare(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let target = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Compare(start, end, target))
    }

    fn parse_disassemble(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let start = self.parse_num_maybe(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Disassemble(start, end))
    }

    fn parse_fill(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
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
            Ok(Cmd::Fill(start, end, data))
        } else {
            Err(format!("Missing data"))
        }
    }

    fn parse_hunt(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
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
            Ok(Cmd::Hunt(start, end, data))
        } else {
            Err(format!("Missing data"))
        }
    }

    fn parse_memory(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let start = self.parse_num_maybe(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Memory(start, end))
    }

    fn parse_mem_char(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let address = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::MemChar(address))
    }

    fn parse_move(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num(tokens.next())?;
        let target = self.parse_num(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Move(start, end, target))
    }

    fn parse_petscii(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let start = self.parse_num(tokens.next())?;
        let end = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Petscii(start, end))
    }

    // -- System

    fn parse_reset(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let mode = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Reset(mode.unwrap_or(0) == 1))
    }

    fn parse_screen(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Screen)
    }

    fn parse_stopwatch(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let reset = if let Some(token) = tokens.next() {
            token == "reset"
        } else {
            false
        };
        Ok(Cmd::Stopwatch(reset))
    }

    // -- Monitor

    fn parse_help(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let command = tokens.next().map(|s| s.to_string());
        self.ensure_eos(tokens)?;
        Ok(Cmd::Help(command))
    }

    fn parse_exit(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Exit)
    }

    fn parse_quit(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        self.ensure_eos(tokens)?;
        Ok(Cmd::Quit)
    }

    fn parse_radix(&self, tokens: &mut Iterator<Item = &str>) -> Result<Cmd, String> {
        let radix = self.parse_num_maybe(tokens.next())?;
        self.ensure_eos(tokens)?;
        Ok(Cmd::Radix(radix))
    }

    // -- Helpers

    fn ensure_eos(&self, tokens: &mut Iterator<Item = &str>) -> Result<(), String> {
        match tokens.next() {
            Some(token) => Err(format!("Unexpected token {}", token)),
            None => Ok(()),
        }
    }

    fn ensure_keyword(
        &self,
        keyword: &str,
        tokens: &mut Iterator<Item = &str>,
    ) -> Result<(), String> {
        match tokens.next() {
            Some(token) if token.to_string().to_lowercase() == keyword => Ok(()),
            _ => Err(format!("Missing keyword {}", keyword)),
        }
    }

    fn parse_byte(&self, value: &str) -> Result<u8, String> {
        u8::from_str_radix(value, self.radix).map_err(|_| format!("Invalid number {}", value))
    }

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

    fn parse_word(&self, value: &str) -> Result<u16, String> {
        u16::from_str_radix(value, self.radix).map_err(|_| format!("Invalid number {}", value))
    }
}

struct CommandHelp;

impl CommandHelp {
    pub fn help(command: Option<String>) -> io::Result<String> {
        if let Some(command) = command {
            match command.trim().to_lowercase().as_str() {
                // Breakpoint
                "break" | "bk" => CommandHelp::help_cmd("break [address]", "bk"),
                "condition" | "cond" => {
                    CommandHelp::help_cmd("condition <index> if <cond_exp>", "cond")
                }
                "enable" | "en" => CommandHelp::help_cmd("enable [<index>]", "en"),
                "delete" | "del" => CommandHelp::help_cmd("delete [<index>]", "del"),
                "disable" | "dis" => CommandHelp::help_cmd("disable [<index>]", "dis"),
                "ignore" => CommandHelp::help_cmd("ignore <index> [<count>]", ""),
                "until" | "un" => CommandHelp::help_cmd("until <address>", "un"),
                // Debugger
                "goto" | "g" => CommandHelp::help_cmd("goto <address>", "g"),
                "next" | "n" => CommandHelp::help_cmd("next [<count>]", "n"),
                "registers" | "r" => {
                    CommandHelp::help_cmd("registers [<reg> = <num>[, <reg> = <num>]*]", "r")
                }
                "return" | "ret" => CommandHelp::help_cmd("return", "ret"),
                "step" | "z" => CommandHelp::help_cmd("step [<count>]", "z"),
                // Memory
                "compare" | "c" => CommandHelp::help_cmd("compare", "c"),
                "disass" | "d" => CommandHelp::help_cmd("disass [<address> [<address>]]", "d"),
                "fill" | "f" => CommandHelp::help_cmd("fill <address> <address> <data_list>", "f"),
                "hunt" | "h" => CommandHelp::help_cmd("hunt <address> <address> <data_list>", "h"),
                "mem" | "m" => CommandHelp::help_cmd("mem [<address> [<address>]]", "m"),
                "memchar" | "mc" => CommandHelp::help_cmd("memchar [<address>]", "mc"),
                "move" | "t" => CommandHelp::help_cmd("move <address> <address> <address>", "t"),
                "petscii" | "i" => CommandHelp::help_cmd("petscii <address> [<address>]", "i"),
                // System
                "reset" => CommandHelp::help_cmd("reset [<type>]", ""),
                "screen" | "sc" => CommandHelp::help_cmd("screen", "sc"),
                "stopwatch" | "sw" => CommandHelp::help_cmd("stopwatch [reset]", "sw"),
                // Monitor
                "exit" | "x" => CommandHelp::help_cmd("exit", "x"),
                "help" | "?" => CommandHelp::help_cmd("help", "?"),
                "quit" => CommandHelp::help_cmd("quit", ""),
                "radix" => CommandHelp::help_cmd("radix <num>", ""),
                _ => Err(Error::new(
                    ErrorKind::Other,
                    format!("Invalid command {}", command),
                )),
            }
        } else {
            CommandHelp::help_star()
        }
    }

    fn help_cmd(syntax: &str, short: &str) -> io::Result<String> {
        let abbr_line = if short.is_empty() {
            String::new()
        } else {
            format!("Shortname: {}\n", short)
        };
        let result = format!("Syntax: {}\n{}", syntax, abbr_line);
        Ok(result)
    }

    fn help_star() -> io::Result<String> {
        let mut buffer = String::new();
        buffer.push_str("* Breakpoint *\n");
        buffer.push_str("break (bk)\n");
        buffer.push_str("condition (cond)\n");
        buffer.push_str("enable (en)\n");
        buffer.push_str("delete (del)\n");
        buffer.push_str("disable (dis)\n");
        buffer.push_str("ignore\n");
        buffer.push_str("until (un)\n");
        buffer.push_str("\n");
        buffer.push_str("* Debug *\n");
        buffer.push_str("goto (g)\n");
        buffer.push_str("next (n)\n");
        buffer.push_str("registers (r)\n");
        buffer.push_str("return (ret)\n");
        buffer.push_str("step (z)\n");
        buffer.push_str("\n");
        buffer.push_str("* Memory *\n");
        buffer.push_str("compare (c)\n");
        buffer.push_str("disass (d)\n");
        buffer.push_str("fill (f)\n");
        buffer.push_str("hunt (h)\n");
        buffer.push_str("mem (m)\n");
        buffer.push_str("memchar (mc)\n");
        buffer.push_str("move (t)\n");
        buffer.push_str("petscii (i)\n");
        buffer.push_str("\n");
        buffer.push_str("* System *\n");
        buffer.push_str("reset\n");
        buffer.push_str("screen (sc)\n");
        buffer.push_str("stopwatch (sw)\n");
        buffer.push_str("\n");
        buffer.push_str("* Monitor *\n");
        buffer.push_str("exit (x)\n");
        buffer.push_str("help (?)\n");
        buffer.push_str("quit\n");
        buffer.push_str("radix\n");
        buffer.push_str("\n");
        Ok(buffer)
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

enum RegName {
    A,
    X,
    Y,
    P,
    SP,
    PC,
}

impl RegName {
    pub fn from(name: &str) -> Result<RegName, String> {
        match name {
            "a" | "A" => Ok(RegName::A),
            "x" | "X" => Ok(RegName::X),
            "y" | "Y" => Ok(RegName::Y),
            "p" | "P" => Ok(RegName::P),
            "sp" | "SP" => Ok(RegName::SP),
            "pc" | "PC" => Ok(RegName::PC),
            _ => Err(format!("invalid register {}", name)),
        }
    }
}
