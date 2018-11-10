// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::slice::Iter;

use core::Cpu;

use super::Condition;

pub struct Breakpoint {
    pub index: u16,
    pub address: u16,
    pub enabled: bool,
    pub condition: Option<Condition>,
    ignore: u16,
    autodelete: bool,
}

pub struct BreakpointManager {
    breakpoints: Vec<Breakpoint>,
    bp_index: u16,
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self {
            breakpoints: Vec::new(),
            bp_index: 1,
        }
    }
}

impl BreakpointManager {

    pub fn check(&mut self, cpu: &dyn Cpu) -> Option<usize> {
        if self.breakpoints.is_empty() {
            None
        } else {
            let pc = cpu.get_pc();
            let bp_pos = self.breakpoints.iter_mut().position(|bp| {
                if bp.address == pc && bp.enabled {
                    if bp.ignore == 0 {
                        bp.condition.as_ref().map_or(true, |cond| cond.eval(cpu))
                    } else {
                        bp.ignore -= 1;
                        false
                    }
                } else {
                    false
                }
            });
            if let Some(pos) = bp_pos {
                if self.breakpoints[pos].autodelete {
                    self.breakpoints.remove(pos);
                }
            }
            bp_pos
        }
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    pub fn enable_all(&mut self, enabled: bool) {
        for bp in self.breakpoints.iter_mut() {
            bp.enabled = enabled;
        }
    }

    pub fn get(&mut self, index: u16) -> Result<&Breakpoint, String> {
        match self.breakpoints.iter().position(|bp| bp.index == index) {
            Some(pos) => Ok(&self.breakpoints[pos]),
            None => Err(format!("Invalid index {}", index)),
        }
    }

    pub fn is_bp_present(&self) -> bool {
        self.breakpoints.iter().any(|bp| bp.enabled)
    }

    pub fn ignore(&mut self, index: u16, count: u16) -> Result<(), String> {
        match self.find_mut(index) {
            Some(bp) => {
                bp.ignore = count;
                Ok(())
            }
            None => Err(format!("Invalid index {}", index)),
        }
    }

    pub fn list(&self) -> Iter<Breakpoint> {
        self.breakpoints.iter()
    }

    pub fn remove(&mut self, index: u16) -> Result<(), String> {
        match self.breakpoints.iter().position(|bp| bp.index == index) {
            Some(pos) => {
                self.breakpoints.remove(pos);
                Ok(())
            }
            None => Err(format!("Invalid index {}", index)),
        }
    }

    pub fn set(&mut self, address: u16, autodelete: bool) -> u16 {
        let index = self.bp_index;
        let bp = Breakpoint {
            index,
            address,
            condition: None,
            enabled: true,
            ignore: 0,
            autodelete,
        };
        self.breakpoints.push(bp);
        self.bp_index += 1;
        index
    }

    pub fn set_condition(
        &mut self,
        index: u16,
        expr: &str,
        radix: Option<u32>,
    ) -> Result<(), String> {
        match self.find_mut(index) {
            Some(bp) => {
                let condition = Condition::parse(expr, radix)?;
                bp.condition = Some(condition);
                Ok(())
            }
            None => Err(format!("Invalid index {}", index)),
        }
    }

    pub fn set_enabled(&mut self, index: u16, enabled: bool) -> Result<(), String> {
        match self.find_mut(index) {
            Some(bp) => {
                bp.enabled = enabled;
                Ok(())
            }
            None => Err(format!("Invalid index {}", index)),
        }
    }

    fn find_mut(&mut self, index: u16) -> Option<&mut Breakpoint> {
        match self.breakpoints.iter().position(|bp| bp.index == index) {
            Some(pos) => Some(&mut self.breakpoints[pos]),
            None => None,
        }
    }
}
