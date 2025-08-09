use core::fmt::{Error, Write};

use heapless::Vec;

pub struct Logger {
    pub(crate) logs_to_write: Vec<u8, 1024>,
}

impl Logger {
    pub(crate) fn new() -> Self {
        Logger {
            logs_to_write: Vec::new(),
        }
    }
    pub(crate) fn advance(&mut self, len: usize) {
        self.logs_to_write.rotate_left(len);
        self.logs_to_write.truncate(self.logs_to_write.len() - len);
    }
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.logs_to_write.extend_from_slice(s.as_bytes()) {
            Ok(()) => core::fmt::Result::Ok(()),
            Err(_) => core::fmt::Result::Err(Error),
        }
    }
}

#[macro_export]
macro_rules! trace {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = writeln!($logger, "TRACE: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = writeln!($logger, "DEBUG: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = writeln!($logger, "INFO: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warn {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = writeln!($logger, "WARN: {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! error {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = writeln!($logger, "ERROR: {}", format_args!($($arg)*));
    }};
}
