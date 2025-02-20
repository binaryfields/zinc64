// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use zinc64_debug::{Executor, RunRequest};
use zinc64_engine::C64;
use zinc64_loader::Loaders;

const KEY_EVENT_CYCLES: u64 = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    Paused,
    Halted,
    Stopped,
}

impl From<RunRequest> for RunState {
    fn from(request: RunRequest) -> Self {
        match request {
            RunRequest::Halt => RunState::Halted,
            RunRequest::Resume => RunState::Running,
            RunRequest::Quit => RunState::Stopped,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum JamAction {
    Continue,
    Quit,
    Reset,
}

pub struct Session {
    c64: C64,
    debugger: Executor,
    jam_action: JamAction,
    state: RunState,
    next_key_event: u64,
}

impl Session {
    pub fn new(c64: C64, debugger: Executor, jam_action: JamAction) -> Self {
        Self {
            c64,
            debugger,
            jam_action,
            state: RunState::Running,
            next_key_event: 0,
        }
    }

    pub fn c64(&self) -> &C64 {
        &self.c64
    }

    pub fn c64_mut(&mut self) -> &mut C64 {
        &mut self.c64
    }

    pub fn state(&self) -> RunState {
        self.state
    }

    // ---- media ----

    pub fn open_image(&mut self, path: &Path) -> Result<(), String> {
        let ext = path.extension().map(|s| s.to_str().unwrap());
        let loader = Loaders::from_ext(ext).map_err(|err| err.to_string())?;
        let file = File::open(path).map_err(|err| err.to_string())?;
        let mut reader = BufReader::new(file);
        let autostart = loader
            .autostart(&mut reader)
            .map_err(|err| err.to_string())?;
        autostart.execute(&mut self.c64);
        Ok(())
    }

    // ---- run ----

    pub fn update(&mut self) -> Result<bool, String> {
        while let Some(command) = self.debugger.poll(self.state == RunState::Halted) {
            if let Ok(Some(request)) = self.debugger.execute(&mut self.c64, &command) {
                self.state = request.into();
            }
        }
        if self.state != RunState::Running {
            return Ok(false);
        }
        self.type_queued_key();
        if self.c64.run_frame() {
            self.handle_jam();
        } else {
            self.halt()?;
        }
        Ok(true)
    }

    fn type_queued_key(&mut self) {
        let keyboard_ready = self.c64.get_cycles() >= self.next_key_event;
        if keyboard_ready && self.c64.get_keyboard().has_events() {
            self.c64.get_keyboard().drain_event();
            self.next_key_event = self.c64.get_cycles().wrapping_add(KEY_EVENT_CYCLES);
        }
    }

    fn handle_jam(&mut self) {
        if !self.c64.is_cpu_jam() {
            return;
        }
        match self.jam_action {
            JamAction::Continue => {}
            JamAction::Quit => {
                warn!("CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
                self.state = RunState::Stopped;
            }
            JamAction::Reset => {
                warn!("CPU JAM detected at 0x{:x}", self.c64.get_cpu().get_pc());
                self.reset(false);
            }
        }
    }

    // ---- control ----

    pub fn halt(&mut self) -> Result<(), String> {
        self.state = RunState::Halted;
        self.debugger.halt()
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            RunState::Running => self.state = RunState::Paused,
            RunState::Paused => self.state = RunState::Running,
            _ => {}
        }
    }

    pub fn quit(&mut self) {
        self.state = RunState::Stopped;
    }

    pub fn reset(&mut self, hard: bool) {
        self.c64.reset(hard);
        self.next_key_event = 0;
    }

    pub fn toggle_tape(&mut self) {
        if !self.c64.is_tape_playing() {
            self.c64.play_tape();
        } else {
            self.c64.stop_tape();
        }
    }
}
