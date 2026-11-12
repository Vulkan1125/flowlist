//! Atomic file writes with a single rolling backup.
//!
//! The write sequence is:
//!   1. serialize into a sibling temp file and fsync it,
//!   2. copy the current file to `<name>.bak`,
//!   3. rename the temp file over the real path,
//!   4. fsync the containing directory.
//!
//! Readers therefore always see either the old or the new complete file.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::invalid(format!("{} has no parent directory", path.display())))?;
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("data.json");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let tmp: PathBuf = parent.join(format!(".{file_name}.tmp-{}-{stamp}", std::process::id()));

    {
        let mut file = OpenOptions::new().create_new(true).write(true).open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    let backup = path.with_file_name(format!("{file_name}.bak"));
    if path.exists() {
        // A failed copy is not fatal: the rename below still preserves the
        // new data, and the old backup may still be usable.
        let _ = fs::copy(path, &backup);
    }

    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    sync_dir(parent);
    Ok(())
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn read_json_value(path: &Path) -> Result<serde_json::Value> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn sync_dir(parent: &Path) {
    #[cfg(unix)]
    {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
    }
}

/// Load a JSON file, falling back to its `.bak` sibling when the primary
/// file is missing or unreadable. Returns the value plus an optional warning
/// describing what was recovered.
pub fn read_with_backup<T: DeserializeOwned>(path: &Path) -> Result<(T, Option<String>)> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => Ok((value, None)),
            Err(error) => {
                let backup = path.with_file_name(format!(
                    "{}.bak",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("data.json")
                ));
                match fs::read(&backup)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<T>(&bytes).ok())
                {
                    Some(value) => Ok((
                        value,
                        Some(format!(
                            "{} was corrupt ({error}); recovered from backup",
                            path.display()
                        )),
                    )),
                    None => Err(Error::Corrupt(format!("{}: {error}", path.display()))),
                }
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let backup = path.with_file_name(format!(
                "{}.bak",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("data.json")
            ));
            match fs::read(&backup) {
                Ok(bytes) => {
                    let value = serde_json::from_slice(&bytes)?;
                    Ok((
                        value,
                        Some(format!(
                            "{} was missing; recovered from backup",
                            path.display()
                        )),
                    ))
                }
                Err(_) => Err(error.into()),
            }
        }
        Err(error) => Err(error.into()),
    }
}

pub fn copy_backup(paths: &crate::paths::DataPaths, destination: &Path) -> Result<()> {
    let source = paths.data();
    if !source.exists() {
        return Err(Error::not_found("data file", source.display().to_string()));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, destination)?;
    Ok(())
}
