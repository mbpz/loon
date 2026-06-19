//! Application-level wrapper around `TagStore`.

use std::sync::Arc;

use loon_core::stores::TagStore;
use loon_core::{CoreResult, Tag, TagId};

pub struct TagAppModule {
    pub store: Arc<dyn TagStore>,
}

impl TagAppModule {
    pub fn new(store: Arc<dyn TagStore>) -> Self {
        Self { store }
    }

    pub async fn create_tag(&self, tag: Tag) -> CoreResult<Tag> {
        self.store.create(tag).await
    }

    pub async fn read_tag(&self, id: &TagId) -> CoreResult<Option<Tag>> {
        self.store.read(id).await
    }

    pub async fn list_tags(&self) -> CoreResult<Vec<Tag>> {
        self.store.list().await
    }

    pub async fn delete_tag(&self, id: &TagId) -> CoreResult<()> {
        self.store.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use loon_core::TagUpdateParams;
    use parking_lot::Mutex;
    use std::collections::HashMap;

    pub struct FakeTagStore {
        pub data: Mutex<HashMap<TagId, Tag>>,
    }
    impl FakeTagStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl TagStore for FakeTagStore {
        async fn create(&self, tag: Tag) -> CoreResult<Tag> {
            let id = tag.id.clone();
            self.data.lock().insert(id, tag.clone());
            Ok(tag)
        }
        async fn read(&self, id: &TagId) -> CoreResult<Option<Tag>> {
            Ok(self.data.lock().get(id).cloned())
        }
        async fn update(&self, id: &TagId, params: TagUpdateParams) -> CoreResult<Tag> {
            use loon_core::CoreError;
            use loon_core::UniqueId;
            let mut d = self.data.lock();
            let t = d
                .get_mut(id)
                .ok_or_else(|| CoreError::NotFound(UniqueId(id.0.clone())))?;
            if let Some(name) = params.name {
                t.name = name;
            }
            Ok(t.clone())
        }
        async fn list(&self) -> CoreResult<Vec<Tag>> {
            Ok(self.data.lock().values().cloned().collect())
        }
        async fn delete(&self, id: &TagId) -> CoreResult<()> {
            self.data.lock().remove(id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn tag_create_read_list() {
        let store: Arc<dyn TagStore> = Arc::new(FakeTagStore::new());
        let module = TagAppModule::new(store);
        let t = module.create_tag(Tag::new("foo")).await.unwrap();
        let loaded = module.read_tag(&t.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "foo");
        let all = module.list_tags().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    /// Regression: \`FakeTagStore\` must implement every method of
    /// the \`TagStore\` trait, including \`update\`. Before this was
    /// added, \`loon-app-modules\` failed to compile with
    /// \`E0046: not all trait items implemented\`. This test goes
    /// through the trait object so a missing method is a compile
    /// error rather than a silent gap.
    #[tokio::test]
    async fn fake_tag_store_implements_update() {
        let store: Arc<dyn TagStore> = Arc::new(FakeTagStore::new());
        let t = Tag::new("original");
        let id = t.id.clone();
        store.create(t).await.unwrap();
        let updated = store
            .update(
                &id,
                TagUpdateParams {
                    name: Some("renamed".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.id, id, "update must preserve identity");
    }
}
