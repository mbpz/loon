pub trait Schematic:
    serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static
{
    fn schema() -> serde_json::Value;
}
