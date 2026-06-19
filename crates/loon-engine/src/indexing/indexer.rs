//! Fast keyword index for guidelines. Accepts a guideline and stores
//! its tokenized condition+action for later fuzzy lookup via `search`.

use async_trait::async_trait;
use parking_lot::RwLock;
use loon_core::Guideline;
use crate::error::EngineResult;

#[async_trait]
pub trait Indexer: Send + Sync {
    async fn index(&self, g: &Guideline) -> EngineResult<()>;
    async fn search(&self, query: &str, top_k: usize) -> EngineResult<Vec<Guideline>>;
}

fn tokenize(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() > 1)
        .collect()
}

pub struct KeywordIndexer {
    entries: RwLock<Vec<(Guideline, Vec<String>)>>,
}

impl KeywordIndexer {
    pub fn new() -> Self { Self { entries: RwLock::new(Vec::new()) } }
}
impl Default for KeywordIndexer { fn default() -> Self { Self::new() } }

#[async_trait]
impl Indexer for KeywordIndexer {
    async fn index(&self, g: &Guideline) -> EngineResult<()> {
        let tokens = tokenize(&format!("{} {}", g.content.condition, g.content.action));
        self.entries.write().push((g.clone(), tokens));
        Ok(())
    }
    async fn search(&self, query: &str, top_k: usize) -> EngineResult<Vec<Guideline>> {
        let qtokens = tokenize(query);
        if qtokens.is_empty() { return Ok(vec![]); }
        let entries = self.entries.read();
        let mut scored: Vec<(usize, &Guideline)> = entries.iter()
            .map(|(g, t)| (t.iter().filter(|w| qtokens.contains(w)).count(), g))
            .filter(|(s, _)| *s > 0)
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        Ok(scored.into_iter().take(top_k).map(|(_, g)| g.clone()).collect())
    }
}

pub struct NoopIndexer;
#[async_trait]
impl Indexer for NoopIndexer {
    async fn index(&self, _: &Guideline) -> EngineResult<()> { Ok(()) }
    async fn search(&self, _: &str, _: usize) -> EngineResult<Vec<Guideline>> { Ok(vec![]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, GuidelineContent, Criticality, GuidelineId};
    fn g(action: &str) -> Guideline {
        Guideline { id: GuidelineId::new(), agent_id: AgentId::new(), content: GuidelineContent { condition: "x".into(), action: action.into(), description: None }, criticality: Criticality::Low, enabled: true, tags: vec![], creation_utc: chrono::Utc::now(), metadata: loon_core::JsonValue::Null }
    }
    #[tokio::test]
    async fn index_and_search_finds_match() {
        let kw = KeywordIndexer::new();
        kw.index(&g("greet user warmly")).await.unwrap();
        kw.index(&g("transfer to billing")).await.unwrap();
        let hits = kw.search("greeting user", 5).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.action.contains("greet"));
    }
}
