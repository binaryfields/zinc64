// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use crate::session::{RunState, Session};

pub fn run(session: &mut Session) -> Result<(), String> {
    while session.state() != RunState::Stopped {
        session.update()?;
        session.c64_mut().reset_vsync();
    }
    Ok(())
}
