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

use std::slice::Iter;

pub struct Breakpoint {
    pub index: u16,
    pub address: u16,
    pub enabled: bool,
    pub ignore: u16,
    pub autodelete: bool,
}

pub struct BreakpointManager {
    breakpoints: Vec<Breakpoint>,
    bp_index: u16,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            breakpoints: Vec::new(),
            bp_index: 0,
        }
    }

    pub fn add(&mut self, address: u16, autodelete: bool) -> u16 {
        let index = self.bp_index;
        let bp = Breakpoint {
            index,
            address,
            enabled: true,
            ignore: 0,
            autodelete,
        };
        self.breakpoints.push(bp);
        self.bp_index += 1;
        index
    }

    #[inline]
    pub fn check(&mut self, pc: u16) -> bool {
        if self.breakpoints.is_empty() {
            false
        } else {
            let mut index = None;
            let mut autodelete = false;
            for bp in self.breakpoints.iter_mut() {
                if bp.address == pc && bp.enabled {
                    if bp.ignore > 0 {
                        bp.ignore -= 1;
                    } else {
                        index = Some(bp.index);
                        autodelete = bp.autodelete;
                        break;
                    }
                }
            }
            if autodelete {
                self.remove(index);
            }
            index.is_some()
        }
    }

    pub fn enable(&mut self, index: Option<u16>, enabled: bool) -> bool {
        match index {
            Some(index) => {
                match self.breakpoints.iter().position(|bp| bp.index == index) {
                    Some(pos) => {
                        self.breakpoints[pos].enabled = enabled;
                        true
                    },
                    None => false,
                }
            }
            None => {
                for bp in self.breakpoints.iter_mut() {
                    bp.enabled = enabled;
                }
                true
            }
        }
    }

    pub fn ignore(&mut self, index: u16, count: u16) -> bool {
        match self.breakpoints.iter().position(|bp| bp.index == index) {
            Some(pos) => {
                self.breakpoints[pos].ignore = count;
                true
            },
            None => false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.breakpoints.iter()
            .position(|bp| bp.enabled)
            .is_some()
    }

    pub fn list(&self) -> Iter<Breakpoint> {
        self.breakpoints.iter()
    }

    pub fn remove(&mut self, index: Option<u16>) -> bool {
        match index {
            Some(index) => {
                match self.breakpoints.iter().position(|bp| bp.index == index) {
                    Some(pos) => {
                        self.breakpoints.remove(pos);
                        true
                    },
                    None => false,
                }
            }
            None => {
                self.breakpoints.clear();
                true
            }
        }
    }
}
