// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(unused)]
#![allow(clippy::cast_lossless)]

use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use byteorder::{BigEndian, WriteBytesExt};
use zinc64_engine::C64;
use zinc64_engine::chip::{Cpu6510, Register};

use crate::breakpoint::BreakpointManager;
use crate::{Command, Output, RegData, RegOp};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Directive {
    Halt,
    Resume,
    Quit,
}

struct CmdResult(Output, Option<Directive>);

impl CmdResult {
    pub fn ok(result: Output) -> Result<CmdResult, String> {
        Ok(CmdResult(result, None))
    }

    pub fn ok_with_state(result: Output, directive: Directive) -> Result<CmdResult, String> {
        Ok(CmdResult(result, Some(directive)))
    }

    pub fn unit() -> Result<CmdResult, String> {
        Ok(CmdResult(Output::Unit, None))
    }
}

// DEFERRED debugger: impl io

pub struct Executor {
    breakpoints: BreakpointManager,
    debug_rx: mpsc::Receiver<Command>,
    debugger: Option<Sender<Output>>,
}

impl Executor {
    pub fn new(debug_rx: mpsc::Receiver<Command>) -> Self {
        Self {
            breakpoints: BreakpointManager::default(),
            debug_rx,
            debugger: None,
        }
    }

    // ---- command polling ----

    pub fn poll(&mut self, debugging: bool) -> Option<Command> {
        if debugging {
            self.debug_rx.recv_timeout(Duration::from_millis(1)).ok()
        } else {
            self.debug_rx.try_recv().ok()
        }
    }

    // ---- command execution ----

    pub fn execute(
        &mut self,
        c64: &mut C64,
        command: &Command,
    ) -> Result<Option<Directive>, String> {
        match self.execute_internal(c64, command) {
            Ok(CmdResult(Output::Await, new_state)) => Ok(new_state),
            Ok(CmdResult(result, new_state)) => {
                self.send_result(result);
                Ok(new_state)
            }
            Err(error) => {
                self.send_result(Output::Error(error.clone()));
                Err(error)
            }
        }
    }

    fn execute_internal(&mut self, c64: &mut C64, command: &Command) -> Result<CmdResult, String> {
        match *command {
            Command::Attach(ref debugger) => self.attach(c64, debugger),
            Command::Detach => self.detach(c64),
            Command::Continue => self.continue_(c64),
            Command::SysQuit => self.quit(c64),
            Command::Step => self.step(c64),
            Command::BpClear => self.bp_clear(),
            Command::BpCondition(index, ref expr, radix) => self.bp_condition(index, expr, radix),
            Command::BpDisable(index) => self.bp_enable(index, false),
            Command::BpDisableAll => self.bp_enable_all(false),
            Command::BpEnable(index) => self.bp_enable(index, true),
            Command::BpEnableAll => self.bp_enable_all(true),
            Command::BpIgnore(index, count) => self.bp_ignore(index, count),
            Command::BpList => self.bp_list(),
            Command::BpRemove(index) => self.bp_remove(index),
            Command::BpSet(address, autodelete) => self.bp_set(address, autodelete),
            Command::MemRead(start, end) => self.mem_read(c64, start, end),
            Command::MemWrite(address, ref data) => self.mem_write(c64, address, data),
            Command::RegRead => self.reg_read(c64),
            Command::RegWrite(ref ops) => self.reg_write(c64, ops),
            Command::SysReset(hard) => self.sys_reset(c64, hard),
            Command::SysScreen => self.sys_screen(c64),
            Command::SysStopwatch(reset) => self.sys_stopwatch(c64, reset),
        }
    }

    fn attach(&mut self, c64: &mut C64, debugger: &Sender<Output>) -> Result<CmdResult, String> {
        self.debugger = Some(debugger.clone());
        CmdResult::ok_with_state(Output::Unit, Directive::Halt)
    }

    fn detach(&mut self, c64: &mut C64) -> Result<CmdResult, String> {
        self.debugger = None;
        CmdResult::ok_with_state(Output::Unit, Directive::Resume)
    }

    fn continue_(&self, c64: &mut C64) -> Result<CmdResult, String> {
        CmdResult::ok_with_state(Output::Await, Directive::Resume)
    }

    fn quit(&self, c64: &mut C64) -> Result<CmdResult, String> {
        CmdResult::ok_with_state(Output::Unit, Directive::Quit)
    }

    fn step(&mut self, c64: &mut C64) -> Result<CmdResult, String> {
        c64.step();
        let bp_hit = if self.check_breakpoint(c64.get_cpu()) {
            1
        } else {
            0
        };
        CmdResult::ok(Output::Number(bp_hit))
    }

    fn bp_clear(&mut self) -> Result<CmdResult, String> {
        self.breakpoints.clear();
        CmdResult::unit()
    }

    fn bp_condition(&mut self, index: u16, expr: &str, radix: u32) -> Result<CmdResult, String> {
        let bpm = &mut self.breakpoints;
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
        CmdResult::ok(Output::Text(buffer))
    }

    fn bp_enable(&mut self, index: u16, enabled: bool) -> Result<CmdResult, String> {
        self.breakpoints.set_enabled(index, enabled)?;
        CmdResult::unit()
    }

    fn bp_enable_all(&mut self, enabled: bool) -> Result<CmdResult, String> {
        self.breakpoints.enable_all(enabled);
        CmdResult::unit()
    }

    fn bp_ignore(&mut self, index: u16, count: u16) -> Result<CmdResult, String> {
        self.breakpoints.ignore(index, count)?;
        CmdResult::unit()
    }

    fn bp_list(&self) -> Result<CmdResult, String> {
        let mut buffer = String::new();
        for bp in self.breakpoints.list() {
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
        CmdResult::ok(Output::Text(buffer))
    }

    fn bp_remove(&mut self, index: u16) -> Result<CmdResult, String> {
        self.breakpoints.remove(index)?;
        CmdResult::unit()
    }

    fn bp_set(&mut self, address: u16, autodelete: bool) -> Result<CmdResult, String> {
        let index = self.breakpoints.set(address, autodelete);
        let buffer = format!("Bp {}: ${:04x}\n", index, address);
        CmdResult::ok(Output::Text(buffer))
    }

    fn mem_read(&self, c64: &mut C64, start: u16, end: u16) -> Result<CmdResult, String> {
        let mut buffer = Vec::new();
        let mut address = start;
        while address < end {
            buffer.push(c64.peek(address));
            address = address.wrapping_add(1);
        }
        CmdResult::ok(Output::Buffer(buffer))
    }

    fn mem_write(&self, c64: &mut C64, address: u16, data: &[u8]) -> Result<CmdResult, String> {
        c64.load(data, address);
        CmdResult::unit()
    }

    fn reg_read(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let clock = c64.get_cycles();
        let port_00 = c64.peek(0x00);
        let port_01 = c64.peek(0x01);
        let cpu = c64.get_cpu();
        let regs = RegData {
            a: cpu.get_register(Register::A),
            x: cpu.get_register(Register::X),
            y: cpu.get_register(Register::Y),
            p: cpu.get_register(Register::P),
            sp: cpu.get_register(Register::SP),
            pc: cpu.get_pc(),
            port_00,
            port_01,
            clock,
        };
        CmdResult::ok(Output::Registers(regs))
    }

    fn reg_write(&self, c64: &mut C64, ops: &[RegOp]) -> Result<CmdResult, String> {
        let cpu = c64.get_cpu_mut();
        for op in ops {
            match *op {
                RegOp::SetA(value) => cpu.set_register(Register::A, value),
                RegOp::SetX(value) => cpu.set_register(Register::X, value),
                RegOp::SetY(value) => cpu.set_register(Register::Y, value),
                RegOp::SetP(value) => cpu.set_register(Register::P, value),
                RegOp::SetSP(value) => cpu.set_register(Register::SP, value),
                RegOp::SetPC(value) => cpu.set_pc(value),
            }
        }
        CmdResult::unit()
    }

    fn sys_reset(&self, c64: &mut C64, hard: bool) -> Result<CmdResult, String> {
        c64.reset(hard);
        CmdResult::unit()
    }

    fn sys_screen(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let cia2_port_a = c64.get_cia_2().port_a().get_value();
        let vm = (((c64.get_vic().peek(0x18) & 0xf0) >> 4) as u16) << 10;
        let vm_base = ((!cia2_port_a & 0x03) as u16) << 14 | vm;
        CmdResult::ok(Output::Number(vm_base))
    }

    fn sys_stopwatch(&self, c64: &mut C64, reset: bool) -> Result<CmdResult, String> {
        if reset {
            c64.reset_stopwatch();
        }
        let mut buffer = Vec::new();
        buffer
            .write_u64::<BigEndian>(c64.stopwatch())
            .map_err(|_| "Op failed")?;
        CmdResult::ok(Output::Buffer(buffer))
    }

    // ---- breakpoints ----

    pub fn set_breakpoint(&mut self, address: u16) {
        self.breakpoints.set(address, false);
    }

    pub fn has_breakpoints(&self) -> bool {
        self.breakpoints.is_bp_present()
    }

    pub fn check_breakpoint(&mut self, cpu: &Cpu6510) -> bool {
        self.breakpoints.check(cpu).is_some()
    }

    // ---- halt notification ----

    pub fn halt(&mut self) -> Result<(), String> {
        self.send_result(Output::Unit)
    }

    // ---- shared ----

    fn send_result(&self, result: Output) -> Result<(), String> {
        if let Some(ref debugger) = self.debugger {
            debugger
                .send(result)
                .map_err(|_| "Failed to send result".to_string())
        } else {
            Ok(())
        }
    }
}
