use crate::logger::{LogLevel, Logger};
use crate::JsonValue;
use std::collections::HashMap;

pub struct ConsoleLogger;

impl ConsoleLogger {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsoleLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger for ConsoleLogger {
    fn info(&self, msg: &str, _context: &mut HashMap<&str, JsonValue>) {
        println!("[INFO] {msg}");
    }
    fn warning(&self, msg: &str, _context: &mut HashMap<&str, JsonValue>) {
        eprintln!("[WARN] {msg}");
    }
    fn error(&self, msg: &str, _context: &mut HashMap<&str, JsonValue>) {
        eprintln!("[ERROR] {msg}");
    }
    fn debug(&self, msg: &str, _context: &mut HashMap<&str, JsonValue>) {
        if cfg!(debug_assertions) {
            println!("[DEBUG] {msg}");
        }
    }
    fn is_enabled_for(&self, level: LogLevel) -> bool {
        !matches!(level, LogLevel::Debug) || cfg!(debug_assertions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn console_logger_info_enabled() {
        let l = ConsoleLogger;
        assert!(l.is_enabled_for(LogLevel::Info));
        let mut ctx = HashMap::new();
        l.info("hello", &mut ctx);
    }
}
