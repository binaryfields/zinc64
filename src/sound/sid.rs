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
        let mut sid = Sid {
            resid: resid::Sid::new(resid::ChipModel::Mos6581),
            buffer,
        };
        sid.resid
            .set_sampling_parameters(resid::SamplingMethod::ResampleFast, 985248, 44100);
        sid
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
        self.resid.write(reg, value);
    }
}
