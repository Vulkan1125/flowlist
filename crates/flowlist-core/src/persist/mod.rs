//! On-disk schema, migration entry point and history file.

pub mod atomic;
pub mod lock;
pub mod migrate;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::paths::DataPaths;
use crate::time::now;

pub const SCHEMA_VERSION: u32 = 1;

/// The whole library as stored in `data.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataFile {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default = "now")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub db: Db,
}

fn current_schema() -> u32 {
    SCHEMA_VERSION
}

impl DataFile {
    pub fn new(db: Db) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            revision: 1,
            updated_at: now(),
            db,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub label: String,
    #[serde(default = "now")]
    pub at: DateTime<Utc>,
    pub state: Db,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryFile {
    #[serde(default = "current_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
    #[serde(default)]
    pub cursor: usize,
}

impl HistoryFile {
    pub fn new(db: &Db) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entries: vec![HistoryEntry {
                label: "Initial state".to_string(),
                at: now(),
                state: db.clone(),
            }],
            cursor: 0,
        }
    }

    pub fn current(&self) -> Option<&Db> {
        self.entries.get(self.cursor).map(|entry| &entry.state)
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }

    pub fn label_for_undo(&self) -> Option<&str> {
        self.can_undo()
            .then(|| {
                self.entries
                    .get(self.cursor)
                    .map(|entry| entry.label.as_str())
            })
            .flatten()
    }

    pub fn label_for_redo(&self) -> Option<&str> {
        self.can_redo()
            .then(|| {
                self.entries
                    .get(self.cursor + 1)
                    .map(|entry| entry.label.as_str())
            })
            .flatten()
    }
}

pub fn load_data(paths: &DataPaths) -> Result<(DataFile, Option<String>)> {
    let (value, warning) = atomic::read_with_backup::<serde_json::Value>(&paths.data())?;
    let file = migrate::prepare(value)?;
    Ok((file, warning))
}

pub fn save_data(paths: &DataPaths, file: &DataFile) -> Result<()> {
    atomic::write_json(&paths.data(), file)
}

pub fn load_history(paths: &DataPaths, current: &Db) -> HistoryFile {
    let loaded = atomic::read_with_backup::<HistoryFile>(&paths.history())
        .ok()
        .map(|(history, _)| history)
        .filter(|history| {
            !history.entries.is_empty()
                && history.cursor < history.entries.len()
                && history.current() == Some(current)
        });
    loaded.unwrap_or_else(|| HistoryFile::new(current))
}

pub fn save_history(paths: &DataPaths, history: &HistoryFile) -> Result<()> {
    atomic::write_json(&paths.history(), history)
}

/// Preserve the pre-migration document before rewriting it.
pub fn save_migration_backup(
    paths: &DataPaths,
    from_version: u32,
    value: &serde_json::Value,
) -> Result<()> {
    let dir = paths.ensure_backups()?;
    let stamp = now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("data-v{from_version}-{stamp}.json"));
    atomic::write_json(&path, value)?;
    clean_backups(&dir, 20);
    Ok(())
}

fn clean_backups(dir: &std::path::Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    let remove = files.len().saturating_sub(keep);
    for path in files.into_iter().take(remove) {
        let _ = std::fs::remove_file(path);
    }
}

/// Utility used by `flowlist doctor` and tests: serialize and parse a database
/// exactly as the store would.
pub fn roundtrip_db(db: &Db) -> Result<Db> {
    let file = DataFile::new(db.clone());
    let value = serde_json::to_value(&file)?;
    let parsed: DataFile = serde_json::from_value(value)?;
    if parsed.schema_version != SCHEMA_VERSION {
        return Err(Error::invalid("schema version changed during roundtrip"));
    }
    Ok(parsed.db)
}
