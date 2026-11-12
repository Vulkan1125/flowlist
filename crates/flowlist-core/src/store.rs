//! The store: one library on disk, serialized through a cross-process lock.
//!
//! Every mutation follows the same protocol:
//! lock → reload latest → apply → validate → append history → atomic save.
//! That makes the GUI, CLI and MCP server safe to run at the same time.

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};

use crate::command::{Command, Outcome};
use crate::db::Db;
use crate::error::{Error, Result};
use crate::paths::DataPaths;
use crate::persist::lock::StoreGuard;
use crate::persist::{self, DataFile, HistoryEntry, HistoryFile, SCHEMA_VERSION};
use crate::time::now;

pub const HISTORY_LIMIT: usize = 40 * 2; // 40 undo steps plus redo states
const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Store {
    paths: DataPaths,
    data: DataFile,
    history: HistoryFile,
    last_mtime: Option<SystemTime>,
    warning: Option<String>,
}

impl Store {
    pub fn open(paths: DataPaths) -> Result<Self> {
        paths.ensure()?;

        // Preserve the pre-migration document before loading.
        if let Ok(raw) = persist::atomic::read_json_value(&paths.data()) {
            let version = raw
                .get("schema_version")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32;
            if version < SCHEMA_VERSION {
                let _ = persist::save_migration_backup(&paths, version, &raw);
            }
        }

        let (mut data, mut warning) = match persist::load_data(&paths) {
            Ok(pair) => pair,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                let data = DataFile::new(Db::new());
                persist::save_data(&paths, &data)?;
                persist::save_history(&paths, &HistoryFile::new(&data.db))?;
                (data, None)
            }
            Err(error) => return Err(error),
        };

        if let Err(problem) = data.db.validate() {
            let mut repaired = data.db.clone();
            let fixes = repaired.repair();
            if repaired.validate().is_ok() {
                data.db = repaired;
                data.revision = data.revision.saturating_add(1);
                data.updated_at = now();
                persist::save_data(&paths, &data)?;
                warning = Some(format!(
                    "Library repaired ({problem}); applied: {}",
                    fixes.join(", ")
                ));
            } else {
                return Err(Error::Corrupt(format!(
                    "library invariants failed: {problem}"
                )));
            }
        }

        let history = persist::load_history(&paths, &data.db);
        persist::save_history(&paths, &history)?;
        let last_mtime = mtime(&paths.data());
        Ok(Self {
            paths,
            data,
            history,
            last_mtime,
            warning,
        })
    }

    pub fn paths(&self) -> &DataPaths {
        &self.paths
    }

    pub fn db(&self) -> &Db {
        &self.data.db
    }

    pub fn data(&self) -> &DataFile {
        &self.data
    }

    pub fn history(&self) -> &HistoryFile {
        &self.history
    }

    pub fn revision(&self) -> u64 {
        self.data.revision
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn clear_warning(&mut self) {
        self.warning = None;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo_label(&self) -> Option<String> {
        self.history
            .label_for_undo()
            .map(|label| format!("Undo {label}"))
    }

    pub fn redo_label(&self) -> Option<String> {
        self.history
            .label_for_redo()
            .map(|label| format!("Redo {label}"))
    }

    /// Reload from disk when another process has written. Returns whether the
    /// in-memory snapshot changed.
    pub fn reload_if_changed(&mut self) -> Result<bool> {
        let current = mtime(&self.paths.data());
        if current.is_some() && current == self.last_mtime {
            return Ok(false);
        }
        let (data, warning) = match persist::load_data(&self.paths) {
            Ok(pair) => pair,
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        if data.revision == self.data.revision {
            self.last_mtime = current;
            return Ok(false);
        }
        if let Err(problem) = data.db.validate() {
            return Err(Error::Corrupt(format!(
                "library changed on disk but is invalid: {problem}"
            )));
        }
        self.data = data;
        self.history = persist::load_history(&self.paths, &self.data.db);
        self.last_mtime = current;
        self.warning = warning;
        Ok(true)
    }

    pub fn execute(&mut self, command: Command) -> Result<Outcome> {
        self.mutate(|db, at| command.apply(db, at))
    }

    pub fn undo(&mut self) -> Result<Outcome> {
        self.time_travel(Travel::Undo)
    }

    pub fn redo(&mut self) -> Result<Outcome> {
        self.time_travel(Travel::Redo)
    }

    fn mutate<F>(&mut self, apply: F) -> Result<Outcome>
    where
        F: FnOnce(&mut Db, DateTime<Utc>) -> Result<Outcome>,
    {
        let _guard = StoreGuard::acquire(&self.paths.lock(), LOCK_TIMEOUT)?;
        let (mut data, warning) = self.load_latest_locked()?;
        let mut history = persist::load_history(&self.paths, &data.db);
        if history.current() != Some(&data.db) {
            history = HistoryFile::new(&data.db);
        }

        let at = now();
        let outcome = apply(&mut data.db, at)?;
        data.db.touch();
        data.db.validate()?;
        data.revision = data.revision.saturating_add(1);
        data.updated_at = now();
        push_history(&mut history, outcome.label.clone(), data.db.clone());

        persist::save_data(&self.paths, &data)?;
        persist::save_history(&self.paths, &history)?;

        self.data = data;
        self.history = history;
        self.last_mtime = mtime(&self.paths.data());
        self.warning = warning;
        Ok(outcome)
    }

    fn time_travel(&mut self, direction: Travel) -> Result<Outcome> {
        let _guard = StoreGuard::acquire(&self.paths.lock(), LOCK_TIMEOUT)?;
        let (mut data, warning) = self.load_latest_locked()?;
        let mut history = persist::load_history(&self.paths, &data.db);
        if history.current() != Some(&data.db) {
            history = HistoryFile::new(&data.db);
        }

        let (label, target) = match direction {
            Travel::Undo => {
                if !history.can_undo() {
                    return Err(Error::invalid("nothing to undo"));
                }
                let label = history
                    .label_for_undo()
                    .unwrap_or("change")
                    .to_string();
                history.cursor -= 1;
                (label, history.current().cloned())
            }
            Travel::Redo => {
                if !history.can_redo() {
                    return Err(Error::invalid("nothing to redo"));
                }
                let label = history
                    .label_for_redo()
                    .unwrap_or("change")
                    .to_string();
                history.cursor += 1;
                (label, history.current().cloned())
            }
        };

        let Some(target) = target else {
            return Err(Error::invalid("history is empty"));
        };
        target.validate()?;
        data.db = target;
        data.revision = data.revision.saturating_add(1);
        data.updated_at = now();

        persist::save_data(&self.paths, &data)?;
        persist::save_history(&self.paths, &history)?;

        self.data = data;
        self.history = history;
        self.last_mtime = mtime(&self.paths.data());
        self.warning = warning;
        Ok(Outcome {
            label: format!(
                "{} {label}",
                match direction {
                    Travel::Undo => "Undo",
                    Travel::Redo => "Redo",
                }
            ),
            ..Outcome::default()
        })
    }

    fn load_latest_locked(&self) -> Result<(DataFile, Option<String>)> {
        match persist::load_data(&self.paths) {
            Ok(pair) => Ok(pair),
            Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok((self.data.clone(), None))
            }
            Err(error) => Err(error),
        }
    }

    /// Compose the current database with disk state without persisting. Used
    /// by read-only commands that still want a fresh view.
    pub fn refresh(&mut self) -> Result<()> {
        self.reload_if_changed().map(|_| ())
    }

    pub fn write_session<T: serde::Serialize>(&self, value: &T) -> Result<()> {
        persist::atomic::write_json(&self.paths.session(), value)
    }

    pub fn read_session<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        persist::atomic::read_json(&self.paths.session()).ok()
    }
}

enum Travel {
    Undo,
    Redo,
}

fn push_history(history: &mut HistoryFile, label: String, state: Db) {
    history.entries.truncate(history.cursor + 1);
    history.entries.push(HistoryEntry {
        label,
        at: now(),
        state,
    });
    history.cursor = history.entries.len() - 1;
    if history.entries.len() > HISTORY_LIMIT {
        let remove = history.entries.len() - HISTORY_LIMIT;
        history.entries.drain(0..remove);
        history.cursor = history.cursor.saturating_sub(remove);
    }
}

fn mtime(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}
