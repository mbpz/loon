//! Short-lived per-tool variable. Distinct from
//! [`loon_core::ContextVariable`] (which is a persisted entity); a
//! `Variable<T>` is just a named value passed between tools within
//! a single engine turn.
//!
//! Parlcant uses these for tool-output → tool-input chaining
//! ("first call returned X, pass X to the next tool"). Phase 1 is
//! storage-only — the engine doesn't yet thread variables through
//! the tool chain.

use serde::{Deserialize, Serialize};

/// A named transient value visible to tools and message composition
/// within one engine turn. `T` is whatever shape the producing tool
/// emits (commonly `serde_json::Value` for opaque blobs, but
/// stronger typing works too).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable<T> {
    pub name: String,
    pub value: T,
}

impl<T> Variable<T> {
    pub fn new(name: impl Into<String>, value: T) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_round_trips_serde() {
        let v = Variable::new("answer", 42i32);
        let s = serde_json::to_string(&v).unwrap();
        let back: Variable<i32> = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn variable_holds_json_blob() {
        let v = Variable::new("payload", serde_json::json!({"k": "v"}));
        assert_eq!(v.value["k"], "v");
    }
}
