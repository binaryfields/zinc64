// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::time::{Duration, Instant};

pub struct Time {
    elapsed: Duration,
    last_time: Instant,
    timer: Option<Timer>,
}

impl Time {
    pub fn new(fps: Option<f64>) -> Self {
        let timer = fps.map(|v| Timer::new(1.0 / v));
        Self {
            elapsed: Duration::from_secs(0),
            last_time: Instant::now(),
            timer,
        }
    }

    pub fn set_fps(&mut self, fps: Option<f64>) {
        self.timer = fps.map(|v| Timer::new(1.0 / v));
    }

    #[allow(unused)]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn has_timer_event(&mut self) -> bool {
        self.timer.as_mut().map(|t| t.has_event()).unwrap_or(true)
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.elapsed = now - self.last_time;
        self.last_time = now;
        if let Some(timer) = self.timer.as_mut() {
            timer.accumulator += self.elapsed.as_secs_f64()
        }
    }
}

struct Timer {
    accumulator: f64,
    interval: f64,
}

impl Timer {
    pub fn new(interval: f64) -> Self {
        Self {
            accumulator: 0.0,
            interval,
        }
    }

    pub fn has_event(&mut self) -> bool {
        if self.accumulator >= self.interval {
            self.accumulator -= self.interval;
            true
        } else {
            false
        }
    }
}
