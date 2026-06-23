use crate::error::PersistenceResult;
use crate::filter::DocumentFilter;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use std::hash::Hash;
use std::sync::Arc;

pub type BaseDocument = JsonValue;
pub type DocumentLoader<T> = std::sync::Arc<dyn Fn(&BaseDocument) -> Option<T> + Send + Sync>;

pub trait Document: Serialize + DeserializeOwned + Send + Sync + Clone + 'static {
    const VERSION: &'static str;
    type Id: Serialize + DeserializeOwned + Send + Sync + Eq + Hash + std::fmt::Display;
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

/// Object-safe subset of `DocumentDatabase` used for type-erased handles.
///
/// `DocumentDatabase::get_or_create_collection` is generic over the document
/// type, so it cannot appear in a `dyn DocumentDatabase` vtable. Callers
/// that need an opaque handle (such as the migration helper, the SDK
/// `ServerBuilder`, or the document-backed entity stores) use this
/// trait object as the type-erased stand-in. Collection access works at
/// the [`BaseDocument`] (= `serde_json::Value`) level so the trait stays
/// dyn-compatible.
#[async_trait]
pub trait DocumentDatabaseHandle: Send + Sync {
    /// Lightweight reachability check.
    async fn ping(&self) -> PersistenceResult<()>;

    /// Open (or lazily create) a named collection. Returns a
    /// type-erased handle that operates at the `BaseDocument` level.
    async fn collection(
        &self,
        name: &str,
    ) -> PersistenceResult<Arc<dyn DocumentCollectionHandle>>;
}

/// Object-safe counterpart of [`DocumentCollection`]. Mirrors the core
/// CRUD surface but using [`BaseDocument`] (= `serde_json::Value`) so
/// the trait can be used through a `dyn` pointer. Each backend
/// implements it independently of [`DocumentCollection<T>`].
#[async_trait]
pub trait DocumentCollectionHandle: Send + Sync {
    async fn insert_one(&self, doc: BaseDocument) -> PersistenceResult<()>;
    async fn find_one(
        &self,
        filter: &DocumentFilter,
    ) -> PersistenceResult<Option<BaseDocument>>;
    async fn find(&self, filter: &DocumentFilter) -> PersistenceResult<Vec<BaseDocument>>;
    async fn update_one(
        &self,
        filter: &DocumentFilter,
        update: DocumentUpdate,
    ) -> PersistenceResult<UpdateResult>;
    async fn delete_one(&self, filter: &DocumentFilter) -> PersistenceResult<DeleteResult>;
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
    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
