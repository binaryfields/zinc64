/*
 * Copyright (c) 2016 DigitalStream <https://www.digitalstream.io>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// http://sta.c64.org/cbm64kbdlay.html
// http://unusedino.de/ec64/technical/aay/c64/keybmatr.htm

// TODO keyboard: add scancode mapping

pub struct Keyboard {
    matrix: [u8; 8],
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard {
            matrix: [0xff; 8]
        }
    }

    pub fn get_row(&self, row: u8) -> u8 {
        self.matrix[row as usize]
    }

    pub fn set_row(&mut self, row: u8, value: u8) {
        self.matrix[row as usize] = value;
    }
}
