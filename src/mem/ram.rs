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

use mem::Addressable;

pub struct Ram {
    data: Vec<u8>
}

impl Ram {
    pub fn new(capacity: usize) -> Ram {
        Ram {
            data: vec![0; capacity]
        }
    }
}

impl Addressable for Ram {
    fn read(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.data[address as usize] = value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mem::Addressable;

    #[test]
    fn new_with_capacity() {
        let ram = Ram::new(0x10000);
        assert_eq!(0x10000, ram.data.len());
    }

    #[test]
    fn read_address() {
        let ram = Ram::new(0x10000);
        assert_eq!(0, ram.read(0xffff));
    }

    #[test]
    fn write_address() {
        let mut ram = Ram::new(0x10000);
        ram.write(0x0001, 31);
        assert_eq!(31, ram.read(0x0001));
    }
}
