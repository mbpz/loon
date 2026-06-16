use crate::tracer::Tracer;
use crate::JsonValue;
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct BasicTracer {
    pub id: String,
    props: RwLock<HashMap<String, JsonValue>>,
}

impl BasicTracer {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            props: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for BasicTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer for BasicTracer {
    fn trace_id(&self) -> &str {
        &self.id
    }
    fn set_property(&self, k: &str, v: JsonValue) {
        self.props.write().insert(k.to_string(), v);
    }
    fn get_property(&self, k: &str) -> Option<JsonValue> {
        self.props.read().get(k).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_tracer_round_trip() {
        let t = BasicTracer::new();
        t.set_property("k", JsonValue::from(1));
        assert_eq!(t.get_property("k"), Some(JsonValue::from(1)));
        assert!(!t.trace_id().is_empty());
    }
}
