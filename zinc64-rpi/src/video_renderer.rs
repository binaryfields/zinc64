// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use zinc64_core::Shared;

use crate::device::frame_buffer::FrameBuffer;
use crate::device::mbox::Mbox;
use crate::util::geo::Rect;
use crate::video_buffer::VideoBuffer;

const FB_SIZE: (u32, u32) = (640, 480);
const FB_BPP: u32 = 32;

pub struct VideoRenderer {
    // Configuration
    viewport_rect: Rect,
    // Resources
    frame_buffer: FrameBuffer,
    video_buffer: Shared<VideoBuffer>,
}

impl VideoRenderer {
    pub fn build(
        mbox: &mut Mbox,
        video_buffer: Shared<VideoBuffer>,
        viewport_offset: (u32, u32),
        viewport_size: (u32, u32),
    ) -> Result<VideoRenderer, &'static str> {
        let viewport_rect = Rect::new_with_origin(viewport_offset, viewport_size);
        let frame_buffer = FrameBuffer::build(
            mbox,
            FB_SIZE,
            (viewport_size.0, viewport_size.1),
            (0, 0),
            FB_BPP,
        )?;
        Ok(VideoRenderer {
            viewport_rect,
            frame_buffer,
            video_buffer,
        })
    }

    pub fn render(&mut self) -> Result<(), &'static str> {
        self.frame_buffer.blit(
            self.video_buffer.borrow().get_data(),
            &self.viewport_rect,
            self.video_buffer.borrow().get_pitch() as u32,
        );
        Ok(())
    }
}
