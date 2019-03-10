// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::fmt::Write;
use log::{LogLevelFilter, LogMetadata, SetLoggerError, ShutdownLoggerError};

pub struct SimpleLogger;

impl SimpleLogger {
    pub fn new() -> Self {
        SimpleLogger
    }

    pub fn init(&self) -> Result<(), SetLoggerError> {
        unsafe {
            log::set_logger_raw(move |max_log_level| {
                max_log_level.set(LogLevelFilter::Info);
                self.ptr()
            })
        }
    }

    fn flush(&self) {}

    fn ptr(&self) -> *const log::Log {
        &*self
    }
}

impl log::Log for SimpleLogger {
    fn enabled(&self, _: &LogMetadata) -> bool {
        false
    }
    fn log(&self, record: &log::LogRecord) {
        unsafe {
            crate::CONSOLE
                .write_fmt(format_args!(
                    "{} [{}] - {}\n",
                    record.level(),
                    record.target(),
                    record.args()
                ))
                .unwrap();
        }
    }
}

pub fn shutdown() -> Result<(), ShutdownLoggerError> {
    log::shutdown_logger_raw().map(|logger| {
        let logger = unsafe { &*(logger as *const SimpleLogger) };
        logger.flush();
    })
}
