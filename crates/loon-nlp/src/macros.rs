pub trait JsonSchema {
    fn schema() -> serde_json::Value;
}

impl JsonSchema for String {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"string"})
    }
}
impl JsonSchema for i32 {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"integer"})
    }
}
impl JsonSchema for i64 {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"integer"})
    }
}
impl JsonSchema for f64 {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"number"})
    }
}
impl JsonSchema for f32 {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"number"})
    }
}
impl JsonSchema for bool {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"boolean"})
    }
}
impl<T: JsonSchema> JsonSchema for Vec<T> {
    fn schema() -> serde_json::Value {
        serde_json::json!({"type":"array", "items": <T as JsonSchema>::schema()})
    }
}
impl<T: JsonSchema> JsonSchema for Option<T> {
    fn schema() -> serde_json::Value {
        <T as JsonSchema>::schema()
    }
}

#[macro_export]
macro_rules! define_schematic {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident { $( $fvis:vis $field:ident : $fty:ty ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize)]
        $vis struct $name { $( $fvis $field : $fty ),* }

        impl $crate::Schematic for $name {
            fn schema() -> ::serde_json::Value {
                ::serde_json::json!({
                    "type": "object",
                    "properties": {
                        $( stringify!($field): <$fty as $crate::JsonSchema>::schema(), )*
                    },
                    "required": [ $( stringify!($field) ),* ],
                    "additionalProperties": false
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::super::Schematic;
    use super::JsonSchema;

    define_schematic! {
        pub struct TestS { pub a: String, pub b: i32 }
    }

    #[test]
    fn schema_has_required_fields() {
        let v = <TestS as Schematic>::schema();
        assert_eq!(v["type"], "object");
        assert_eq!(v["required"], serde_json::json!(["a", "b"]));
        assert_eq!(v["properties"]["a"]["type"], "string");
        assert_eq!(v["properties"]["b"]["type"], "integer");
        assert_eq!(v["additionalProperties"], false);
    }

    #[test]
    fn json_schema_built_ins() {
        assert_eq!(<String as JsonSchema>::schema(), serde_json::json!({"type":"string"}));
        assert_eq!(<i32 as JsonSchema>::schema(), serde_json::json!({"type":"integer"}));
        assert_eq!(<bool as JsonSchema>::schema(), serde_json::json!({"type":"boolean"}));
        assert_eq!(<Vec<f32> as JsonSchema>::schema(),
            serde_json::json!({"type":"array","items":{"type":"number"}}));
    }
}
