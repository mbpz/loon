use std::collections::HashMap;
use xxhash_rust::xxh3::xxh3_64;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::common::UniqueId;

pub struct IdGenerator {
    cache: HashMap<String, u64>,
    counter: u64,
}

impl IdGenerator {
    pub fn new() -> Self { Self { cache: HashMap::new(), counter: 0 } }
    pub fn generate(&mut self, content_checksum: &str) -> UniqueId {
        let hash = xxh3_64(content_checksum.as_bytes());
        if let Some(existing) = self.cache.get(content_checksum) {
            return UniqueId(Self::encode(*existing));
        }
        self.cache.insert(content_checksum.to_string(), hash);
        UniqueId(Self::encode(hash))
    }
    pub fn generate_random() -> UniqueId { UniqueId::new() }
    fn encode(hash: u64) -> String {
        URL_SAFE_NO_PAD.encode(hash.to_le_bytes()).chars().take(10).collect()
    }
}
impl Default for IdGenerator { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_id_is_stable() {
        let mut gen = IdGenerator::new();
        let a = gen.generate("hello world");
        let b = gen.generate("hello world");
        assert_eq!(a, b);
        assert_eq!(a.as_str().len(), 10);
    }
    #[test]
    fn different_content_yields_different_id() {
        let mut gen = IdGenerator::new();
        let a = gen.generate("alpha");
        let b = gen.generate("beta");
        assert_ne!(a, b);
    }
    #[test]
    fn random_id_is_unique() {
        let a = IdGenerator::generate_random();
        let b = IdGenerator::generate_random();
        assert_ne!(a, b);
    }
}
