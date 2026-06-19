//! JSON-file → JSON-file migration helper.
//!
//! Phase 1 / Phase 2 transition: a tool that scans a directory of `.json`
//! files and applies a migration plan to each one. The result is written
//! atomically (via `*.json.tmp.<uuid>` + rename) so a crash mid-write
//! cannot corrupt the on-disk document.

use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use crate::error::{PersistenceError, PersistenceResult};
use crate::migration::MigrationPlan;

pub struct JsonFileMigrator {
    pub plan: Arc<MigrationPlan>,
}

impl JsonFileMigrator {
    pub fn new(plan: Arc<MigrationPlan>) -> Self {
        Self { plan }
    }

    /// Migrate every `.json` file under `dir` in place. Returns the
    /// number of files migrated.
    pub async fn migrate_dir(&self, dir: &Path) -> PersistenceResult<usize> {
        let mut entries = fs::read_dir(dir).await.map_err(PersistenceError::Io)?;
        let mut count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(PersistenceError::Io)? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            // Skip in-flight temporary files left over by a prior crash.
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.ends_with(".tmp"))
                .unwrap_or(false)
            {
                continue;
            }
            self.migrate_file(&path).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Migrate a single JSON file. Walks the migration chain from
    /// the file's `version` to the latest version in the plan.
    pub async fn migrate_file(&self, path: &Path) -> PersistenceResult<()> {
        let bytes = fs::read(path).await.map_err(PersistenceError::Io)?;
        let mut current: JsonValue =
            serde_json::from_slice(&bytes).map_err(PersistenceError::Json)?;
        let target = self.latest_version();
        let start = current
            .get("_version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0");
        if let Some(chain) = self.plan.chain(start, target) {
            for step in chain {
                current = step.migrate(current).await?;
                if let Some(obj) = current.as_object_mut() {
                    obj.insert("_version".into(), serde_json::json!(step.to_version()));
                }
            }
        }
        // Write back atomically.
        let tmp = tmp_path(path);
        let bytes = serde_json::to_vec_pretty(&current).map_err(PersistenceError::Json)?;
        fs::write(&tmp, &bytes)
            .await
            .map_err(PersistenceError::Io)?;
        fs::rename(&tmp, path).await.map_err(PersistenceError::Io)?;
        Ok(())
    }

    /// Find the max-reachable version in the plan.
    pub fn latest_version(&self) -> &str {
        self.plan
            .steps
            .iter()
            .map(|s| s.to_version())
            .max()
            .unwrap_or("0.1.0")
    }
}

fn tmp_path(p: &Path) -> PathBuf {
    let mut s = p.to_path_buf();
    let new_ext = match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    };
    s.set_extension(new_ext);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::MigrationStep;
    use async_trait::async_trait;
    use serde_json::json;
    use tempfile::tempdir;

    struct AddField;
    #[async_trait]
    impl MigrationStep for AddField {
        fn from_version(&self) -> &'static str {
            "0.1.0"
        }
        fn to_version(&self) -> &'static str {
            "0.2.0"
        }
        async fn migrate(
            &self,
            mut doc: serde_json::Value,
        ) -> crate::error::PersistenceResult<serde_json::Value> {
            if let Some(obj) = doc.as_object_mut() {
                obj.insert("new_field".into(), json!("hello"));
            }
            Ok(doc)
        }
    }

    #[tokio::test]
    async fn migrates_dir_in_place() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.json");
        tokio::fs::write(&path, br#"{"_version":"0.1.0","id":"x"}"#)
            .await
            .unwrap();
        let plan = Arc::new(MigrationPlan::new(vec![Arc::new(AddField)]));
        let mig = JsonFileMigrator::new(plan);
        let n = mig.migrate_dir(dir.path()).await.unwrap();
        assert_eq!(n, 1);
        let bytes = tokio::fs::read(&path).await.unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["_version"], "0.2.0");
        assert_eq!(doc["new_field"], "hello");
    }

    #[tokio::test]
    async fn skips_non_json_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        tokio::fs::write(&path, b"not json").await.unwrap();
        let plan = Arc::new(MigrationPlan::new(vec![Arc::new(AddField)]));
        let mig = JsonFileMigrator::new(plan);
        let n = mig.migrate_dir(dir.path()).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn migrates_already_at_target_is_noop() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.json");
        tokio::fs::write(&path, br#"{"_version":"0.2.0","id":"x"}"#)
            .await
            .unwrap();
        let plan = Arc::new(MigrationPlan::new(vec![Arc::new(AddField)]));
        let mig = JsonFileMigrator::new(plan);
        mig.migrate_file(&path).await.unwrap();
        let bytes = tokio::fs::read(&path).await.unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["_version"], "0.2.0");
        // No migration step ran, so `new_field` should not have been added.
        assert!(doc.get("new_field").is_none());
    }

    #[test]
    fn tmp_path_appends_tmp_extension() {
        let p = Path::new("/a/b/doc.json");
        let t = tmp_path(p);
        assert_eq!(t, PathBuf::from("/a/b/doc.json.tmp"));
    }
}
