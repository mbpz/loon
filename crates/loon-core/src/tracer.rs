use crate::JsonValue;

pub trait Tracer: Send + Sync {
    fn trace_id(&self) -> &str;
    fn set_property(&self, key: &str, value: JsonValue);
    fn get_property(&self, key: &str) -> Option<JsonValue>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic_tracer::BasicTracer;
    #[test]
    fn tracer_trait_is_object_safe() {
        let _: Box<dyn Tracer> = Box::new(BasicTracer::new());
    }
}
