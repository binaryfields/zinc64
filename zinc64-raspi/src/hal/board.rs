// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::mbox;

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum Clock {
    Uart = 0x00000002,
    Arm = 0x00000003,
}

pub fn get_max_clock_rate(mbox: &mut mbox::Mbox, clock: Clock) -> Result<u32, &'static str> {
    let mut data = [clock as u32, 0];
    mbox.property(mbox::Tag::GetMaxClockRate, &mut data)
        .map_err(|_| "failed to get max clock rate")?;
    if data[0] == clock as u32 {
        Ok(data[1])
    } else {
        Err("unable to get max clock rate")
    }
}

pub fn get_serial(mbox: &mut mbox::Mbox) -> Result<u64, &'static str> {
    let mut data = [0, 0];
    mbox.property(mbox::Tag::GetBoardSerial, &mut data)
        .map_err(|_| "unable to get board serial")?;
    let serial = (data[0] as u64) << 32 | (data[1] as u64);
    Ok(serial)
}

pub fn set_clock_speed(mbox: &mut mbox::Mbox, clock: Clock, hz: u32) -> Result<(), &'static str> {
    let mut data = [clock as u32, hz, 0];
    mbox.property(mbox::Tag::SetClockRate, &mut data)
        .map_err(|_| "failed to set clock speed")?;
    if data[0] == clock as u32 && data[1] == hz {
        Ok(())
    } else {
        Err("failed to set clock speed")
    }
}
