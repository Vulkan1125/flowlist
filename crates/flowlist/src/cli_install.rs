//! Put the bundled `flowlist` executable on the user's PATH.
//!
//! A macOS app bundle does not install its command-line binary. This module
//! creates a symlink named `flowlist` in the first writable directory already
//! on PATH, preferring the user's own `~/.local/bin` so nothing needs admin
//! rights. The Settings sheet and `flowlist install-cli` share it.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Installed {
    pub path: PathBuf,
    pub on_path: bool,
    pub replaced: bool,
}

#[derive(Clone, Debug)]
pub enum Status {
    Installed { path: PathBuf, on_path: bool },
    Missing,
}

/// The executable behind the current process, with symlinks resolved so the
/// link we create points at the real bundle and not at `~/.local/bin/flowlist`.
pub fn executable() -> Result<PathBuf, String> {
    let exe = env::current_exe().map_err(|error| format!("Cannot find the executable: {error}"))?;
    fs::canonicalize(&exe).map_err(|error| format!("Cannot resolve {}: {error}", exe.display()))
}

/// Look for a `flowlist` link that already points at this executable.
pub fn status() -> Status {
    let Ok(exe) = executable() else {
        return Status::Missing;
    };
    let path_dirs = path_dirs();
    for (dir, on_path) in candidate_dirs(&path_dirs) {
        let link = dir.join("flowlist");
        if points_to(&link, &exe) {
            return Status::Installed {
                path: link,
                on_path,
            };
        }
    }
    Status::Missing
}

/// Install or repair the link. Returns the chosen path and whether that
/// directory is currently on PATH.
pub fn install() -> Result<Installed, String> {
    let exe = executable()?;
    let path_dirs = path_dirs();
    let mut last_error: Option<String> = None;

    // Prefer a writable directory the shell already searches. User-local
    // directories come first so we do not write into Homebrew's bin when it is
    // not necessary.
    for (dir, on_path) in candidate_dirs(&path_dirs) {
        if !on_path || !dir.is_dir() {
            continue;
        }
        match install_into(&dir, &exe) {
            Ok(replaced) => {
                return Ok(Installed {
                    path: dir.join("flowlist"),
                    on_path: true,
                    replaced,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                last_error = Some(format!("{} is not writable", dir.display()));
            }
            Err(error) => {
                return Err(format!("Could not install into {}: {error}", dir.display()));
            }
        }
    }

    // No PATH directory is writable. Fall back to ~/.local/bin and tell the
    // caller that it still has to be added to PATH.
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let dir = home.join(".local").join("bin");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create {}: {error}", dir.display()))?;
    let replaced = install_into(&dir, &exe).map_err(|error| {
        format!(
            "Could not install into {}: {error}{}",
            dir.display(),
            last_error
                .as_deref()
                .map(|previous| format!(" ({previous})"))
                .unwrap_or_default()
        )
    })?;
    Ok(Installed {
        path: dir.join("flowlist"),
        on_path: path_dirs.iter().any(|candidate| same_dir(candidate, &dir)),
        replaced,
    })
}

/// Append a PATH line to `~/.zshrc` when the link lives outside PATH. Used by
/// the Settings sheet only; the CLI prints the line instead of editing a shell
/// profile without being asked.
pub fn add_to_zshrc(dir: &Path) -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;
    let rc = home.join(".zshrc");
    let line = format!("export PATH=\"{}:$PATH\"", dir.display());
    let existing = fs::read_to_string(&rc).unwrap_or_default();
    if existing.lines().any(|entry| entry.trim() == line) {
        return Ok(rc);
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n# Flowlist command-line tool\n");
    text.push_str(&line);
    text.push('\n');
    fs::write(&rc, text).map_err(|error| format!("Could not update {}: {error}", rc.display()))?;
    Ok(rc)
}

fn install_into(dir: &Path, exe: &Path) -> std::io::Result<bool> {
    let link = dir.join("flowlist");
    let mut replaced = false;
    if let Ok(metadata) = fs::symlink_metadata(&link) {
        if metadata.file_type().is_symlink() {
            if points_to(&link, exe) {
                return Ok(false);
            }
            fs::remove_file(&link)?;
            replaced = true;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "a regular file named flowlist already exists there",
            ));
        }
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(exe, &link)?;
    #[cfg(not(unix))]
    fs::copy(exe, &link)?;
    Ok(replaced)
}

fn points_to(link: &Path, exe: &Path) -> bool {
    let Ok(target) = fs::read_link(link) else {
        return false;
    };
    let target = if target.is_absolute() {
        target
    } else {
        link.parent().map(|dir| dir.join(&target)).unwrap_or(target)
    };
    match fs::canonicalize(&target) {
        Ok(target) => target == exe,
        Err(_) => target == exe,
    }
}

fn path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn candidate_dirs(path_dirs: &[PathBuf]) -> Vec<(PathBuf, bool)> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let mut dirs = Vec::new();
    if let Some(home) = &home {
        dirs.push(home.join(".local").join("bin"));
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    if let Some(home) = &home {
        dirs.push(home.join("bin"));
    }
    dirs.into_iter()
        .map(|dir| {
            let on_path = path_dirs.iter().any(|candidate| same_dir(candidate, &dir));
            (dir, on_path)
        })
        .collect()
}

fn same_dir(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_order_prefers_a_user_local_bin() {
        let home = PathBuf::from("/Users/example");
        let dirs = candidate_dirs(&[home.join(".local/bin")]);
        assert_eq!(dirs[0].0, home.join(".local/bin"));
        assert!(dirs[0].1);
        assert!(
            dirs.iter()
                .any(|(dir, _)| dir == Path::new("/opt/homebrew/bin"))
        );
    }
}
