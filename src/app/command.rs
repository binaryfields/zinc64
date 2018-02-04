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

#[allow(dead_code)]
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
    SysIo(u16),
    SysQuit,
    SysReset(bool),
    SysScreen,
    SysStopwatch(bool),
}

#[allow(dead_code)]
pub enum CommandResult {
    Await,
    Buffer(Vec<u8>),
    Error(String),
    Number(u16),
    Registers(RegData),
    Text(String),
    Unit,
}

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
