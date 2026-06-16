use std::path::{Path, PathBuf};
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use uuid::Uuid;
use serde::Serialize;
use serde_json::Value as JsonValue;
use crate::{
    Document, DocumentCollection, DocumentDatabase, DocumentFilter, DocumentUpdate,
    InsertResult, UpdateResult, DeleteResult, BaseDocument, DocumentLoader,
};
use crate::error::PersistenceResult;

pub struct JsonFileDocumentDatabase {
    root: PathBuf,
    flush_interval: Duration,
}

impl JsonFileDocumentDatabase {
    pub fn new(root: &Path, flush_interval: Duration) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            flush_interval,
        })
    }
}

#[async_trait]
impl DocumentDatabase for JsonFileDocumentDatabase {
    async fn get_or_create_collection<TDocument: Document>(
        &self,
        name: &str,
        _schema: JsonValue,
        loader: DocumentLoader<TDocument>,
    ) -> PersistenceResult<Box<dyn DocumentCollection<TDocument>>> {
        let dir = self.root.join(name);
        std::fs::create_dir_all(&dir)?;
        let cache = load_all_from_dir::<TDocument>(&dir, &loader)?;
        Ok(Box::new(JsonFileCollection {
            name: name.to_string(),
            dir,
            cache: Arc::new(RwLock::new(cache)),
            flush_interval: self.flush_interval,
        }))
    }
}

fn load_all_from_dir<T: Document>(
    dir: &Path,
    loader: &DocumentLoader<T>,
) -> PersistenceResult<HashMap<String, T>> {
    let mut map = HashMap::new();
    if !dir.exists() {
        return Ok(map);
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".tmp"))
            .unwrap_or(false)
        {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let base: BaseDocument = serde_json::from_slice(&bytes)?;
        if let Some(doc) = loader(&base) {
            let key = serde_json::to_string(doc.id())?;
            map.insert(key, doc);
        }
    }
    Ok(map)
}

pub struct JsonFileCollection<T: Document> {
    #[allow(dead_code)]
    name: String,
    dir: PathBuf,
    cache: Arc<RwLock<HashMap<String, T>>>,
    #[allow(dead_code)]
    flush_interval: Duration,
}

#[async_trait]
impl<T: Document + 'static> DocumentCollection<T> for JsonFileCollection<T> {
    async fn insert_one(&self, doc: T) -> PersistenceResult<InsertResult> {
        let key = serde_json::to_string(doc.id())?;
        let path = self.dir.join(format!("{}.json", sanitize(&key)));
        let bytes = serde_json::to_vec(&doc)?;
        atomic_write(&path, &bytes)?;
        self.cache.write().insert(key.clone(), doc);
        Ok(InsertResult { id: key })
    }

    async fn find_one(&self, filter: &DocumentFilter) -> PersistenceResult<Option<T>> {
        let all = self.cache.read();
        Ok(all.values().find(|d| matches_filter(*d, filter)).cloned())
    }

    async fn find(&self, filter: &DocumentFilter) -> PersistenceResult<Vec<T>> {
        let all = self.cache.read();
        Ok(all
            .values()
            .filter(|d| matches_filter(*d, filter))
            .cloned()
            .collect())
    }

    async fn update_one(
        &self,
        filter: &DocumentFilter,
        update: DocumentUpdate,
    ) -> PersistenceResult<UpdateResult> {
        let mut cache = self.cache.write();
        let mut matched = 0u64;
        let mut modified = 0u64;
        for (_k, v) in cache.iter_mut() {
            if matches_filter(v, filter) {
                matched += 1;
                let mut value = serde_json::to_value(&*v)?;
                if apply_update(&mut value, &update) {
                    if let Ok(new_doc) = serde_json::from_value::<T>(value) {
                        *v = new_doc;
                        modified += 1;
                        let key = serde_json::to_string(v.id())?;
                        let path = self.dir.join(format!("{}.json", sanitize(&key)));
                        let bytes = serde_json::to_vec(&*v)?;
                        atomic_write(&path, &bytes)?;
                    }
                }
            }
        }
        Ok(UpdateResult { matched, modified })
    }

    async fn delete_one(&self, filter: &DocumentFilter) -> PersistenceResult<DeleteResult> {
        let mut cache = self.cache.write();
        let mut deleted = 0u64;
        let keys: Vec<String> = cache
            .iter()
            .filter(|(_, v)| matches_filter(*v, filter))
            .map(|(k, _)| k.clone())
            .collect();
        for k in keys {
            let path = self.dir.join(format!("{}.json", sanitize(&k)));
            let _ = std::fs::remove_file(&path);
            cache.remove(&k);
            deleted += 1;
        }
        Ok(DeleteResult { deleted })
    }

    async fn count(&self, filter: &DocumentFilter) -> PersistenceResult<u64> {
        Ok(self
            .cache
            .read()
            .values()
            .filter(|d| matches_filter(*d, filter))
            .count() as u64)
    }

    async fn find_sorted(
        &self,
        filter: &DocumentFilter,
        sort_by: &str,
        ascending: bool,
    ) -> PersistenceResult<Vec<T>> {
        let mut docs = self.find(filter).await?;
        docs.sort_by(|a, b| {
            let va = field_value(a, sort_by);
            let vb = field_value(b, sort_by);
            let cmp = match (va, vb) {
                (JsonValue::Number(x), JsonValue::Number(y)) => x
                    .as_f64()
                    .partial_cmp(&y.as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal),
                (JsonValue::String(x), JsonValue::String(y)) => x.cmp(&y),
                _ => std::cmp::Ordering::Equal,
            };
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
        Ok(docs)
    }
}

fn matches_filter<T: Document>(doc: &T, filter: &DocumentFilter) -> bool {
    match filter {
        DocumentFilter::Eq { field, value } => field_value(doc, field) == *value,
        DocumentFilter::In { field, values } => values.contains(&field_value(doc, field)),
        DocumentFilter::And(fs) => fs.iter().all(|f| matches_filter(doc, f)),
        DocumentFilter::Or(fs) => fs.iter().any(|f| matches_filter(doc, f)),
        DocumentFilter::Not(f) => !matches_filter(doc, f),
    }
}

fn field_value<T: Document + Serialize>(doc: &T, field: &str) -> JsonValue {
    let v = serde_json::to_value(doc).unwrap_or(JsonValue::Null);
    v.get(field).cloned().unwrap_or(JsonValue::Null)
}

fn apply_update(value: &mut JsonValue, update: &DocumentUpdate) -> bool {
    match update {
        DocumentUpdate::Set { field, value: val } => {
            if let Some(obj) = value.as_object_mut() {
                obj.insert(field.clone(), val.clone());
                true
            } else {
                false
            }
        }
        DocumentUpdate::Inc { field, by } => {
            if let Some(obj) = value.as_object_mut() {
                if let Some(n) = obj.get(field).and_then(|v| v.as_f64()) {
                    obj.insert(
                        field.clone(),
                        JsonValue::Number(
                            serde_json::Number::from_f64(n + *by as f64)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                    return true;
                }
            }
            false
        }
    }
}

fn sanitize(s: &str) -> String {
    s.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn atomic_write(path: &Path, bytes: &[u8]) -> PersistenceResult<()> {
    let tmp = path.with_extension(format!("json.tmp.{}", Uuid::new_v4()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::filter::DocumentFilter;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tempfile::tempdir;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct Item {
        id: String,
        name: String,
        #[serde(default)]
        qty: i64,
    }

    impl Document for Item {
        const VERSION: &'static str = "0.1.0";
        type Id = String;
        fn id(&self) -> &Self::Id {
            &self.id
        }
    }

    fn loader(doc: &BaseDocument) -> Option<Item> {
        serde_json::from_value::<Item>(doc.clone()).ok()
    }

    #[tokio::test]
    async fn json_file_insert_find_delete() {
        let dir = tempdir().unwrap();
        let db = JsonFileDocumentDatabase::new(dir.path(), Duration::from_secs(1)).unwrap();
        let loader_arc: DocumentLoader<Item> = Arc::new(loader);
        let coll = db
            .get_or_create_collection("items", json!({}), loader_arc)
            .await
            .unwrap();

        let a = Item {
            id: "a".into(),
            name: "alpha".into(),
            qty: 1,
        };
        let b = Item {
            id: "b".into(),
            name: "beta".into(),
            qty: 2,
        };
        coll.insert_one(a.clone()).await.unwrap();
        coll.insert_one(b.clone()).await.unwrap();

        let all = coll
            .find(&DocumentFilter::Or(vec![]))
            .await
            .unwrap();
        // empty Or => true (any of nothing = false). Use And([]) -> all-match.
        let _ = all;

        let found = coll
            .find_one(&DocumentFilter::Eq {
                field: "id".into(),
                value: json!("a"),
            })
            .await
            .unwrap();
        assert_eq!(found, Some(a.clone()));

        let count = coll
            .count(&DocumentFilter::And(vec![]))
            .await
            .unwrap();
        assert_eq!(count, 2);

        let res = coll
            .delete_one(&DocumentFilter::Eq {
                field: "id".into(),
                value: json!("b"),
            })
            .await
            .unwrap();
        assert_eq!(res.deleted, 1);
        let count = coll
            .count(&DocumentFilter::And(vec![]))
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
