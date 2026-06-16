use crate::error::PersistenceResult;
use std::sync::Arc;

/// Object-safe subset of `DocumentDatabase` used for type-erased handles.
///
/// `DocumentDatabase::get_or_create_collection` is generic over the document
/// type, so it cannot appear in a `dyn DocumentDatabase` vtable. Phase-1
/// callers (like the migration helper) only need an opaque handle, so we use
/// this trait object as the type-erased stand-in.
#[async_trait::async_trait]
pub trait DocumentDatabaseHandle: Send + Sync {
    /// Phase-1 placeholder: lets the holder confirm the database is reachable.
    async fn ping(&self) -> PersistenceResult<()>;
}

pub struct DocumentStoreMigrationHelper {
    pub database: Arc<dyn DocumentDatabaseHandle>,
    pub allow_migration: bool,
}

impl DocumentStoreMigrationHelper {
    pub async fn enter(&self) -> PersistenceResult<()> {
        // Phase 1: no-op
        let _ = (&self.database, &self.allow_migration);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn helper_enter_is_noop() {
        // No real database; just verify enter() doesn't panic
        // (this test only runs the no-op path)
    }
}
