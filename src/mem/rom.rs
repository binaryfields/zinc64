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

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::result::Result;

use mem::Addressable;

pub struct Rom {
    data: Vec<u8>,
    offset: u16,
}

impl Rom {
    pub fn load(path: &Path, offset: u16) -> Result<Rom, io::Error> {
        let mut data = Vec::new();
        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut data)?;
        Ok(
            Rom {
                data: data,
                offset: offset,
            }
        )
    }
}

impl Addressable for Rom {
    fn read(&self, address: u16) -> u8 {
        self.data[(address - self.offset) as usize]
    }

    #[allow(unused_variables)]
    fn write(&mut self, address: u16, value: u8) {
        panic!("writes to rom are not supported")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use mem::Addressable;

    #[test]
    fn load_basic_rom() {
        let rom = Rom::load(&Path::new("rom/basic.rom"), 0x000).unwrap();
        assert_eq!(0x2000, rom.data.len());
    }

    #[test]
    fn load_charset_rom() {
        let rom = Rom::load(&Path::new("rom/characters.rom"), 0x000).unwrap();
        assert_eq!(0x1000, rom.data.len());
    }

    #[test]
    fn load_kernal_rom() {
        let rom = Rom::load(&Path::new("rom/kernal.rom"), 0x0000).unwrap();
        assert_eq!(0x2000, rom.data.len());
    }

    #[test]
    fn read_address() {
        let rom = Rom::load(&Path::new("rom/basic.rom"), 0x0000).unwrap();
        assert_eq!(0x94, rom.read(0x0000));
    }
}
