// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![allow(clippy::cast_lossless)]

use std::sync::Arc;

use crate::board::{BaseAddr, Board};
use crate::breakpoint::BreakpointManager;
use crate::chip::cia::Cia;
use crate::chip::sid::Sid;
use crate::chip::vic::Vic;
use crate::chip::{Bus, Cpu6510, SoundOutput};
use crate::config::Config;
use crate::media::{Autostart, Media};
use crate::peripheral::{Cartridge, Joystick, Keyboard, Tape};

pub struct C64 {
    // Dependencies
    config: Config,
    // Chipset
    cpu: Cpu6510,
    board: Board,
    // Buffers
    sound_buffer: Arc<dyn SoundOutput>,
    // Runtime State
    autostart: Option<Autostart>,
    breakpoints: BreakpointManager,
    frame_count: u32,
    last_pc: u16,
    stopwatch_start: u64,
}

impl C64 {
    pub fn new(config: Config, sound_buffer: Arc<dyn SoundOutput>) -> C64 {
        info!(target: "c64", "Initializing system");
        let board = Board::new(&config, sound_buffer.clone());
        C64 {
            config,
            cpu: Cpu6510::new(),
            board,
            sound_buffer,
            autostart: None,
            breakpoints: BreakpointManager::default(),
            frame_count: 0,
            last_pc: 0,
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

    pub fn get_cia_1_mut(&mut self) -> &mut Cia {
        self.board.cia_1_mut()
    }

    pub fn get_cia_2(&self) -> &Cia {
        self.board.cia_2()
    }

    pub fn get_cia_2_mut(&mut self) -> &mut Cia {
        self.board.cia_2_mut()
    }

    pub fn get_sid(&self) -> &Sid {
        self.board.sid()
    }

    pub fn get_sid_mut(&mut self) -> &mut Sid {
        self.board.sid_mut()
    }

    pub fn get_vic(&self) -> &Vic {
        self.board.vic()
    }

    pub fn get_vic_mut(&mut self) -> &mut Vic {
        self.board.vic_mut()
    }

    pub fn get_joystick1(&self) -> &Option<Joystick> {
        self.board.joystick_1()
    }

    pub fn get_joystick1_mut(&mut self) -> &mut Option<Joystick> {
        self.board.joystick_1_mut()
    }

    pub fn get_joystick2(&self) -> &Option<Joystick> {
        self.board.joystick_2()
    }

    pub fn get_joystick2_mut(&mut self) -> &mut Option<Joystick> {
        self.board.joystick_2_mut()
    }

    pub fn get_keyboard(&mut self) -> &mut Keyboard {
        self.board.keyboard_mut()
    }

    pub fn get_bpm(&self) -> &BreakpointManager {
        &self.breakpoints
    }

    pub fn get_bpm_mut(&mut self) -> &mut BreakpointManager {
        &mut self.breakpoints
    }

    pub fn get_cycles(&self) -> u64 {
        self.board.cycles()
    }

    pub fn frame(&self) -> &[u8] {
        self.board.vic().frame()
    }

    pub fn get_frame_count(&self) -> u32 {
        self.frame_count
    }

    pub fn get_vsync(&self) -> bool {
        self.board.vsync()
    }

    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    // ---- reset ----
    pub fn reset(&mut self, hard: bool) {
        info!(target: "c64", "Resetting system");
        self.cpu.reset();
        self.board.reset(hard);
        // Buffers
        self.sound_buffer.reset();
        // Runtime State
        self.frame_count = 0;
        self.last_pc = 0;
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
    pub fn run_frame(&mut self) -> bool {
        let bp_present = self.breakpoints.is_bp_present();
        while !self.board.vsync() {
            self.step_internal();
            if bp_present && self.check_breakpoints() {
                break;
            }
        }
        if self.board.vsync() {
            self.board.process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
        self.board.vsync()
    }

    pub fn reset_vsync(&mut self) {
        self.board.reset_vsync();
    }

    pub fn step(&mut self) {
        self.step_internal();
        if self.board.vsync() {
            self.board.process_vsync();
            self.frame_count = self.frame_count.wrapping_add(1);
        }
    }

    pub fn clock(&mut self) {
        self.cpu.clock(&mut self.board);
        self.board.tick();
    }

    pub fn check_breakpoints(&mut self) -> bool {
        self.breakpoints.check(&self.cpu).is_some()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SystemModel;

    struct NullSound;
    impl SoundOutput for NullSound {
        fn reset(&self) {}
        fn write(&self, _samples: &[i16]) {}
    }

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
        let sound_output = Arc::new(NullSound {});
        let mut c64 = C64::new(config, sound_output);
        c64.reset(false);
        assert_eq!(0x94, c64.read(0xa000));
    }
}
