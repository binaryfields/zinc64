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

use std::sync::{Arc, Mutex};

use log::LogLevel;
use resid;

use super::SoundBuffer;

pub struct Sid {
    resid: resid::Sid,
    // I/O
    buffer: Arc<Mutex<SoundBuffer>>,
}

impl Sid {
    pub fn new(buffer: Arc<Mutex<SoundBuffer>>) -> Sid {
        info!(target: "sound", "Initializing SID");
        Sid {
            resid: resid::Sid::new(resid::ChipModel::Mos6581),
            buffer,
        }
    }

    pub fn enable_filter(&mut self, enabled: bool) {
        self.resid.enable_filter(enabled);
    }

    pub fn set_sampling_parameters(
        &mut self,
        method: resid::SamplingMethod,
        clock_freq: u32,
        sample_freq: u32,
    ) {
        self.resid.set_sampling_parameters(method, clock_freq, sample_freq);
    }

    #[inline(always)]
    pub fn clock_delta(&mut self, cycles: u32) {
        let mut buffer = [0i16; 4096]; // FIXME magic value
        let buffer_length = buffer.len();
        let mut samples = 0;
        let mut delta = cycles;
        while delta > 0 {
            let (read, next_delta) =
                self.resid
                    .sample(delta, &mut buffer[samples..], buffer_length - samples, 1);
            samples += read as usize;
            delta = next_delta;
        }
        // println!("SID cyc {} samples {}", cycles, samples);
        let mut output = self.buffer.lock().unwrap();
        for i in 0..samples {
            output.push(buffer[i]);
        }
    }

    pub fn reset(&mut self) {
        self.resid.reset();
    }

    // -- Device I/O

    pub fn read(&self, reg: u8) -> u8 {
        self.resid.read(reg)
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        if log_enabled!(LogLevel::Trace) {
            trace!(target: "sid::reg", "Write 0x{:02x} = 0x{:02x}", reg, value);
        }
        self.resid.write(reg, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    static SID_DATA: [u16; 51] = [
        25, 177, 250, 28, 214, 250,
        25, 177, 250, 25, 177, 250,
        25, 177, 125, 28, 214, 125,
        32, 94, 750, 25, 177, 250,
        28, 214, 250, 19, 63, 250,
        19, 63, 250, 19, 63, 250,
        21, 154, 63, 24, 63, 63,
        25, 177, 250, 24, 63, 125,
        19, 63, 250,
    ];

    fn setup_sid() -> Sid {
        let buffer = Arc::new(Mutex::new(SoundBuffer::new(8192)));
        let mut sid = Sid::new(buffer);
        sid.reset();
        sid
    }

    #[test]
    fn test_sid_output() {
        let mut sid = setup_sid();
        sid.write(0x05, 0x09); // AD1
        sid.write(0x06, 0x00); // SR1
        sid.write(0x18, 0x0f); // MODVOL
        let mut i = 0;
        let mut clocks = 0usize;
        while i < SID_DATA.len() {
            sid.write(0x01, SID_DATA[i + 0] as u8); // FREQHI1
            sid.write(0x00, SID_DATA[i + 1] as u8); // FREQLO1
            sid.write(0x00, 0x21); // CR1
            for _j in 0..SID_DATA[i + 2] {
                sid.clock_delta(22);
                clocks += 22;
            }
            sid.write(0x00, 0x20); // CR1
            for _j in 0..50 {
                sid.clock_delta(22);
                clocks += 22;
            }
            i += 3;
        }
        let buffer = sid.buffer.lock().unwrap();
        assert_eq!(clocks * 44100 / 985248, buffer.len());
    }
}