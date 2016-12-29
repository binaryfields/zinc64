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

#[inline(always)]
pub fn bit_set(bit: u8, enabled: bool) -> u8 {
    if enabled { 1 << bit } else { 0 }
}

#[inline(always)]
pub fn bit_test(value: u8, bit: u8) -> bool {
    value & (1 << bit) != 0
}

#[inline(always)]
pub fn bit_val(value: u8, bit: u8) -> u8 {
    if (value & (1 << bit)) != 0 { 1 } else { 0 }
}

#[inline(always)]
pub fn bit_val16(value: u16, bit: u8) -> u8 {
    if (value & (1 << bit)) != 0 { 1 } else { 0 }
}

#[inline(always)]
pub fn bit_update(value: u8, bit: u8, enabled: bool) -> u8 {
    if enabled { value | (1 << bit) } else { value & !(1 << bit) }
}

#[inline(always)]
pub fn bit_update16(value: u16, bit: u8, enabled: bool) -> u16 {
    if enabled { value | ((1 << bit) as u16) } else { value & !((1 << bit) as u16) }
}

