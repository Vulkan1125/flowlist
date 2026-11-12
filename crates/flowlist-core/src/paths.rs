//! Filesystem locations for the library and UI session.

use std::env;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// All files that make up a Flowlist library live in one directory.
#[derive(Clone, Debug)]
pub struct DataPaths {
    pub root: PathBuf,
}

impl DataPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn discover(explicit: Option<PathBuf>) -> Result<Self> {
        if let Some(root) = explicit {
            return Ok(Self::new(root));
        }
        if let Some(root) = env::var_os("FLOWLIST_HOME") {
            if !root.is_empty() {
                return Ok(Self::new(PathBuf::from(root)));
            }
        }
        default_data_dir().map(Self::new)
    }

    pub fn data(&self) -> PathBuf {
        self.root.join("data.json")
    }

    pub fn data_backup(&self) -> PathBuf {
        self.root.join("data.json.bak")
    }

    pub fn history(&self) -> PathBuf {
        self.root.join("history.json")
    }

    pub fn history_backup(&self) -> PathBuf {
        self.root.join("history.json.bak")
    }

    pub fn session(&self) -> PathBuf {
        self.root.join("session.json")
    }

    pub fn lock(&self) -> PathBuf {
        self.root.join("data.lock")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn ensure_backups(&self) -> Result<PathBuf> {
        let dir = self.backups_dir();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

pub fn default_data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = env::var_os("HOME")
            .ok_or_else(|| Error::invalid("HOME is not set; pass --data-dir"))?;
        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Flowlist"));
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = env::var_os("APPDATA")
            .ok_or_else(|| Error::invalid("APPDATA is not set; pass --data-dir"))?;
        return Ok(PathBuf::from(appdata).join("Flowlist"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return Ok(PathBuf::from(xdg).join("flowlist"));
            }
        }
        let home = env::var_os("HOME")
            .ok_or_else(|| Error::invalid("HOME is not set; pass --data-dir"))?;
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("flowlist"));
    }
    #[allow(unreachable_code)]
    Err(Error::invalid(
        "unsupported platform; pass --data-dir explicitly",
    ))
}

pub fn display_path(path: &Path) -> String {
    path.display().to_string()
}
