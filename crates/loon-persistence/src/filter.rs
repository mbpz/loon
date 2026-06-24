use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// AST for document-level queries. Backends translate this into
/// their native query language.
///
/// # Example
///
/// ```
/// # use loon_persistence::DocumentFilter;
/// # use serde_json::json;
/// let f = DocumentFilter::And(vec![
///     DocumentFilter::Eq { field: "status".into(), value: json!("active") },
///     DocumentFilter::Eq { field: "priority".into(), value: json!(1) },
/// ]);
/// assert!(matches!(f, DocumentFilter::And(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
pub enum DocumentFilter {
    Eq {
        field: String,
        value: JsonValue,
    },
    In {
        field: String,
        values: Vec<JsonValue>,
    },
    And(Vec<DocumentFilter>),
    Or(Vec<DocumentFilter>),
    Not(Box<DocumentFilter>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn filter_round_trip_eq() {
        let f = DocumentFilter::Eq {
            field: "name".into(),
            value: json!("a"),
        };
        let s = serde_json::to_string(&f).unwrap();
        let back: DocumentFilter = serde_json::from_str(&s).unwrap();
        assert_eq!(f, back);
    }
    #[test]
    fn filter_and_composite() {
        let f = DocumentFilter::And(vec![
            DocumentFilter::Eq {
                field: "a".into(),
                value: json!(1),
            },
            DocumentFilter::Eq {
                field: "b".into(),
                value: json!(2),
            },
        ]);
        let s = serde_json::to_string(&f).unwrap();
        assert!(s.contains("\"And\""));
    }
}
