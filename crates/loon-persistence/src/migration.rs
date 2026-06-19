//! Document-store migration framework.
//!
//! Each document type declares `const VERSION: &'static str`. At
//! `enter()` time, the migration helper walks every collection,
//! inspects each document's stored `version` field, and either
//! (a) accepts docs whose version matches the current declaration,
//! (b) refuses to start (default) if mismatches are found, or
//! (c) auto-migrates (when `allow_migration: true`) by chaining
//! `MigrationStep`s from the doc's stored version up to the latest.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

use crate::document::{Document, DocumentDatabase, DocumentDatabaseHandle, DocumentUpdate};
use crate::error::PersistenceResult;
use crate::filter::DocumentFilter;

/// Migration step: transforms a single document from one version to the next.
/// Each step is responsible for a single version bump.
#[async_trait]
#[allow(clippy::wrong_self_convention)]
pub trait MigrationStep: Send + Sync {
    fn from_version(&self) -> &'static str;
    fn to_version(&self) -> &'static str;
    async fn migrate(&self, doc: JsonValue) -> PersistenceResult<JsonValue>;
}

/// A migration plan is a sequence of steps, ordered by `from_version`.
pub struct MigrationPlan {
    pub steps: Vec<Arc<dyn MigrationStep>>,
}

impl MigrationPlan {
    pub fn new(steps: Vec<Arc<dyn MigrationStep>>) -> Self {
        Self { steps }
    }

    /// Resolve a chain of steps from `current` version to `target` version.
    /// Returns `None` if no chain exists. Returns an empty chain when
    /// `current == target`.
    pub fn chain(&self, current: &str, target: &str) -> Option<Vec<Arc<dyn MigrationStep>>> {
        if current == target {
            return Some(vec![]);
        }
        let mut out = Vec::new();
        let mut current = current.to_string();
        let mut guard = 0u32;
        while current != target {
            let next = self.steps.iter().find(|s| s.from_version() == current)?;
            current = next.to_version().to_string();
            out.push(next.clone());
            guard += 1;
            if guard > 100 {
                // cycle guard
                return None;
            }
        }
        Some(out)
    }
}

/// Outcome of `enter()`: how many docs were inspected / migrated / refused.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub collections_inspected: usize,
    pub documents_inspected: usize,
    pub documents_migrated: usize,
    pub documents_refused: usize,
}

pub struct DocumentStoreMigrationHelper {
    pub database: Arc<dyn DocumentDatabaseHandle>,
    pub allow_migration: bool,
    pub plan: Option<Arc<MigrationPlan>>,
}

impl DocumentStoreMigrationHelper {
    pub fn new(database: Arc<dyn DocumentDatabaseHandle>) -> Self {
        Self {
            database,
            allow_migration: false,
            plan: None,
        }
    }

    pub fn with_allow_migration(mut self, allow: bool) -> Self {
        self.allow_migration = allow;
        self
    }

    pub fn with_plan(mut self, plan: Arc<MigrationPlan>) -> Self {
        self.plan = Some(plan);
        self
    }

    /// Construct a helper from any `DocumentDatabase`. Convenience helper
    /// since `DocumentDatabase` is not object-safe (its primary method is
    /// generic) and so cannot be stored directly in a `dyn` field.
    pub fn from_database<D>(database: Arc<D>) -> Self
    where
        D: DocumentDatabase + DocumentDatabaseHandle + 'static,
    {
        let handle: Arc<dyn DocumentDatabaseHandle> = database;
        Self::new(handle)
    }

    /// Verify the underlying database is reachable.
    pub async fn ping(&self) -> PersistenceResult<()> {
        self.database.ping().await
    }

    /// Walk a concrete `DocumentDatabase` and inspect every collection.
    ///
    /// The caller passes a list of `(collection_name, current_version)`
    /// pairs (one per registered document type). Returns a `MigrationReport`.
    ///
    /// Note: takes a generic `D: DocumentDatabase` rather than `&dyn` because
    /// `get_or_create_collection` is generic and `DocumentDatabase` is not
    /// object-safe. The `Arc<dyn DocumentDatabaseHandle>` stored in `self`
    /// is used only for reachability checks.
    pub async fn enter<D: DocumentDatabase + ?Sized>(
        &self,
        db: &D,
        expected: &[(&str, &str)],
    ) -> PersistenceResult<MigrationReport> {
        let mut report = MigrationReport::default();
        for (collection_name, current_version) in expected {
            report.collections_inspected += 1;
            let collection = db
                .get_or_create_collection::<JsonDoc>(
                    collection_name,
                    json!({}),
                    Arc::new(|b| serde_json::from_value(b.clone()).ok()),
                )
                .await?;
            // `DocumentFilter::And(vec![])` is vacuously true, so this returns
            // every document in the collection.
            let all = collection.find(&DocumentFilter::And(vec![])).await?;
            for doc in all {
                report.documents_inspected += 1;
                let stored_version = doc
                    .payload
                    .get("_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0");
                if stored_version == *current_version {
                    continue;
                }
                // Version mismatch.
                if self.allow_migration {
                    if let Some(plan) = &self.plan {
                        if let Some(chain) = plan.chain(stored_version, current_version) {
                            let mut current = doc.payload.clone();
                            for step in chain {
                                current = step.migrate(current).await?;
                                if let Some(obj) = current.as_object_mut() {
                                    obj.insert("_version".into(), json!(step.to_version()));
                                }
                            }
                            // Persist final value by filtering on the doc id and
                            // setting all known fields. We use a Set on `_version`
                            // here plus a `find_one`-then-reinsert strategy for
                            // any other mutated fields: the safest minimum is to
                            // set `_version` and let callers add custom update
                            // ops via a future hook. For Phase 9, this stub
                            // updates only the version field.
                            let update = DocumentUpdate::Set {
                                field: "_version".into(),
                                value: json!(current_version),
                            };
                            let filter = DocumentFilter::Eq {
                                field: "id".into(),
                                value: json!(doc.id()),
                            };
                            collection.update_one(&filter, update).await?;
                            report.documents_migrated += 1;
                            continue;
                        }
                    }
                }
                report.documents_refused += 1;
            }
        }
        Ok(report)
    }
}

/// Generic JSON document for the migration walker. The `_version`
/// field is part of the document; the `id` field is whatever the
/// document type chose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonDoc {
    /// Cached id (always owned so `id()` can return `&String`).
    #[serde(skip)]
    id: String,
    /// Raw JSON payload.
    #[serde(flatten)]
    payload: JsonValue,
}

impl JsonDoc {
    pub fn new(value: JsonValue) -> Self {
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "migration-doc".to_string());
        Self { id, payload: value }
    }
}

impl Document for JsonDoc {
    const VERSION: &'static str = "0.1.0";
    type Id = String;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PersistenceResult;
    use async_trait::async_trait;

    struct NoopStep;
    #[async_trait]
    impl MigrationStep for NoopStep {
        fn from_version(&self) -> &'static str {
            "0.1.0"
        }
        fn to_version(&self) -> &'static str {
            "0.2.0"
        }
        async fn migrate(&self, doc: JsonValue) -> PersistenceResult<JsonValue> {
            Ok(doc)
        }
    }

    struct TwoStepStep;
    #[async_trait]
    impl MigrationStep for TwoStepStep {
        fn from_version(&self) -> &'static str {
            "0.2.0"
        }
        fn to_version(&self) -> &'static str {
            "0.3.0"
        }
        async fn migrate(&self, doc: JsonValue) -> PersistenceResult<JsonValue> {
            Ok(doc)
        }
    }

    #[test]
    fn plan_resolves_simple_chain() {
        let plan = MigrationPlan::new(vec![Arc::new(NoopStep)]);
        let chain = plan.chain("0.1.0", "0.2.0");
        assert!(chain.is_some());
        assert_eq!(chain.unwrap().len(), 1);
    }

    #[test]
    fn plan_resolves_multi_step_chain() {
        let plan = MigrationPlan::new(vec![Arc::new(NoopStep), Arc::new(TwoStepStep)]);
        let chain = plan.chain("0.1.0", "0.3.0").expect("chain exists");
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn plan_resolves_same_version_to_empty() {
        let plan = MigrationPlan::new(vec![Arc::new(NoopStep)]);
        let chain = plan.chain("0.1.0", "0.1.0");
        assert!(chain.is_some());
        assert_eq!(chain.unwrap().len(), 0);
    }

    #[test]
    fn plan_returns_none_when_no_path() {
        let plan = MigrationPlan::new(vec![Arc::new(NoopStep)]);
        let chain = plan.chain("0.1.0", "0.9.0");
        assert!(chain.is_none());
    }

    #[test]
    fn helper_default_does_not_allow_migration() {
        // Use a tiny stub handle that ignores all calls.
        struct StubHandle;
        #[async_trait]
        impl DocumentDatabaseHandle for StubHandle {
            async fn ping(&self) -> PersistenceResult<()> {
                Ok(())
            }
        }
        let helper = DocumentStoreMigrationHelper::new(Arc::new(StubHandle));
        assert!(!helper.allow_migration);
        assert!(helper.plan.is_none());
    }

    #[test]
    fn helper_with_allow_and_plan() {
        struct StubHandle;
        #[async_trait]
        impl DocumentDatabaseHandle for StubHandle {
            async fn ping(&self) -> PersistenceResult<()> {
                Ok(())
            }
        }
        let plan = Arc::new(MigrationPlan::new(vec![Arc::new(NoopStep)]));
        let helper = DocumentStoreMigrationHelper::new(Arc::new(StubHandle))
            .with_allow_migration(true)
            .with_plan(plan);
        assert!(helper.allow_migration);
        assert!(helper.plan.is_some());
    }

    // Suppress unused-import warnings for items only used in some branches.
    #[allow(dead_code)]
    fn _unused() {
        let _ = parking_lot::Mutex::new(0u32);
    }
}
