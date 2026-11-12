//! Cross-process advisory locking. Flowlist never keeps a lock between
//! commands, so the GUI, CLI and MCP server can interleave safely.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use fs2::FileExt;

use crate::error::{Error, Result};

/// An exclusive lock held for the duration of a read-modify-write cycle.
pub struct StoreGuard {
    file: File,
}

impl StoreGuard {
    pub fn acquire(path: &Path, timeout: Duration) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        let start = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(Error::LockTimeout);
                    }
                    sleep(Duration::from_millis(25));
                }
                // Some filesystems return a different error for contention.
                Err(error) => {
                    if start.elapsed() >= timeout {
                        return Err(error.into());
                    }
                    sleep(Duration::from_millis(25));
                }
            }
        }
    }
}

impl Drop for StoreGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
