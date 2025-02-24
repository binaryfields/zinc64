// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use zinc64_debug::{Directive, Executor};
use zinc64_engine::{C64, Stop};
use zinc64_loader::Loaders;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    Paused,
    Halted,
    Stopped,
}

impl From<Directive> for RunState {
    fn from(directive: Directive) -> Self {
        match directive {
            Directive::Halt => RunState::Halted,
            Directive::Resume => RunState::Running,
            Directive::Quit => RunState::Stopped,
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
}

impl Session {
    pub fn new(c64: C64, debugger: Executor, jam_action: JamAction) -> Self {
        Self {
            c64,
            debugger,
            jam_action,
            state: RunState::Running,
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
            if let Ok(Some(directive)) = self.debugger.execute(&mut self.c64, &command) {
                self.state = directive.into();
            }
        }
        if self.state != RunState::Running {
            return Ok(false);
        }
        let stop = if self.debugger.has_breakpoints() {
            let debugger = &mut self.debugger;
            self.c64
                .run_frame_until(|cpu| debugger.check_breakpoint(cpu))
        } else {
            self.c64.run_frame()
        };
        match stop {
            Stop::Frame => self.handle_jam(),
            Stop::Breakpoint => self.halt()?,
        }
        Ok(true)
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
                self.c64.reset(false);
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
    }

    pub fn toggle_tape(&mut self) {
        if !self.c64.is_tape_playing() {
            self.c64.play_tape();
        } else {
            self.c64.stop_tape();
        }
    }
}
