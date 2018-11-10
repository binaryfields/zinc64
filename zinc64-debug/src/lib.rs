// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

extern crate bit_field;
extern crate byteorder;
#[macro_use]
extern crate log;
extern crate zinc64;

mod charset;
mod debugger;
mod disassembler;
mod rap_server;

use std::sync::mpsc::Sender;

pub use self::debugger::Debugger;
pub use self::rap_server::RapServer;

pub enum Command {
    Attach(Sender<CommandResult>),
    Detach,
    // Breakpoint
    BpClear,
    BpCondition(u16, String, u32),
    BpDisable(u16),
    BpDisableAll,
    BpEnable(u16),
    BpEnableAll,
    BpIgnore(u16, u16),
    BpList,
    BpRemove(u16),
    BpSet(u16, bool),
    // Debugger
    Continue,
    RegRead,
    RegWrite(Vec<RegOp>),
    Step,
    // Memory
    MemRead(u16, u16),
    MemWrite(u16, Vec<u8>),
    // System
    SysQuit,
    SysReset(bool),
    SysScreen,
    SysStopwatch(bool),
}

pub enum CommandResult {
    Await,
    Buffer(Vec<u8>),
    Error(String),
    Number(u16),
    Registers(RegData),
    Text(String),
    Unit,
}

#[derive(Clone, Copy)]
pub struct RegData {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub sp: u8,
    pub pc: u16,
    pub port_00: u8,
    pub port_01: u8,
}

pub enum RegOp {
    SetA(u8),
    SetX(u8),
    SetY(u8),
    SetP(u8),
    SetSP(u8),
    SetPC(u16),
}
