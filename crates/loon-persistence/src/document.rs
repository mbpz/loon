use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::hash::Hash;
use crate::error::PersistenceResult;
use crate::filter::DocumentFilter;

pub type BaseDocument = JsonValue;
pub type DocumentLoader<T> = std::sync::Arc<dyn Fn(&BaseDocument) -> Option<T> + Send + Sync>;

pub trait Document: Serialize + DeserializeOwned + Send + Sync + 'static {
    const VERSION: &'static str;
    type Id: Serialize + DeserializeOwned + Send + Sync + Eq + Hash;
    fn id(&self) -> &Self::Id;
    fn to_base_document(&self) -> Result<BaseDocument, crate::error::PersistenceError> {
        Ok(serde_json::to_value(self)?)
    }
}

pub struct InsertResult {
    pub id: String,
}
pub struct UpdateResult {
    pub matched: u64,
    pub modified: u64,
}
pub struct DeleteResult {
    pub deleted: u64,
}

#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub enum DocumentUpdate {
    Set { field: String, value: JsonValue },
    Inc { field: String, by: i64 },
}

#[async_trait]
pub trait DocumentDatabase: Send + Sync {
    async fn get_or_create_collection<TDocument: Document>(
        &self,
        name: &str,
        schema: JsonValue,
        loader: DocumentLoader<TDocument>,
    ) -> PersistenceResult<Box<dyn DocumentCollection<TDocument>>>;
}

#[async_trait]
pub trait DocumentCollection<TDocument: Document>: Send + Sync {
    async fn insert_one(&self, document: TDocument) -> PersistenceResult<InsertResult>;
    async fn find_one(&self, filters: &DocumentFilter) -> PersistenceResult<Option<TDocument>>;
    async fn find(&self, filters: &DocumentFilter) -> PersistenceResult<Vec<TDocument>>;
    async fn update_one(
        &self,
        filters: &DocumentFilter,
        update: DocumentUpdate,
    ) -> PersistenceResult<UpdateResult>;
    async fn delete_one(&self, filters: &DocumentFilter) -> PersistenceResult<DeleteResult>;
    async fn count(&self, filters: &DocumentFilter) -> PersistenceResult<u64>;
    async fn find_paginated(
        &self,
        filter: &DocumentFilter,
        offset: usize,
        limit: usize,
    ) -> PersistenceResult<PaginatedResult<TDocument>> {
        let all = self.find(filter).await?;
        let total = all.len();
        let items: Vec<_> = all.into_iter().skip(offset).take(limit).collect();
        Ok(PaginatedResult {
            items,
            total,
            offset,
            limit,
        })
    }
    async fn find_sorted(
        &self,
        filter: &DocumentFilter,
        sort_by: &str,
        ascending: bool,
    ) -> PersistenceResult<Vec<TDocument>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestDoc {
        id: String,
        name: String,
    }
    impl Document for TestDoc {
        const VERSION: &'static str = "0.1.0";
        type Id = String;
        fn id(&self) -> &Self::Id {
            &self.id
        }
    }
    #[test]
    fn document_serializes_to_base() {
        let d = TestDoc {
            id: "1".into(),
            name: "a".into(),
        };
        let base = d.to_base_document().unwrap();
        assert_eq!(base.get("id").unwrap(), &JsonValue::String("1".into()));
    }
    #[test]
    fn paginated_result_construction() {
        let r = PaginatedResult {
            items: vec![1, 2, 3],
            total: 10,
            offset: 0,
            limit: 3,
        };
        assert_eq!(r.items.len(), 3);
        assert_eq!(r.total, 10);
    }
}
