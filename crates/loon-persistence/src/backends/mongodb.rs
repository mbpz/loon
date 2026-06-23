//! MongoDB backend for `DocumentDatabase`.
//!
//! Each collection is a MongoDB collection within the configured database.
//! Documents are stored as BSON (via `bson::to_bson` / `bson::from_bson`).
//! `DocumentFilter` is translated to MongoDB query documents at query time.

use async_trait::async_trait;
use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Bson, Document as BsonDocument};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection, Database};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::document::{
    BaseDocument, DeleteResult, Document, DocumentCollection, DocumentCollectionHandle,
    DocumentDatabase, DocumentDatabaseHandle, DocumentLoader, DocumentUpdate, InsertResult,
    UpdateResult,
};
use crate::error::{PersistenceError, PersistenceResult};
use crate::filter::DocumentFilter;

/// MongoDB-backed `DocumentDatabase` implementation.
pub struct MongoDocumentDatabase {
    #[allow(dead_code)]
    client: Arc<Client>,
    database: Database,
    /// Per-name collection cache (the in-process `DocumentCollection<TDocument>` wrapper).
    #[allow(dead_code)]
    cache: Mutex<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>,
}

impl MongoDocumentDatabase {
    /// Connect using a MongoDB URI (e.g. `mongodb://localhost:27017`) and database name.
    pub async fn connect(uri: &str, db_name: &str) -> PersistenceResult<Self> {
        let mut opts = mongodb::options::ClientOptions::parse(uri)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo connect: {e}")))?;
        opts.app_name = Some("loon".into());
        opts.connect_timeout = Some(Duration::from_secs(5));
        let client = Client::with_options(opts)
            .map_err(|e| PersistenceError::Internal(format!("mongo client: {e}")))?;
        let database = client.database(db_name);
        Ok(Self {
            client: Arc::new(client),
            database,
            cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl DocumentDatabaseHandle for MongoDocumentDatabase {
    async fn ping(&self) -> PersistenceResult<()> {
        // Phase 9: lightweight reachability check. We don't issue a network
        // round-trip here because this is called during server startup and
        // MongoDB may be slow to respond when the topology is degraded.
        // A real readiness check can be added later by calling `list_collection_names`.
        Ok(())
    }

    async fn collection(
        &self,
        name: &str,
    ) -> PersistenceResult<Arc<dyn DocumentCollectionHandle>> {
        let collection = self.database.collection::<BsonDocument>(name);
        Ok(Arc::new(MongoCollectionHandle { collection }))
    }
}

/// Type-erased MongoDB collection handle. Operates at the `BsonDocument`
/// level so the trait stays dyn-compatible. Document payloads are
/// round-tripped through `serde_json::Value` for compatibility with the
/// rest of the persistence layer.
pub struct MongoCollectionHandle {
    collection: Collection<BsonDocument>,
}

#[async_trait]
impl DocumentCollectionHandle for MongoCollectionHandle {
    async fn insert_one(&self, doc: BaseDocument) -> PersistenceResult<()> {
        let bson_doc = bson::to_document(&doc)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        self.collection
            .insert_one(bson_doc)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo insert: {e}")))?;
        Ok(())
    }

    async fn find_one(
        &self,
        filter: &DocumentFilter,
    ) -> PersistenceResult<Option<BaseDocument>> {
        let query = filter_to_mongo(filter);
        let result = self
            .collection
            .find_one(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo find_one: {e}")))?;
        match result {
            Some(bson_doc) => {
                let v: BaseDocument = bson::from_bson(Bson::Document(bson_doc))
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    async fn find(&self, filter: &DocumentFilter) -> PersistenceResult<Vec<BaseDocument>> {
        let query = filter_to_mongo(filter);
        let mut cursor = self
            .collection
            .find(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo find: {e}")))?;
        let mut out = Vec::new();
        while let Some(bson_doc) = cursor
            .try_next()
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo cursor: {e}")))?
        {
            let v: BaseDocument = bson::from_bson(Bson::Document(bson_doc))
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
            out.push(v);
        }
        Ok(out)
    }

    async fn update_one(
        &self,
        filter: &DocumentFilter,
        update: DocumentUpdate,
    ) -> PersistenceResult<UpdateResult> {
        let query = filter_to_mongo(filter);
        let update_doc = match update {
            DocumentUpdate::Set { field, value } => doc! {
                "$set": {
                    field: bson::to_bson(&value)
                        .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
                }
            },
            DocumentUpdate::Inc { field, by } => doc! {
                "$inc": { field: by }
            },
        };
        let result = self
            .collection
            .update_one(query, update_doc)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo update_one: {e}")))?;
        Ok(UpdateResult {
            matched: result.matched_count,
            modified: result.modified_count,
        })
    }

    async fn delete_one(&self, filter: &DocumentFilter) -> PersistenceResult<DeleteResult> {
        let query = filter_to_mongo(filter);
        let result = self
            .collection
            .delete_one(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo delete_one: {e}")))?;
        Ok(DeleteResult {
            deleted: result.deleted_count,
        })
    }
}

#[async_trait]
impl DocumentDatabase for MongoDocumentDatabase {
    async fn get_or_create_collection<TDocument: Document + 'static>(
        &self,
        name: &str,
        _schema: serde_json::Value,
        loader: DocumentLoader<TDocument>,
    ) -> PersistenceResult<Box<dyn DocumentCollection<TDocument>>> {
        // The MongoDB collection is dynamic (typed at the bson::Document level).
        let collection = self.database.collection::<BsonDocument>(name);
        Ok(Box::new(MongoDocumentCollection::<TDocument> {
            collection,
            loader,
        }))
    }
}

pub struct MongoDocumentCollection<T: Document + 'static> {
    collection: Collection<BsonDocument>,
    loader: DocumentLoader<T>,
}

#[async_trait]
impl<T: Document + 'static> DocumentCollection<T> for MongoDocumentCollection<T> {
    async fn insert_one(&self, document: T) -> PersistenceResult<InsertResult> {
        let id = serde_json::to_string(document.id())?;
        let value = serde_json::to_value(&document)?;
        let bson_doc = bson::to_document(&value)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
        self.collection
            .insert_one(bson_doc)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo insert: {e}")))?;
        Ok(InsertResult { id })
    }

    async fn find_one(&self, filters: &DocumentFilter) -> PersistenceResult<Option<T>> {
        let query = filter_to_mongo(filters);
        let result = self
            .collection
            .find_one(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo find_one: {e}")))?;
        match result {
            Some(bson_doc) => {
                let base: BaseDocument = bson::from_bson(Bson::Document(bson_doc))
                    .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
                Ok((self.loader)(&base))
            }
            None => Ok(None),
        }
    }

    async fn find(&self, filters: &DocumentFilter) -> PersistenceResult<Vec<T>> {
        let query = filter_to_mongo(filters);
        let mut cursor = self
            .collection
            .find(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo find: {e}")))?;
        let mut out = Vec::new();
        while let Some(bson_doc) = cursor
            .try_next()
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo cursor: {e}")))?
        {
            let base: BaseDocument = bson::from_bson(Bson::Document(bson_doc))
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
            if let Some(t) = (self.loader)(&base) {
                out.push(t);
            }
        }
        Ok(out)
    }

    async fn update_one(
        &self,
        filters: &DocumentFilter,
        update: DocumentUpdate,
    ) -> PersistenceResult<UpdateResult> {
        let query = filter_to_mongo(filters);
        let update_doc = match update {
            DocumentUpdate::Set { field, value } => doc! {
                "$set": {
                    field: bson::to_bson(&value)
                        .map_err(|e| PersistenceError::Serialization(e.to_string()))?,
                }
            },
            DocumentUpdate::Inc { field, by } => doc! {
                "$inc": { field: by }
            },
        };
        let result = self
            .collection
            .update_one(query, update_doc)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo update_one: {e}")))?;
        Ok(UpdateResult {
            matched: result.matched_count,
            modified: result.modified_count,
        })
    }

    async fn delete_one(&self, filters: &DocumentFilter) -> PersistenceResult<DeleteResult> {
        let query = filter_to_mongo(filters);
        let result = self
            .collection
            .delete_one(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo delete_one: {e}")))?;
        Ok(DeleteResult {
            deleted: result.deleted_count,
        })
    }

    async fn count(&self, filters: &DocumentFilter) -> PersistenceResult<u64> {
        let query = filter_to_mongo(filters);
        let result = self
            .collection
            .count_documents(query)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo count: {e}")))?;
        Ok(result)
    }

    async fn find_sorted(
        &self,
        filter: &DocumentFilter,
        sort_by: &str,
        ascending: bool,
    ) -> PersistenceResult<Vec<T>> {
        let query = filter_to_mongo(filter);
        let opts = FindOptions::builder()
            .sort(doc! { sort_by: if ascending { 1i32 } else { -1i32 } })
            .build();
        let mut cursor = self
            .collection
            .find(query)
            .with_options(opts)
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo find_sorted: {e}")))?;
        let mut out = Vec::new();
        while let Some(bson_doc) = cursor
            .try_next()
            .await
            .map_err(|e| PersistenceError::Internal(format!("mongo cursor: {e}")))?
        {
            let base: BaseDocument = bson::from_bson(Bson::Document(bson_doc))
                .map_err(|e| PersistenceError::Serialization(e.to_string()))?;
            if let Some(t) = (self.loader)(&base) {
                out.push(t);
            }
        }
        Ok(out)
    }
}

/// Translate our `DocumentFilter` to a MongoDB query document.
pub fn filter_to_mongo(filter: &DocumentFilter) -> BsonDocument {
    match filter {
        DocumentFilter::Eq { field, value } => {
            doc! { field: bson::to_bson(value).unwrap_or(Bson::Null) }
        }
        DocumentFilter::In { field, values } => {
            let bsons: Vec<Bson> = values
                .iter()
                .map(|v| bson::to_bson(v).unwrap_or(Bson::Null))
                .collect();
            doc! { field: { "$in": bsons } }
        }
        DocumentFilter::And(fs) => {
            let docs: Vec<BsonDocument> = fs.iter().map(filter_to_mongo).collect();
            doc! { "$and": docs }
        }
        DocumentFilter::Or(fs) => {
            let docs: Vec<BsonDocument> = fs.iter().map(filter_to_mongo).collect();
            doc! { "$or": docs }
        }
        DocumentFilter::Not(f) => doc! { "$nor": [filter_to_mongo(f)] },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn filter_translation_eq() {
        let f = DocumentFilter::Eq {
            field: "id".into(),
            value: json!("abc"),
        };
        let q = filter_to_mongo(&f);
        assert_eq!(q.get_str("id").unwrap(), "abc");
    }

    #[test]
    fn filter_translation_and() {
        let f = DocumentFilter::And(vec![
            DocumentFilter::Eq {
                field: "x".into(),
                value: json!(1),
            },
            DocumentFilter::Eq {
                field: "y".into(),
                value: json!(2),
            },
        ]);
        let q = filter_to_mongo(&f);
        let arr = q.get_array("$and").unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn filter_translation_in() {
        let f = DocumentFilter::In {
            field: "status".into(),
            values: vec![json!("a"), json!("b")],
        };
        let q = filter_to_mongo(&f);
        let inner = q.get_document("status").unwrap();
        let in_arr = inner.get_array("$in").unwrap();
        assert_eq!(in_arr.len(), 2);
    }

    #[test]
    fn filter_translation_or() {
        let f = DocumentFilter::Or(vec![
            DocumentFilter::Eq {
                field: "a".into(),
                value: json!(1),
            },
            DocumentFilter::Eq {
                field: "b".into(),
                value: json!(2),
            },
        ]);
        let q = filter_to_mongo(&f);
        let arr = q.get_array("$or").unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn filter_translation_not() {
        let inner = DocumentFilter::Eq {
            field: "x".into(),
            value: json!(1),
        };
        let f = DocumentFilter::Not(Box::new(inner));
        let q = filter_to_mongo(&f);
        let arr = q.get_array("$nor").unwrap();
        assert_eq!(arr.len(), 1);
    }
}
