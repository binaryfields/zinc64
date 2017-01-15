/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
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

use super::filter::Filter;
use super::voice::Voice;

// SPEC: http://www.oxyron.de/html/registers_sid.html

enum Reg {
    FREQLO1,
    FREQHI1,
    PWLO1,
    PWHI1,
    CR1,
    AD1,
    SR1,
    FREQLO2,
    FREQHI2,
    PWLO2,
    PWHI2,
    CR2,
    AD2,
    SR2,
    FREQLO3,
    FREQHI3,
    PWLO3,
    PWHI3,
    CR3,
    AD3,
    SR3,
    FCLO,
    FCHI,
    RESFILT,
    MODVOL,
    POTX,
    POTY,
    OSC3,
    ENV3,
}

impl Reg {
    pub fn from(reg: u8) -> Reg {
        match reg {
            0x00 => Reg::FREQLO1,
            0x01 => Reg::FREQHI1,
            0x02 => Reg::PWLO1,
            0x03 => Reg::PWHI1,
            0x04 => Reg::CR1,
            0x05 => Reg::AD1,
            0x06 => Reg::SR1,
            0x07 => Reg::FREQLO2,
            0x08 => Reg::FREQHI2,
            0x09 => Reg::PWLO2,
            0x0a => Reg::PWHI2,
            0x0b => Reg::CR2,
            0x0c => Reg::AD2,
            0x0d => Reg::SR2,
            0x0e => Reg::FREQLO3,
            0x0f => Reg::FREQHI3,
            0x10 => Reg::PWLO3,
            0x11 => Reg::PWHI3,
            0x12 => Reg::CR3,
            0x13 => Reg::AD3,
            0x14 => Reg::SR3,
            0x15 => Reg::FCLO,
            0x16 => Reg::FCHI,
            0x17 => Reg::RESFILT,
            0x18 => Reg::MODVOL,
            0x19 => Reg::POTX,
            0x1a => Reg::POTY,
            0x1b => Reg::OSC3,
            0x1c => Reg::ENV3,
            _ => panic!("invalid reg {}", reg),
        }
    }
}

pub struct Sid {
    voices: [Voice; 3],
    filter: Filter,

}

impl Sid {
    pub fn new() -> Sid {
        Sid {
            voices: [Voice::new(); 3],
            filter: Filter::new(),
        }
    }

    pub fn reset(&mut self) {
        self.voices[0].reset();
        self.voices[1].reset();
        self.voices[2].reset();
        self.filter.reset();
    }
    
    // -- Device I/O

    pub fn read(&mut self, reg: u8) -> u8 {
        match Reg::from(reg) {
            Reg::POTX => 0,
            Reg::POTY => 0,
            Reg::OSC3 => 0, // FIXME
            Reg::ENV3 => 0, // FIXME
            _ => 0,
        }
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        match Reg::from(reg) {
            Reg::FREQLO1 => {
                let value = (self.voices[0].wave.frequency & 0xff00) | (value as u16);
                self.voices[0].wave.frequency = value;
            },
            Reg::FREQHI1 => {
                let value = (self.voices[0].wave.frequency & 0x00ff) | ((value as u16) << 8);
                self.voices[0].wave.frequency = value;
            },
            Reg::PWLO1 => {
                let value = (self.voices[0].wave.pulse_width & 0xff00) | (value as u16);
                self.voices[0].wave.pulse_width = value;
            },
            Reg::PWHI1 => {
                let value = (self.voices[0].wave.pulse_width & 0x00ff) | ((value as u16) << 8);
                self.voices[0].wave.pulse_width = value;
            },
            Reg::CR1 => {
                self.voices[0].envelope.set_control(value);
                self.voices[0].wave.set_control(value);
            },
            Reg::AD1 => {
                self.voices[0].envelope.attack = (value & 0xf0) >> 4;
                self.voices[0].envelope.decay = value & 0x0f;
            },
            Reg::SR1 => {
                self.voices[0].envelope.sustain = (value & 0xf0) >> 4;
                self.voices[0].envelope.release = value & 0x0f;
            },
            Reg::FREQLO2 => {
                let value = (self.voices[1].wave.frequency & 0xff00) | (value as u16);
                self.voices[1].wave.frequency = value;
            },
            Reg::FREQHI2 => {
                let value = (self.voices[1].wave.frequency & 0x00ff) | ((value as u16) << 8);
                self.voices[1].wave.frequency = value;
            },
            Reg::PWLO2 => {
                let value = (self.voices[1].wave.pulse_width & 0xff00) | (value as u16);
                self.voices[1].wave.pulse_width = value;
            },
            Reg::PWHI2 => {
                let value = (self.voices[1].wave.pulse_width & 0x00ff) | ((value as u16) << 8);
                self.voices[1].wave.pulse_width = value;
            },
            Reg::CR2 => {
                self.voices[1].envelope.set_control(value);
                self.voices[1].wave.set_control(value);
            },
            Reg::AD2 => {
                self.voices[1].envelope.attack = (value & 0xf0) >> 4;
                self.voices[1].envelope.decay = value & 0x0f;
            },
            Reg::SR2 => {
                self.voices[1].envelope.sustain = (value & 0xf0) >> 4;
                self.voices[1].envelope.release = value & 0x0f;
            },
            Reg::FREQLO3 => {
                let value = (self.voices[2].wave.frequency & 0xff00) | (value as u16);
                self.voices[2].wave.frequency = value;
            },
            Reg::FREQHI3 => {
                let value = (self.voices[2].wave.frequency & 0x00ff) | ((value as u16) << 8);
                self.voices[2].wave.frequency = value;
            },
            Reg::PWLO3 => {
                let value = (self.voices[2].wave.pulse_width & 0xff00) | (value as u16);
                self.voices[2].wave.pulse_width = value;
            },
            Reg::PWHI3 => {
                let value = (self.voices[2].wave.pulse_width & 0x00ff) | ((value as u16) << 8);
                self.voices[2].wave.pulse_width = value;
            },
            Reg::CR3 => {
                self.voices[2].envelope.set_control(value);
                self.voices[2].wave.set_control(value);
            },
            Reg::AD3 => {
                self.voices[2].envelope.attack = (value & 0xf0) >> 4;
                self.voices[2].envelope.decay = value & 0x0f;
            },
            Reg::SR3 => {
                self.voices[2].envelope.sustain = (value & 0xf0) >> 4;
                self.voices[2].envelope.release = value & 0x0f;
            },
            Reg::FCLO => {
                // FIXME
                let value = (self.filter.fc & 0xff00) | (value as u16);
                self.filter.fc = value;
            },
            Reg::FCHI => {
                // FIXME
                let value = (self.filter.fc & 0x00ff) | ((value as u16) << 8);
                self.filter.fc = value;
            },
            Reg::RESFILT => {
                self.filter.resonance = (value & 0xf0) >> 4;
                self.filter.filt = value & 0x0f;
            },
            Reg::MODVOL => {
                self.filter.mode = (value & 0xf0) >> 4;
                self.filter.volume = value & 0x0f;
            }
            _ => {},
        }
    }
}