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

pub struct CpuState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub p: u8,
    pub pc: u16,
    pub port_00: u8,
    pub port_01: u8,
}

pub struct InstructionInfo {
    pub opcode: u8,
    pub instr_bytes: Vec<u8>,
    pub instr_text: String,
}

pub struct SystemState {
    pub pc: u16,
}

#[allow(dead_code)]
pub enum Command {
    Attach(Sender<Response>),
    Detach,
    Break,
    // Machine
    Goto(u16),
    Io(u16),
    Next(u16),
    Registers,
    Reset(bool),
    Return,
    Screen,
    Step(u16),
    Stopwatch(bool),
    // Memory
    Disassemble(Option<u16>, Option<u16>),
    Compare(u16, u16, u16),
    Fill(u16, u16, Vec<u8>),
    Hunt(u16, u16, Vec<u8>),
    MemChar(u16),
    MemPetscii(u16, Option<u16>),
    Move(u16, u16, u16),
    Read(u16, Option<u16>),
    Write(u16, Vec<u8>),
    // Monitor
    Exit,
    Quit,
    Radix(Option<u16>),
    // Breakpoints
    BpDelete(Option<u16>),
    BpDisable(Option<u16>),
    BpEnable(Option<u16>),
    BpIgnore(u16, u16),
    BpList,
    BpSet(u16),
    BpUntil(u16),
}

#[allow(dead_code)]
pub enum CommandResult {
    Empty,
    Error(String),
    Deferred,
    ExecutionState(CpuState, InstructionInfo),
    Memory(u16, Vec<u8>),
    Registers(CpuState),
    Text(Vec<String>),
}

pub struct Response(pub CommandResult, pub SystemState);
