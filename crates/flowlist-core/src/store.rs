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
use crate::model::{
    Condition, DueWindow, FilterNode, GroupBy, IconSpec, Perspective, ProjectIcon, SortBy,
    TaskStatus,
};
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
        // A fresh history is written by the first mutation. Rewriting the whole
        // undo log on every launch was a needless write (and it can be large).
        let last_mtime = mtime(&paths.data());
        let mut store = Self {
            paths,
            data,
            history,
            last_mtime,
            warning,
        };
        // Offer the shipped views once. The marker makes deletion and
        // renaming permanent instead of re-seeding on every launch.
        store.seed_shipped_views()?;
        // Trash that outlived the retention setting is permanently removed on
        // open. The purge is a normal command, so it stays in history and can
        // be undone.
        store.sweep_trash()?;
        Ok(store)
    }

    /// Add the saved views Flowlist ships with. They are ordinary records, so
    /// the user can edit, rename or delete them; the settings marker only
    /// stops them from coming back.
    fn seed_shipped_views(&mut self) -> Result<()> {
        if self.data.db.settings.seeded_running_view
            && self.data.db.settings.cleared_running_icon
        {
            return Ok(());
        }
        self.mutate(|db, _at| {
            db.settings.seeded_running_view = true;
            if !db
                .perspectives
                .values()
                .any(|perspective| perspective.name.eq_ignore_ascii_case("Running"))
            {
                let mut perspective = Perspective::new("Running");
                perspective.filter = FilterNode::All(vec![
                    FilterNode::Condition(Condition::Status {
                        value: TaskStatus::Available,
                    }),
                    FilterNode::Condition(Condition::Due {
                        window: DueWindow::Later,
                    }),
                    FilterNode::Any(vec![
                        FilterNode::Condition(Condition::Planned {
                            window: DueWindow::Overdue,
                        }),
                        FilterNode::Condition(Condition::Planned {
                            window: DueWindow::Today,
                        }),
                    ]),
                ]);
                perspective.group_by = GroupBy::Project;
                perspective.sort = SortBy::Planned;
                perspective.pinned = true;
                perspective.icon = None;
                perspective.position = db
                    .perspectives
                    .values()
                    .map(|existing| existing.position)
                    .max()
                    .unwrap_or(0)
                    + 1;
                db.perspectives
                    .insert(perspective.id.clone(), perspective);
            }
            // Retire the old running figure from libraries that saw it. It is
            // kept as a legacy ProjectIcon value only so old data can load.
            if !db.settings.cleared_running_icon {
                for perspective in db.perspectives.values_mut() {
                    if perspective.icon == Some(IconSpec::builtin(ProjectIcon::Running)) {
                        perspective.icon = None;
                    }
                }
                for project in db.projects.values_mut() {
                    if project.icon == IconSpec::builtin(ProjectIcon::Running) {
                        project.icon = IconSpec::default();
                    }
                }
                db.settings.cleared_running_icon = true;
            }
            let mut outcome = Outcome::new("Seed shipped views");
            outcome.message = Some("Added the Running saved view".to_string());
            Ok(outcome)
        })?;
        Ok(())
    }

    /// Permanently remove Trash older than `settings.trash_retention_days`.
    /// A retention of `0` keeps trash forever.
    pub fn sweep_trash(&mut self) -> Result<()> {
        let days = self.data.db.settings.trash_retention_days;
        if days == 0 {
            return Ok(());
        }
        let before = now() - chrono::Duration::days(i64::from(days));
        let expired = self
            .data
            .db
            .tasks
            .values()
            .any(|task| task.trashed_at.is_some_and(|at| at < before));
        if expired {
            self.execute(Command::PurgeExpiredTrash { before })?;
        }
        Ok(())
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
            .map(|label| format!("Undo {}", self.humanize_history_label(label)))
    }

    pub fn redo_label(&self) -> Option<String> {
        self.history
            .label_for_redo()
            .map(|label| format!("Redo {}", self.humanize_history_label(label)))
    }

    /// Older history entries were written with raw ULIDs. Resolve what we can
    /// against the current and neighbouring snapshots so the undo menu keeps
    /// showing names even for work done before the labels changed.
    fn humanize_history_label(&self, label: &str) -> String {
        let mut candidates: Vec<(String, String)> = Vec::new();
        let mut collect = |db: &Db| {
            for (id, task) in &db.tasks {
                candidates.push((id.to_string(), task.title.clone()));
            }
            for (id, project) in &db.projects {
                candidates.push((id.to_string(), project.name.clone()));
            }
            for (id, folder) in &db.folders {
                candidates.push((id.to_string(), folder.name.clone()));
            }
            for (id, tag) in &db.tags {
                candidates.push((id.to_string(), tag.name.clone()));
            }
            for (id, perspective) in &db.perspectives {
                candidates.push((id.to_string(), perspective.name.clone()));
            }
        };
        collect(&self.data.db);
        if let Some(entry) = self
            .history
            .cursor
            .checked_sub(1)
            .and_then(|index| self.history.entries.get(index))
        {
            collect(&entry.state);
        }
        if let Some(entry) = self.history.entries.get(self.history.cursor + 1) {
            collect(&entry.state);
        }

        let mut text = label.to_string();
        for (id, name) in candidates {
            if text.contains(&id) {
                text = text.replace(&id, &format!("“{name}”"));
            }
        }
        // Drop any id that no snapshot can name, rather than showing a ULID.
        text.split_whitespace()
            .filter(|token| !is_ulid_token(token))
            .collect::<Vec<_>>()
            .join(" ")
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
                let label = history.label_for_undo().unwrap_or("change").to_string();
                history.cursor -= 1;
                (label, history.current().cloned())
            }
            Travel::Redo => {
                if !history.can_redo() {
                    return Err(Error::invalid("nothing to redo"));
                }
                let label = history.label_for_redo().unwrap_or("change").to_string();
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
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// ULIDs are 26 Crockford-base32 characters. History labels used to embed
/// them directly; this recognizes the token so it can be hidden when no
/// snapshot can supply a name.
fn is_ulid_token(token: &str) -> bool {
    token.len() == 26
        && token
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('A'..='Z').contains(&ch))
}
