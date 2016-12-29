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

#[derive(Copy, Clone)]
pub struct Dimension {
    pub width: u16,
    pub height: u16,
}

impl Dimension {
    pub fn new(width: u16, height: u16) -> Dimension {
        Dimension {
            width: width,
            height: height,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Rect {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

impl Rect {
    pub fn new(left: u16, right: u16, top: u16, bottom: u16) -> Rect {
        Rect {
            left: left,
            right: right,
            top: top,
            bottom: bottom,
        }
    }

    pub fn new_with_dim(left: u16, top: u16, size: Dimension) -> Rect {
        Rect {
            left: left,
            right: left + size.width - 1,
            top: top,
            bottom: top + size.height - 1,
        }
    }

    pub fn size(&self) -> Dimension {
        Dimension::new(self.right - self.left + 1, self.bottom - self.top + 1)
    }
}
