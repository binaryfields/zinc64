// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[cfg(not(feature = "std"))]
use alloc::rc::Rc;
use core::cell::{Cell, RefCell};
#[cfg(feature = "std")]
use std::rc::Rc;

pub type Shared<T> = Rc<RefCell<T>>;
pub type SharedCell<T> = Rc<Cell<T>>;

pub fn new_shared<T>(value: T) -> Shared<T> {
    Rc::new(RefCell::new(value))
}

pub fn new_shared_cell<T>(value: T) -> SharedCell<T> {
    Rc::new(Cell::new(value))
}
