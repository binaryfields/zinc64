// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(unused)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use byteorder::{BigEndian, WriteBytesExt};
use zinc64_debug::{Command, Output, RegData, RegOp};
use zinc64_emu::system::C64;

use crate::app::RuntimeState;

// DEFERRED debugger: impl io

struct CmdResult(Output, Option<RuntimeState>);

impl CmdResult {
    pub fn ok(result: Output) -> Result<CmdResult, String> {
        Ok(CmdResult(result, None))
    }

    pub fn ok_with_state(result: Output, state: RuntimeState) -> Result<CmdResult, String> {
        Ok(CmdResult(result, Some(state)))
    }

    pub fn unit() -> Result<CmdResult, String> {
        Ok(CmdResult(Output::Unit, None))
    }
}

pub struct Debug {
    debug_rx: mpsc::Receiver<Command>,
    debugger: Option<Sender<Output>>,
}

impl Debug {
    pub fn new(debug_rx: mpsc::Receiver<Command>) -> Self {
        Self {
            debug_rx,
            debugger: None,
        }
    }

    pub fn execute(
        &mut self,
        c64: &mut C64,
        command: &Command,
    ) -> Result<Option<RuntimeState>, String> {
        match self.execute_internal(c64, &command) {
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

    pub fn halt(&mut self) -> Result<(), String> {
        self.send_result(Output::Unit)
    }

    pub fn poll(&mut self, debugging: bool) -> Option<Command> {
        if debugging {
            self.debug_rx.recv_timeout(Duration::from_millis(1)).ok()
        } else {
            self.debug_rx.try_recv().ok()
        }
    }

    fn execute_internal(&mut self, c64: &mut C64, command: &Command) -> Result<CmdResult, String> {
        match *command {
            Command::Attach(ref debugger) => self.attach(c64, debugger),
            Command::Detach => self.detach(c64),
            Command::Continue => self.continue_(c64),
            Command::SysQuit => self.quit(c64),
            Command::Step => self.step(c64),
            Command::BpClear => self.bp_clear(c64),
            Command::BpCondition(index, ref expr, radix) => {
                self.bp_condition(c64, index, expr, radix)
            }
            Command::BpDisable(index) => self.bp_enable(c64, index, false),
            Command::BpDisableAll => self.bp_enable_all(c64, false),
            Command::BpEnable(index) => self.bp_enable(c64, index, true),
            Command::BpEnableAll => self.bp_enable_all(c64, true),
            Command::BpIgnore(index, count) => self.bp_ignore(c64, index, count),
            Command::BpList => self.bp_list(c64),
            Command::BpRemove(index) => self.bp_remove(c64, index),
            Command::BpSet(address, autodelete) => self.bp_set(c64, address, autodelete),
            Command::MemRead(start, end) => self.mem_read(c64, start, end),
            Command::MemWrite(address, ref data) => self.mem_write(c64, address, data),
            Command::RegRead => self.reg_read(c64),
            Command::RegWrite(ref ops) => self.reg_write(c64, ops),
            Command::SysReset(hard) => self.sys_reset(c64, hard),
            Command::SysScreen => self.sys_screen(c64),
            Command::SysStopwatch(reset) => self.sys_stopwatch(c64, reset),
        }
    }

    fn send_result(&self, result: Output) -> Result<(), String> {
        if let Some(ref debugger) = self.debugger {
            debugger
                .send(result)
                .map_err(|_| "Failed to send result".to_string())
        } else {
            Ok(())
        }
    }

    // -- Commands

    fn attach(&mut self, c64: &mut C64, debugger: &Sender<Output>) -> Result<CmdResult, String> {
        self.debugger = Some(debugger.clone());
        CmdResult::ok_with_state(Output::Unit, RuntimeState::Halted)
    }

    fn detach(&mut self, c64: &mut C64) -> Result<CmdResult, String> {
        self.debugger = None;
        CmdResult::ok_with_state(Output::Unit, RuntimeState::Running)
    }

    fn continue_(&self, c64: &mut C64) -> Result<CmdResult, String> {
        CmdResult::ok_with_state(Output::Await, RuntimeState::Running)
    }

    fn quit(&self, c64: &mut C64) -> Result<CmdResult, String> {
        CmdResult::ok_with_state(Output::Unit, RuntimeState::Stopped)
    }

    fn step(&self, c64: &mut C64) -> Result<CmdResult, String> {
        c64.step();
        let bp_hit = if c64.check_breakpoints() { 1 } else { 0 };
        CmdResult::ok(Output::Number(bp_hit))
    }

    fn bp_clear(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        bpm.clear();
        CmdResult::unit()
    }

    fn bp_condition(
        &self,
        c64: &mut C64,
        index: u16,
        expr: &str,
        radix: u32,
    ) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
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

    fn bp_enable(&self, c64: &mut C64, index: u16, enabled: bool) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        bpm.set_enabled(index, enabled)?;
        CmdResult::unit()
    }

    fn bp_enable_all(&self, c64: &mut C64, enabled: bool) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        bpm.enable_all(enabled);
        CmdResult::unit()
    }

    fn bp_ignore(&self, c64: &mut C64, index: u16, count: u16) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        bpm.ignore(index, count)?;
        CmdResult::unit()
    }

    fn bp_list(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm();
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
        CmdResult::ok(Output::Text(buffer))
    }

    fn bp_remove(&self, c64: &mut C64, index: u16) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        bpm.remove(index)?;
        CmdResult::unit()
    }

    fn bp_set(&self, c64: &mut C64, address: u16, autodelete: bool) -> Result<CmdResult, String> {
        let bpm = c64.get_bpm_mut();
        let index = bpm.set(address, autodelete);
        let buffer = format!("Bp {}: ${:04x}\n", index, address);
        CmdResult::ok(Output::Text(buffer))
    }

    fn mem_read(&self, c64: &mut C64, start: u16, end: u16) -> Result<CmdResult, String> {
        let cpu = c64.get_cpu();
        let mut buffer = Vec::new();
        let mut address = start;
        while address < end {
            buffer.push(cpu.read(address));
            address = address.wrapping_add(1);
        }
        CmdResult::ok(Output::Buffer(buffer))
    }

    fn mem_write(&self, c64: &mut C64, address: u16, data: &[u8]) -> Result<CmdResult, String> {
        c64.load(data, address);
        CmdResult::unit()
    }

    fn reg_read(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let clock = c64.get_clock().get();
        let cpu = c64.get_cpu();
        let regs = RegData {
            a: cpu.get_a(),
            x: cpu.get_x(),
            y: cpu.get_y(),
            p: cpu.get_p(),
            sp: cpu.get_sp(),
            pc: cpu.get_pc(),
            port_00: cpu.read(0x00),
            port_01: cpu.read(0x01),
            clock,
        };
        CmdResult::ok(Output::Registers(regs))
    }

    fn reg_write(&self, c64: &mut C64, ops: &[RegOp]) -> Result<CmdResult, String> {
        let cpu = c64.get_cpu_mut();
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
        CmdResult::unit()
    }

    fn sys_reset(&self, c64: &mut C64, hard: bool) -> Result<CmdResult, String> {
        c64.reset(hard);
        CmdResult::unit()
    }

    fn sys_screen(&self, c64: &mut C64) -> Result<CmdResult, String> {
        let cia2 = c64.get_cia_2();
        let vic = c64.get_vic();
        let cia2_port_a = cia2.borrow_mut().read(0x00);
        let vm = (((vic.borrow_mut().read(0x18) & 0xf0) >> 4) as u16) << 10;
        let vm_base = ((!cia2_port_a & 0x03) as u16) << 14 | vm;
        CmdResult::ok(Output::Number(vm_base))
    }

    fn sys_stopwatch(&self, c64: &mut C64, reset: bool) -> Result<CmdResult, String> {
        let clock = c64.get_clock();
        if reset {
            clock.reset();
        }
        let mut buffer = Vec::new();
        buffer
            .write_u64::<BigEndian>(clock.get())
            .map_err(|_| "Op failed")?;
        CmdResult::ok(Output::Buffer(buffer))
    }
}
