// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::mpsc::Sender;

use byteorder::{BigEndian, WriteBytesExt};
use zinc64_debug::{Command, CommandResult, RegData, RegOp};
use zinc64_emu::system::C64;

// DEFERRED debugger: impl io

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State {
    Starting,
    Running,
    Paused,
    Halted,
    Stopped,
}

pub struct ExecutionEngine {
    // Dependencies
    c64: C64,
    // Runtime State
    debugger: Option<Sender<CommandResult>>,
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

    pub fn halt(&mut self) -> Result<(), String> {
        self.state = State::Halted;
        self.send_result(CommandResult::Unit)
    }

    pub fn execute(&mut self, command: &Command) -> Result<(), String> {
        match self.execute_internal(command) {
            Ok(CommandResult::Await) => Ok(()),
            Ok(result) => self.send_result(result),
            Err(error) => self.send_result(CommandResult::Error(error)),
        }
    }

    fn execute_internal(&mut self, command: &Command) -> Result<CommandResult, String> {
        match *command {
            Command::Attach(ref debugger) => Ok(self.attach(debugger)),
            Command::Detach => Ok(self.detach()),
            // Breakpoint
            Command::BpClear => Ok(self.bp_clear()),
            Command::BpCondition(index, ref expr, radix) => self.bp_condition(index, expr, radix),
            Command::BpDisable(index) => self.bp_enable(index, false),
            Command::BpDisableAll => Ok(self.bp_enable_all(false)),
            Command::BpEnable(index) => self.bp_enable(index, true),
            Command::BpEnableAll => Ok(self.bp_enable_all(true)),
            Command::BpIgnore(index, count) => self.bp_ignore(index, count),
            Command::BpList => Ok(self.bp_list()),
            Command::BpRemove(index) => self.bp_remove(index),
            Command::BpSet(address, autodelete) => Ok(self.bp_set(address, autodelete)),
            // Debugger
            Command::Continue => Ok(self.continue_()),
            Command::RegRead => Ok(self.reg_read()),
            Command::RegWrite(ref ops) => Ok(self.reg_write(ops)),
            Command::Step => Ok(self.step()),
            // Memory
            Command::MemRead(start, end) => Ok(self.mem_read(start, end)),
            Command::MemWrite(address, ref data) => Ok(self.mem_write(address, data)),
            // System
            Command::SysQuit => Ok(self.sys_quit()),
            Command::SysReset(hard) => Ok(self.sys_reset(hard)),
            Command::SysScreen => Ok(self.sys_screen()),
            Command::SysStopwatch(reset) => self.sys_stopwatch(reset),
        }
    }

    fn send_result(&self, result: CommandResult) -> Result<(), String> {
        if let Some(ref debugger) = self.debugger {
            debugger
                .send(result)
                .map_err(|_| "Failed to send result".to_string())
        } else {
            Ok(())
        }
    }

    // -- Commands

    fn attach(&mut self, debugger: &Sender<CommandResult>) -> CommandResult {
        self.debugger = Some(debugger.clone());
        self.state = State::Halted;
        CommandResult::Unit
    }

    fn detach(&mut self) -> CommandResult {
        self.debugger = None;
        self.state = State::Running;
        CommandResult::Unit
    }

    // -- Breakpoint

    fn bp_clear(&mut self) -> CommandResult {
        let bpm = self.c64.get_bpm_mut();
        bpm.clear();
        CommandResult::Unit
    }

    fn bp_condition(
        &mut self,
        index: u16,
        expr: &str,
        radix: u32,
    ) -> Result<CommandResult, String> {
        let bpm = self.c64.get_bpm_mut();
        bpm.set_condition(index, expr, Some(radix))?;
        let bp = bpm.get(index)?;
        let buffer = format!(
            "Setting condition for breakpoint {} to: {}\n",
            bp.index,
            bp.condition
                .as_ref()
                .map(|cond| format!("{}", cond))
                .unwrap_or_else(|| "".to_string())
        );
        Ok(CommandResult::Text(buffer))
    }

    fn bp_enable(&mut self, index: u16, enabled: bool) -> Result<CommandResult, String> {
        let bpm = self.c64.get_bpm_mut();
        bpm.set_enabled(index, enabled)?;
        Ok(CommandResult::Unit)
    }

    fn bp_enable_all(&mut self, enabled: bool) -> CommandResult {
        let bpm = self.c64.get_bpm_mut();
        bpm.enable_all(enabled);
        CommandResult::Unit
    }

    fn bp_ignore(&mut self, index: u16, count: u16) -> Result<CommandResult, String> {
        let bpm = self.c64.get_bpm_mut();
        bpm.ignore(index, count)?;
        Ok(CommandResult::Unit)
    }

    fn bp_list(&self) -> CommandResult {
        let bpm = self.c64.get_bpm();
        let mut buffer = String::new();
        for bp in bpm.list() {
            buffer.push_str(
                format!(
                    "Bp {}: ${:04x}{}{}\n",
                    bp.index,
                    bp.address,
                    bp.condition
                        .as_ref()
                        .map_or(String::new(), |cond| format!(" if {}", cond)),
                    if bp.enabled { "" } else { " disabled" },
                )
                .as_str(),
            );
        }
        if buffer.is_empty() {
            buffer.push_str("No breakpoints are set\n");
        }
        CommandResult::Text(buffer)
    }

    fn bp_remove(&mut self, index: u16) -> Result<CommandResult, String> {
        let bpm = self.c64.get_bpm_mut();
        bpm.remove(index)?;
        Ok(CommandResult::Unit)
    }

    fn bp_set(&mut self, address: u16, autodelete: bool) -> CommandResult {
        let bpm = self.c64.get_bpm_mut();
        let index = bpm.set(address, autodelete);
        let buffer = format!("Bp {}: ${:04x}\n", index, address);
        CommandResult::Text(buffer)
    }

    // Debugger

    fn continue_(&mut self) -> CommandResult {
        self.state = State::Running;
        CommandResult::Await
    }

    fn reg_read(&mut self) -> CommandResult {
        let cpu = self.c64.get_cpu();
        let regs = RegData {
            a: cpu.get_a(),
            x: cpu.get_x(),
            y: cpu.get_y(),
            p: cpu.get_p(),
            sp: cpu.get_sp(),
            pc: cpu.get_pc(),
            port_00: cpu.read(0x00),
            port_01: cpu.read(0x01),
        };
        CommandResult::Registers(regs)
    }

    fn reg_write(&mut self, ops: &[RegOp]) -> CommandResult {
        let cpu = self.c64.get_cpu_mut();
        for op in ops {
            match *op {
                RegOp::SetA(value) => cpu.set_a(value),
                RegOp::SetX(value) => cpu.set_x(value),
                RegOp::SetY(value) => cpu.set_y(value),
                RegOp::SetP(value) => cpu.set_p(value),
                RegOp::SetSP(value) => cpu.set_sp(value),
                RegOp::SetPC(value) => cpu.set_pc(value),
            }
        }
        CommandResult::Unit
    }

    fn step(&mut self) -> CommandResult {
        self.c64.step();
        let bp_hit = if self.c64.check_breakpoints() { 1 } else { 0 };
        CommandResult::Number(bp_hit)
    }

    // -- Memory

    fn mem_read(&self, start: u16, end: u16) -> CommandResult {
        let cpu = self.c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = start;
        while address < end {
            buffer.push(cpu.read(address));
            address = address.wrapping_add(1);
        }
        CommandResult::Buffer(buffer)
    }

    fn mem_write(&mut self, address: u16, data: &[u8]) -> CommandResult {
        self.c64.load(data, address);
        CommandResult::Unit
    }

    // -- System

    fn sys_quit(&mut self) -> CommandResult {
        self.state = State::Stopped;
        CommandResult::Unit
    }

    fn sys_reset(&mut self, hard: bool) -> CommandResult {
        self.c64.reset(hard);
        CommandResult::Unit
    }

    fn sys_screen(&self) -> CommandResult {
        let cia2 = self.c64.get_cia_2();
        let vic = self.c64.get_vic();
        let cia2_port_a = cia2.borrow_mut().read(0x00);
        let vm = (((vic.borrow_mut().read(0x18) & 0xf0) >> 4) as u16) << 10;
        let vm_base = ((!cia2_port_a & 0x03) as u16) << 14 | vm;
        CommandResult::Number(vm_base)
    }

    fn sys_stopwatch(&mut self, reset: bool) -> Result<CommandResult, String> {
        let clock = self.c64.get_clock();
        if reset {
            clock.reset();
        }
        let mut buffer = Vec::new();
        buffer
            .write_u64::<BigEndian>(clock.get())
            .map_err(|_| "Op failed")?;
        Ok(CommandResult::Buffer(buffer))
    }
}
