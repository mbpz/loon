use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{CustomerId, TagId, JsonValue};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Customer {
    pub id: CustomerId,
    pub name: String,
    pub metadata: JsonValue,
    pub tags: Vec<TagId>,
    pub creation_utc: DateTime<Utc>,
}

impl Customer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: CustomerId::new(),
            name: name.into(),
            metadata: JsonValue::Null,
            tags: vec![],
            creation_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CustomerUpdateParams {
    pub name: Option<String>,
    pub metadata: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn customer_default_has_empty_metadata() {
        let c = Customer::new("alice");
        assert_eq!(c.metadata, serde_json::Value::Null);
    }
}
