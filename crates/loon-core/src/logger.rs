use crate::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

pub trait Logger: Send + Sync {
    fn info(&self, msg: &str, context: &mut HashMap<&str, JsonValue>);
    fn warning(&self, msg: &str, context: &mut HashMap<&str, JsonValue>);
    fn error(&self, msg: &str, context: &mut HashMap<&str, JsonValue>);
    fn debug(&self, msg: &str, context: &mut HashMap<&str, JsonValue>);
    fn is_enabled_for(&self, level: LogLevel) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logger_trait_is_object_safe() {
        let _: Box<dyn Logger> = Box::new(crate::console_logger::ConsoleLogger);
    }
}
