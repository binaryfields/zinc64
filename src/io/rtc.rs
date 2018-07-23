// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cmp::PartialEq;

pub struct Rtc {
    enabled: bool,
    hours: u8,
    minutes: u8,
    seconds: u8,
    tenth: u8,
    pm: bool,
}

impl Rtc {
    pub fn new() -> Self {
        Self {
            enabled: true,
            hours: 0,
            minutes: 0,
            seconds: 0,
            tenth: 0,
            pm: false,
        }
    }

    pub fn get_hours(&self) -> u8 {
        self.hours
    }

    pub fn get_minutes(&self) -> u8 {
        self.minutes
    }

    pub fn get_seconds(&self) -> u8 {
        self.seconds
    }

    pub fn get_tenth(&self) -> u8 {
        self.tenth
    }

    pub fn get_pm(&self) -> bool {
        self.pm
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_hours(&mut self, value: u8) {
        self.hours = value;
    }

    pub fn set_minutes(&mut self, value: u8) {
        self.minutes = value;
    }

    pub fn set_seconds(&mut self, value: u8) {
        self.seconds = value;
    }

    pub fn set_tenth(&mut self, value: u8) {
        self.tenth = value;
    }

    pub fn set_pm(&mut self, pm: bool) {
        self.pm = pm;
    }

    pub fn tick(&mut self) {
        if self.enabled {
            self.tenth += 1;
            if self.tenth == 10 {
                self.tenth = 0;
                self.seconds += 1;
                if self.seconds == 60 {
                    self.seconds = 0;
                    self.minutes += 1;
                    if self.minutes == 60 {
                        self.minutes = 0;
                        self.hours += 1;
                        if self.hours == 12 {
                            self.pm = !self.pm;
                        }
                        if self.hours == 13 {
                            self.hours = 1;
                        }
                    }
                }
            }
        }
    }
}

impl PartialEq for Rtc {
    fn eq(&self, other: &Rtc) -> bool {
        self.hours == other.hours
            && self.minutes == other.minutes
            && self.seconds == other.seconds
            && self.tenth == other.tenth
    }
}
