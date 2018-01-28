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

use std::sync::mpsc::Sender;

use bit_field::BitField;
use zinc64::system::C64;

use super::charset;
use super::command;
use super::command::{Command, CommandResult, SystemState, Response};
use super::disassembler::Disassembler;

const OPCODE_JSR: u8 = 0x20;
const OPCODE_RTS: u8 = 0x60;

// TODO debugger: impl io
// TODO debugger: print triggered breakpoint

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State {
    Starting,
    Running,
    Paused,
    Stopped,
}

pub struct ExecutionEngine {
    // Dependencies
    c64: C64,
    // Runtime State
    debugger: Option<Sender<Response>>,
    state: State,
}

impl ExecutionEngine {
    pub fn new(c64: C64) -> Self {
        Self {
            c64,
            debugger: None,
            state: State::Starting,
        }
    }

    pub fn get_c64(&self) -> &C64 {
        &self.c64
    }

    pub fn get_c64_mut(&mut self) -> &mut C64 {
        &mut self.c64
    }

    pub fn get_state(&self) -> State {
        self.state
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn execute(&mut self, command: &Command) -> Result<(), String> {
        match self.execute_internal(command) {
            Ok(CommandResult::Deferred) => Ok(()),
            Ok(result) => self.send_result(result),
            Err(error) => self.send_result(CommandResult::Error(error)),
        }
    }

    fn execute_internal(&mut self, command: &Command) -> Result<CommandResult, String> {
        match *command {
            Command::Attach(ref debugger) => self.attach(debugger),
            Command::Break => self.break_(),
            Command::Detach => self.detach(),
            // Machine
            Command::Goto(address) => self.goto(address),
            Command::Next(count) => self.next(count),
            Command::Registers => self.registers(),
            Command::Reset(hard) => self.reset(hard),
            Command::Return => self.return_(),
            Command::Screen => self.screen(),
            Command::Step(steps) => self.step(steps),
            Command::Stopwatch(reset) => self.stopwatch(reset),
            // Memory
            Command::Compare(start, end, target) => self.compare(start, end, target),
            Command::Disassemble(start, end) => self.disassemble(start, end),
            Command::Fill(start, end, ref data) => self.fill(start, end, data),
            Command::Hunt(start, end, ref data) => self.hunt(start, end, data),
            Command::MemChar(address) => self.mem_char(address),
            Command::MemPetscii(start, end) => self.mem_petscii(start, end),
            Command::Move(start, end, target) => self.move_(start, end, target),
            Command::Read(start, end) => self.read(start, end),
            Command::Write(address, ref data) => self.write(address, data),
            // Monitor
            Command::Quit => self.quit(),
            // Breakpoints
            Command::BpDelete(index) => self.bp_delete(index),
            Command::BpDisable(index) => self.bp_enable(index, false),
            Command::BpEnable(index) => self.bp_enable(index, true),
            Command::BpIgnore(index, count) => self.bp_ignore(index, count),
            Command::BpList => self.bp_list(),
            Command::BpSet(address) => self.bp_set(address),
            Command::BpUntil(address) => self.bp_until(address),
            _ => Err(format!("Unsupported command")),
        }
    }

    fn send_result(&self, result: CommandResult) -> Result<(), String> {
        if let Some(ref debugger) = self.debugger {
            debugger.send(Response(result, self.get_system_state()))
                .map_err(|_| format!("Failed to failed resonse"))
        } else {
            Ok(())
        }
    }

    // -- Commands

    fn attach(&mut self, debugger: &Sender<Response>) -> Result<CommandResult, String> {
        self.debugger = Some(debugger.clone());
        self.state = State::Paused;
        Ok(CommandResult::Empty)
    }

    fn break_(&mut self) -> Result<CommandResult, String> {
        self.state = State::Paused;
        Ok(CommandResult::Empty)
    }

    fn detach(&mut self) -> Result<CommandResult, String> {
        self.debugger = None;
        self.state = State::Running;
        Ok(CommandResult::Empty)
    }

    // -- Machine

    fn goto(&mut self, address: u16) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        cpu.borrow_mut().set_pc(address);
        self.state = State::Running;
        Ok(CommandResult::Empty)
    }

    fn next(&mut self, _count: u16) -> Result<CommandResult, String> { // FIXME debugger: impl next.count
        let cpu = self.c64.get_cpu();
        let pc = cpu.borrow().get_pc();
        let opcode = cpu.borrow().read_debug(pc);
        if opcode == OPCODE_JSR {
            let target = pc + 3;
            self.bp_until(target)
        } else {
            self.c64.step();
            Ok(self.execution_state())
        }
    }

    fn registers(&mut self) -> Result<CommandResult, String> {
        Ok(CommandResult::Registers(self.build_cpu_state()))
    }

    fn reset(&mut self, hard: bool) -> Result<CommandResult, String> {
        self.c64.reset(hard);
        Ok(self.execution_state())
    }

    fn return_(&mut self) -> Result<CommandResult, String> {
        loop {
            let cpu = self.c64.get_cpu();
            let pc = cpu.borrow().get_pc();
            let opcode = cpu.borrow().read_debug(pc);
            if opcode == OPCODE_RTS {
                break;
            }
            self.c64.step();
            if self.c64.check_breakpoints() {
                break;
            }
        }
        Ok(self.execution_state())
    }

    fn screen(&self) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let cia2 = self.c64.get_cia_2();
        let vic = self.c64.get_vic();
        let cia2_port_a = cia2.borrow_mut().read(0x00);
        let vm = (((vic.borrow_mut().read(0x18) & 0xf0) >> 4) as u16) << 10; // FIXME
        let vm_base = ((!cia2_port_a & 0x03) as u16) << 14 | vm;
        let mut buffer = String::new();
        let mut counter = 0;
        let mut address = vm_base;
        let end_address = address + 1000;
        buffer.push_str(format!("Displaying 40x25 screen at ${:04x}\n", vm_base).as_str());
        while address < end_address {
            if counter % 40 == 0 {
                buffer.push_str("\n");
            }
            let code = cpu.borrow().read_debug(address);
            let ascii = match charset::screen_code_to_ascii(code) {
                0 => 46,
                v => v,
            };
            buffer.push(char::from(ascii));
            counter += 1;
            address = address.wrapping_add(1);
        }
        Ok(CommandResult::Text(vec![buffer]))
    }

    fn step(&mut self, count: u16) -> Result<CommandResult, String> {
        for _i in 0..count {
            self.c64.step();
            if self.c64.check_breakpoints() {
                break;
            }
        }
        Ok(self.execution_state())
    }

    fn stopwatch(&mut self, reset: bool) -> Result<CommandResult, String> {
        let clock = self.c64.get_clock();
        if reset {
            clock.reset();
        }
        let buffer = format!("Clock: {}", clock.get());
        Ok(CommandResult::Text(vec![buffer]))
    }

    // -- Memory

    fn compare(&self, start: u16, end: u16, target: u16) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = start;
        let mut target = target;
        while address < end {
            let source = cpu.borrow().read_debug(address);
            let dest = cpu.borrow().read_debug(target);
            if source != dest {
                buffer.push(format!("${:04x} ${:04x}: {:02x} {:02x}", address, target, source, dest));
            }
            address = address.wrapping_add(1);
            target = target.wrapping_add(1);
        }
        Ok(CommandResult::Text(buffer))
    }

    fn disassemble(&self, start: Option<u16>, end: Option<u16>) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let dis = Disassembler::new(cpu.clone());
        let mut buffer = Vec::new();
        let mut address = start.unwrap_or(cpu.borrow().get_pc());
        let end_address = end.unwrap_or(address + 64);
        while address < end_address {
            let (instr, instr_len) = dis.disassemble(address);
            let mut instr_bytes = String::new();
            for i in 0..instr_len as u16 {
                let byte = cpu.borrow().read_debug(address + i);
                instr_bytes.push_str(format!("{:02x} ", byte).as_str());
            }
            let instr_text = format!("{}", instr);
            buffer.push(format!("${:04x}  {:12} {}", address, instr_bytes, instr_text));
            address = address.wrapping_add(instr_len as u16);
        }
        Ok(CommandResult::Text(buffer))
    }

    fn fill(&self, start: u16, end: u16, data: &Vec<u8>) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut address = start;
        let mut index = 0;
        while address < end {
            if index >= data.len() {
                index = 0;
            }
            cpu.borrow_mut().write_debug(address, data[index]);
            address = address.wrapping_add(1);
            index += 1;
        }
        Ok(CommandResult::Empty)
    }

    fn hunt(&self, start: u16, end: u16, data: &Vec<u8>) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = start;
        while address < end {
            let mut found = true;
            for i in 0..data.len() {
                if cpu.borrow().read_debug(address + i as u16) != data[i] {
                    found = false;
                    break;
                }
            }
            if found {
                buffer.push(format!("{:04x}", address));
            }
            address = address.wrapping_add(1);
        }
        Ok(CommandResult::Text(buffer))
    }

    fn mem_char(&self, address: u16) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = address;
        let end_address = address + 8;
        while address < end_address {
            let value = cpu.borrow().read_debug(address);
            let mut s = String::new();
            s.push_str(format!("${:04x} ", address).as_str());
            for i in 0..8 {
                let bit = if value.get_bit(7 - i) { "." } else { "*" };
                s.push_str(bit);
            }
            buffer.push(s);
            address = address.wrapping_add(1);
        }
        Ok(CommandResult::Text(buffer))
    }

    fn mem_petscii(&self, start: u16, end: Option<u16>) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut address = start;
        let end_address = end.unwrap_or(start.wrapping_add(400));
        let mut buffer = String::new();
        let mut counter = 0;
        while address < end_address {
            if counter % 40 == 0 {
                buffer.push_str("\n");
            }
            let code = cpu.borrow().read_debug(address);
            let ascii = match charset::pet_to_ascii(code) {
                0 => 46,
                v => v,
            };
            buffer.push(char::from(ascii));
            counter += 1;
            address = address.wrapping_add(1);
        }
        Ok(CommandResult::Text(vec![buffer]))
    }

    fn move_(&self, start: u16, end: u16, target: u16) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let buffer = Vec::new();
        let mut address = start;
        let mut target = target;
        while address < end {
            let source = cpu.borrow().read_debug(address);
            cpu.borrow_mut().write_debug(target, source);
            address = address.wrapping_add(1);
            target = target.wrapping_add(1);
        }
        Ok(CommandResult::Text(buffer))
    }

    fn read(&self, start: u16, end: Option<u16>) -> Result<CommandResult, String> {
        let cpu = self.c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = start;
        let end_address = end.unwrap_or(start + 96);
        while address < end_address {
            buffer.push(cpu.borrow().read_debug(address));
            address = address.wrapping_add(1);
        }
        Ok(CommandResult::Memory(start, buffer))
    }

    fn write(&mut self, address: u16, data: &Vec<u8>) -> Result<CommandResult, String> {
        self.c64.load(data, address);
        Ok(CommandResult::Empty)
    }

    // -- Monitor

    fn quit(&mut self) -> Result<CommandResult, String> {
        self.state = State::Stopped;
        Ok(CommandResult::Empty)
    }

    // -- Breakpoints

    fn bp_delete(&mut self, index: Option<u16>) -> Result<CommandResult, String> {
        if self.c64.remove_breakpoint(index) {
            Ok(CommandResult::Empty)
        } else {
            Err(format!("Invalid breakpoint {}", index.unwrap_or(0)))
        }
    }

    fn bp_enable(&mut self, index: Option<u16>, enabled: bool) -> Result<CommandResult, String> {
        if self.c64.enable_breakpoint(index, enabled) {
            Ok(CommandResult::Empty)
        } else {
            Err(format!("Invalid breakpoint {}", index.unwrap_or(0)))
        }
    }

    fn bp_ignore(&mut self, index: u16, count: u16) -> Result<CommandResult, String> {
        if self.c64.ignore_breakpoint(index, count) {
            Ok(CommandResult::Empty)
        } else {
            Err(format!("Invalid breakpoint {}", index))
        }
    }

    fn bp_list(&self) -> Result<CommandResult, String> {
        let mut buffer = Vec::new();
        for bp in self.c64.list_breakpoints() {
            buffer.push(format!("Bp {}: ${:04x}", bp.index, bp.address));
        }
        Ok(CommandResult::Text(buffer))
    }

    fn bp_set(&mut self, address: u16) -> Result<CommandResult, String> {
        let index = self.c64.add_breakpoint(address, false);
        let mut buffer = Vec::new();
        buffer.push(format!("Bp {}: ${:04x}", index, address));
        Ok(CommandResult::Text(buffer))
    }

    fn bp_until(&mut self, address: u16) -> Result<CommandResult, String> {
        self.c64.add_breakpoint(address, true);
        self.state = State::Running;
        // let mut buffer = Vec::new();
        // buffer.push(format!("Bp {}: ${:04x}", index, address));
        Ok(CommandResult::Deferred)
    }

    // -- Helpers

    fn build_cpu_state(&self) -> command::CpuState {
        let cpu = self.c64.get_cpu();
        let cpu_state = command::CpuState {
            a: cpu.borrow().get_a(),
            x: cpu.borrow().get_x(),
            y: cpu.borrow().get_y(),
            sp: cpu.borrow().get_sp(),
            p: cpu.borrow().get_p(),
            pc: cpu.borrow().get_pc(),
            port_00: cpu.borrow().read_debug(0x00),
            port_01: cpu.borrow().read_debug(0x01),
        };
        cpu_state
    }

    fn execution_state(&self) -> CommandResult {
        let cpu = self.c64.get_cpu();
        let pc = cpu.borrow().get_pc();
        let dis = Disassembler::new(cpu.clone());
        let (instr, instr_len) = dis.disassemble(pc);
        let mut instr_bytes = Vec::new();
        for i in 0..instr_len as u16 {
            instr_bytes.push(cpu.borrow().read_debug(pc + i));
        }
        let instr_text = format!("{}", instr);
        let instruction_info = command::InstructionInfo {
            opcode: cpu.borrow().read_debug(pc),
            instr_bytes,
            instr_text,
        };
        CommandResult::ExecutionState(self.build_cpu_state(), instruction_info)
    }

    fn get_system_state(&self) -> SystemState {
        let cpu = self.c64.get_cpu();
        let sys_state = SystemState {
            pc: cpu.borrow().get_pc(),
        };
        sys_state
    }
}
