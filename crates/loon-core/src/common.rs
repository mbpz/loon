use serde::{Deserialize, Serialize};

pub type JsonValue = serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueId(pub String);

impl UniqueId {
    pub fn new() -> Self {
        Self(nanoid::nanoid!(10))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Default for UniqueId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UniqueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Criticality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    pub offset: usize,
    pub limit: usize,
}
impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_serializes_to_string() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, r#"{"major":1,"minor":2,"patch":3}"#);
    }

    #[test]
    fn unique_id_round_trip() {
        let id = UniqueId("abc123".into());
        let s = serde_json::to_string(&id).unwrap();
        let back: UniqueId = serde_json::from_str(&s).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn pagination_default() {
        let p = Pagination::default();
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 50);
    }
}
