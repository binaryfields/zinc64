// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use cortex_a_semihosting::hprintln;
use log::{LogLevelFilter, SetLoggerError, ShutdownLoggerError, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, _: &LogMetadata) -> bool { false }
    fn log(&self, record: &log::LogRecord) {
        hprintln!(
                "{} [{}] - {}",
                record.level(),
                record.target(),
                record.args()
            ).unwrap();
    }
}

impl SimpleLogger {
    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    unsafe {
        log::set_logger_raw(|max_log_level| {
            static LOGGER: SimpleLogger = SimpleLogger;
            max_log_level.set(LogLevelFilter::Info);
            &LOGGER
        })
    }
}

pub fn shutdown() -> Result<(), ShutdownLoggerError> {
    log::shutdown_logger_raw().map(|logger| {
        let logger = unsafe { &*(logger as *const SimpleLogger) };
        logger.flush();
    })
}
