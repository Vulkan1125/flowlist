//! Launchd agent for the mobile server.
//!
//! Settings → Mobile and `flowlist serve --install-agent` share this module.
//! The agent runs the `serve` subcommand, so closing the window does not stop
//! syncing. It is per-user (`gui/<uid>`), needs no password, and is removed on
//! uninstall.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use flowlist_core::paths::DataPaths;

pub const LABEL: &str = "dev.flowlist.serve";

pub fn plist_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

pub fn log_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Logs")
        .join("Flowlist-serve.log"))
}

pub fn installed() -> bool {
    plist_path().map(|path| path.exists()).unwrap_or(false)
}

/// Whether launchd currently has the job loaded.
pub fn loaded() -> bool {
    let Ok(uid) = uid() else {
        return false;
    };
    Command::new("/bin/launchctl")
        .args(["print", &format!("gui/{uid}/{LABEL}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn install(paths: &DataPaths, port: u16) -> Result<PathBuf> {
    let binary = install_binary();
    let plist = plist_path()?;
    let log = log_path()?;
    if let Some(parent) = plist.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    if let Some(parent) = log.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let text = plist_text(&binary, &paths.root, &log, port);
    std::fs::write(&plist, text).with_context(|| format!("writing {}", plist.display()))?;

    let uid = uid()?;
    let _ = bootout(uid);
    // launchd needs a moment to release the old job. Without this retry a
    // toggle off/on can fail with "Bootstrap failed: 5: Input/output error".
    let mut loaded = false;
    for attempt in 0..6 {
        let status = Command::new("/bin/launchctl")
            .args([
                "bootstrap",
                &format!("gui/{uid}"),
                &plist.display().to_string(),
            ])
            .status()
            .context("running launchctl bootstrap")?;
        if status.success() {
            loaded = true;
            break;
        }
        if attempt < 5 {
            std::thread::sleep(Duration::from_millis(150 * (attempt + 1)));
        }
    }
    if !loaded {
        bail!(
            "launchctl could not load the server agent. Check {}.",
            log.display()
        );
    }
    let _ = Command::new("/bin/launchctl")
        .args(["enable", &format!("gui/{uid}/{LABEL}")])
        .status();
    Ok(plist)
}

/// The executable the agent should run. A binary inside `Flowlist.app` is
/// ideal: macOS can attach the app's name and icon to the login item. When the
/// command is run from a bare build such as `target/release/flowlist`, prefer a
/// packaged app nearby so the login item does not appear as an anonymous
/// executable.
fn install_binary() -> PathBuf {
    let exe = std::env::current_exe()
        .ok()
        .map(|path| std::fs::canonicalize(&path).unwrap_or(path));
    let Some(exe) = exe else {
        return PathBuf::from("flowlist");
    };
    if in_app_bundle(&exe) {
        return exe;
    }
    for ancestor in exe.ancestors().skip(1).take(5) {
        let bundled = ancestor
            .join("dist")
            .join("Flowlist.app")
            .join("Contents")
            .join("MacOS")
            .join("flowlist");
        if bundled.is_file() {
            return bundled;
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    for root in [
        Some(PathBuf::from("/Applications")),
        home.map(|home| home.join("Applications")),
    ]
    .into_iter()
    .flatten()
    {
        let bundled = root
            .join("Flowlist.app")
            .join("Contents")
            .join("MacOS")
            .join("flowlist");
        if bundled.is_file() {
            return bundled;
        }
    }
    exe
}

fn in_app_bundle(path: &Path) -> bool {
    path.to_string_lossy().contains(".app/Contents/MacOS/")
}

/// The launchd plist for the mobile server. `AssociatedBundleIdentifiers`
/// attaches the agent to the Flowlist app bundle, so Login Items shows the app
/// name and icon instead of an anonymous command-line executable.
fn plist_text(binary: &Path, root: &Path, log: &Path, port: u16) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>AssociatedBundleIdentifiers</key>
    <array>
        <string>dev.flowlist.Flowlist</string>
    </array>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>--data-dir</string>
        <string>{root}</string>
        <string>serve</string>
        <string>--port</string>
        <string>{port}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>
"#,
        binary = xml(&binary.display().to_string()),
        root = xml(&root.display().to_string()),
        log = xml(&log.display().to_string()),
    )
}

pub fn remove() -> Result<bool> {
    let plist = plist_path()?;
    let was_loaded = loaded();
    if let Ok(uid) = uid() {
        let _ = bootout(uid);
    }
    let existed = plist.exists();
    if existed {
        std::fs::remove_file(&plist).with_context(|| format!("removing {}", plist.display()))?;
    }
    Ok(existed || was_loaded)
}

pub fn restart(paths: &DataPaths, port: u16) -> Result<PathBuf> {
    let _ = remove();
    install(paths, port)
}

fn bootout(uid: u32) -> bool {
    Command::new("/bin/launchctl")
        .args(["bootout", &format!("gui/{uid}/{LABEL}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn uid() -> Result<u32> {
    let output = Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .context("running id -u")?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim().parse::<u32>().context("reading the user id")
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn logs() -> Option<PathBuf> {
    log_path().ok().filter(|path| path.exists())
}

pub fn describe(root: &Path) -> String {
    match plist_path() {
        Ok(path) => format!("{} (library {})", path.display(), root.display()),
        Err(_) => "LaunchAgent unavailable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escapes_paths() {
        assert_eq!(xml("/a&b/<c>"), "/a&amp;b/&lt;c&gt;");
    }

    #[test]
    fn recognises_app_bundle_executables() {
        assert!(in_app_bundle(Path::new(
            "/Applications/Flowlist.app/Contents/MacOS/flowlist"
        )));
        assert!(!in_app_bundle(Path::new(
            "/Users/me/Projects/flowlist/target/release/flowlist"
        )));
    }

    #[test]
    fn plist_associates_the_agent_with_the_app() {
        let text = plist_text(
            Path::new("/Applications/Flowlist.app/Contents/MacOS/flowlist"),
            Path::new("/Users/me/Library/Application Support/Flowlist"),
            Path::new("/Users/me/Library/Logs/Flowlist-serve.log"),
            7777,
        );
        assert!(text.contains("<key>AssociatedBundleIdentifiers</key>"));
        assert!(text.contains("<string>dev.flowlist.Flowlist</string>"));
        assert!(text.contains("<string>7777</string>"));
    }
}
