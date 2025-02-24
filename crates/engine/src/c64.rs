// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::cast_lossless)]

use crate::board::{BaseAddr, Board};
use crate::chip::cia::Cia;
use crate::chip::sid::Sid;
use crate::chip::vic::Vic;
use crate::chip::{Bus, Cpu6510};
use crate::config::Config;
use crate::media::{Autostart, Media};
use crate::peripheral::{Cartridge, Joystick, Keyboard, Tape};

const KEY_EVENT_CYCLES: u64 = 20_000;

pub struct C64 {
    // Dependencies
    config: Config,
    // Chipset
    cpu: Cpu6510,
    board: Board,
    // Runtime State
    autostart: Option<Autostart>,
    frame_count: u32,
    last_pc: u16,
    next_key_event: u64,
    stopwatch_start: u64,
}

impl C64 {
    pub fn new(config: Config) -> C64 {
        info!(target: "c64", "Initializing system");
        let board = Board::new(&config);
        C64 {
            config,
            cpu: Cpu6510::new(),
            board,
            autostart: None,
            frame_count: 0,
            last_pc: 0,
            next_key_event: 0,
            stopwatch_start: 0,
        }
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_cpu(&self) -> &Cpu6510 {
        &self.cpu
    }

    pub fn get_cpu_mut(&mut self) -> &mut Cpu6510 {
        &mut self.cpu
    }

    pub fn is_cpu_jam(&self) -> bool {
        self.last_pc == self.cpu.get_pc()
    }

    pub fn get_cia_1(&self) -> &Cia {
        self.board.cia_1()
    }

    pub fn get_cia_2(&self) -> &Cia {
        self.board.cia_2()
    }

    pub fn get_sid(&self) -> &Sid {
        self.board.sid()
    }

    pub fn get_vic(&self) -> &Vic {
        self.board.vic()
    }

    pub fn get_joystick1(&self) -> &Joystick {
        self.board.joystick_1()
    }

    pub fn get_joystick1_mut(&mut self) -> &mut Joystick {
        self.board.joystick_1_mut()
    }

    pub fn get_joystick2(&self) -> &Joystick {
        self.board.joystick_2()
    }

    pub fn get_joystick2_mut(&mut self) -> &mut Joystick {
        self.board.joystick_2_mut()
    }

    pub fn get_keyboard(&mut self) -> &mut Keyboard {
        self.board.keyboard_mut()
    }

    pub fn get_cycles(&self) -> u64 {
        self.board.cycles()
    }

    pub fn frame(&self) -> &[u8] {
        self.board.vic().frame()
    }

    pub fn samples(&self) -> &[i16] {
        self.board.sid().samples()
    }

    pub fn get_frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    // ---- reset ----
    pub fn reset(&mut self, hard: bool) {
        info!(target: "c64", "Resetting system");
        self.cpu.reset();
        self.board.reset(hard);
        // Runtime State
        self.frame_count = 0;
        self.last_pc = 0;
        self.next_key_event = 0;
        self.stopwatch_start = 0;
    }

    // ---- stopwatch ----
    pub fn stopwatch(&self) -> u64 {
        self.board.cycles() - self.stopwatch_start
    }

    pub fn reset_stopwatch(&mut self) {
        self.stopwatch_start = self.board.cycles();
    }

    // ---- memory ----
    pub fn read(&mut self, address: u16) -> u8 {
        self.board.read(address)
    }

    pub fn peek(&self, address: u16) -> u8 {
        self.board.peek(address)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.board.write(address, value);
    }

    // ---- media ----
    pub fn load(&mut self, data: &[u8], offset: u16) {
        self.board.load(data, offset);
    }

    pub fn mount(&mut self, media: Media) {
        match media {
            Media::Program { data, offset } => {
                info!(target: "c64", "Mounting program at 0x{:04x}", offset);
                self.load(&data, offset);
            }
            Media::Binary { data, offset } => {
                info!(target: "c64", "Mounting binary at 0x{:04x}", offset);
                self.write(0x0001, 0);
                self.load(&data, offset);
                self.cpu.set_pc(offset);
            }
            Media::Cartridge(cartridge) => {
                info!(target: "c64", "Mounting cartridge");
                self.attach_cartridge(*cartridge);
            }
            Media::Tape(tape) => {
                info!(target: "c64", "Mounting tape");
                self.attach_tape(tape);
            }
        }
    }

    pub fn attach_cartridge(&mut self, cartridge: Cartridge) {
        self.board.attach_cartridge(cartridge);
    }

    pub fn attach_tape(&mut self, tape: Box<dyn Tape>) {
        self.board.attach_tape(tape);
    }

    pub fn detach_cartridge(&mut self) {
        self.board.detach_cartridge();
        self.reset(false);
    }

    pub fn detach_tape(&mut self) {
        self.board.detach_tape();
    }

    pub fn is_tape_playing(&self) -> bool {
        self.board.is_tape_playing()
    }

    pub fn play_tape(&mut self) {
        self.board.play_tape();
    }

    pub fn stop_tape(&mut self) {
        self.board.stop_tape();
    }

    // ---- execution ----
    pub fn run_frame(&mut self) -> StopReason {
        self.run_frame_until(|_| false)
    }

    pub fn run_frame_until(&mut self, mut stop: impl FnMut(&Cpu6510) -> bool) -> StopReason {
        self.board.begin_run();
        self.type_queued_key();
        while !self.board.vsync() {
            self.step_internal();
            if stop(&self.cpu) {
                break;
            }
        }
        if self.end_frame() {
            StopReason::Frame
        } else {
            StopReason::Breakpoint
        }
    }

    pub fn step(&mut self) {
        self.board.begin_run();
        self.type_queued_key();
        self.step_internal();
        self.end_frame();
    }

    pub fn clock(&mut self) {
        self.cpu.clock(&mut self.board);
        self.board.tick();
    }

    fn end_frame(&mut self) -> bool {
        if self.board.vsync() {
            self.board.process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        self.board.vsync()
    }

    fn type_queued_key(&mut self) {
        let cycles = self.board.cycles();
        let keyboard = self.board.keyboard_mut();
        if keyboard.has_events() && cycles >= self.next_key_event {
            keyboard.drain_event();
            self.next_key_event = cycles.wrapping_add(KEY_EVENT_CYCLES);
        }
    }

    #[inline]
    fn step_internal(&mut self) {
        self.last_pc = self.cpu.get_pc();
        loop {
            self.clock();
            if self.cpu.is_opcode_fetched() {
                break;
            }
        }
        if self.autostart.is_some()
            && self.cpu.get_pc() == (BaseAddr::BootComplete.addr())
            && let Some(autostart) = self.autostart.take()
        {
            autostart.execute(self);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Frame,
    Breakpoint,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SystemModel;

    #[test]
    fn verify_mem_layout() {
        let mut basic = vec![0x00; 0x2000];
        basic[0] = 0x94;
        let config = Config::new_with_roms(
            SystemModel::from("pal"),
            &basic,
            &[0x00; 0x1000],
            &[0x00; 0x2000],
        );
        let mut c64 = C64::new(config);
        c64.reset(false);
        assert_eq!(0x94, c64.read(0xa000));
    }
}
