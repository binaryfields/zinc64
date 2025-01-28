// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: https://github.com/raspberrypi/linux/blob/rpi-3.12.y/drivers/video/bcm2708_fb.c

use core::mem;
use core::ptr;
use core::result::Result;
use core::sync::atomic::{compiler_fence, Ordering};

use super::mbox;
use crate::util::geo::Rect;

#[allow(unused)]
pub struct FrameBuffer {
    width: u32,
    height: u32,
    virtual_width: u32,
    virtual_height: u32,
    depth: u32,
    pitch: u32,
    base: usize,
}

impl FrameBuffer {
    pub fn build(
        mbox: &mut mbox::Mbox,
        physical_size: (u32, u32),
        virtual_size: (u32, u32),
        virtual_offset: (u32, u32),
        depth: u32,
    ) -> Result<FrameBuffer, &'static str> {
        let buf = &mut mbox.buffer;
        buf[0] = 35 * mem::size_of::<u32>() as u32;
        buf[1] = mbox::Code::Request as u32;

        buf[2] = mbox::Tag::SetPhysicalWidthHeight as u32;
        buf[3] = 8;
        buf[4] = 8;
        buf[5] = physical_size.0;
        buf[6] = physical_size.1;

        buf[7] = mbox::Tag::SetVirtualWidthHeight as u32;
        buf[8] = 8;
        buf[9] = 8;
        buf[10] = virtual_size.0;
        buf[11] = virtual_size.1;

        buf[12] = mbox::Tag::SetVirtualOffset as u32;
        buf[13] = 8;
        buf[14] = 8;
        buf[15] = virtual_offset.0;
        buf[16] = virtual_offset.1;

        buf[17] = mbox::Tag::SetDepth as u32;
        buf[18] = 4;
        buf[19] = 4;
        buf[20] = depth;

        buf[21] = mbox::Tag::SetPixelOrder as u32;
        buf[22] = 4;
        buf[23] = 4;
        buf[24] = 1; // rgb

        buf[25] = mbox::Tag::SetAllocateBuffer as u32;
        buf[26] = 8;
        buf[27] = 8;
        buf[28] = 4096;
        buf[29] = 0;

        buf[30] = mbox::Tag::GetPitch as u32;
        buf[31] = 4;
        buf[32] = 4;
        buf[33] = 0;

        buf[34] = mbox::Tag::PropertyEnd as u32;

        compiler_fence(Ordering::Release);
        mbox.call2(mbox::Channel::Property)
            .map_err(|_| "unable to set screen resolution")?;

        if mbox.buffer[20] == depth && mbox.buffer[28] != 0 {
            Ok(FrameBuffer {
                width: mbox.buffer[5],
                height: mbox.buffer[6],
                virtual_width: mbox.buffer[10],
                virtual_height: mbox.buffer[11],
                depth: mbox.buffer[20],
                pitch: mbox.buffer[33],
                base: (mbox.buffer[28] & 0x3FFFFFFF) as usize,
            })
        } else {
            Err("failed to allocate frame buffer")
        }
    }

    pub fn as_ptr(&self) -> *mut u32 {
        self.base as *mut u32
    }

    pub fn blit(&self, src: &[u32], src_rect: &Rect, src_pitch: u32) {
        let mut dst = self.as_ptr();
        for y in src_rect.y..(src_rect.y + src_rect.h) {
            let index = y as usize * (src_pitch / 4) as usize + src_rect.x as usize;
            unsafe {
                ptr::copy_nonoverlapping(&src[index], dst, src_rect.w as usize);
                dst = dst.offset((self.pitch / 4) as isize);
            }
        }
    }

    #[allow(unused)]
    pub fn blit_slow(&self, src: &[u32], src_rect: &Rect, src_pitch: u32) {
        let mut dst = self.as_ptr();
        for y in src_rect.y..(src_rect.y + src_rect.h) {
            let start_dst = dst;
            for x in src_rect.x..(src_rect.x + src_rect.w) {
                let index = y as usize * (src_pitch / 4) as usize + x as usize;
                unsafe {
                    dst.write(src[index]);
                    dst = dst.offset(1);
                }
            }
            unsafe { dst = start_dst.offset((self.pitch / 4) as isize) }
        }
    }

    #[allow(unused)]
    pub fn set_virtual_offset(
        &self,
        mbox: &mut mbox::Mbox,
        x: u32,
        y: u32,
    ) -> Result<(), &'static str> {
        let mut data = [x, y];
        mbox.property(mbox::Tag::SetVirtualOffset, &mut data)
            .map_err(|_| "unable to set virtual offset")?;
        if data[0] == x && data[1] == y {
            Ok(())
        } else {
            Err("unable to set virtual offset")
        }
    }
}
