//! Remove the app, its command-line link and, on request, the library.
//!
//! A process cannot reliably delete its own bundle while it is running, so the
//! GUI and CLI schedule a tiny shell helper, quit, and let the helper remove
//! the paths a second later. Every path is passed as an argument, never
//! interpolated into shell text.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Plan {
    pub app: Option<PathBuf>,
    pub cli: Vec<PathBuf>,
    pub data: Option<PathBuf>,
    pub extras: Vec<PathBuf>,
}

/// Work out exactly what an uninstall would remove. The library is only
/// included when `purge_data` is set.
pub fn plan(purge_data: bool, data_root: &Path) -> Plan {
    let app = app_bundle();
    let cli = crate::cli_install::executable()
        .map(|exe| crate::cli_install::links_for(&exe))
        .unwrap_or_default();
    let data = purge_data.then(|| data_root.to_path_buf());
    Plan {
        app,
        cli,
        data,
        extras: extra_paths(),
    }
}

/// Paths outside the library that belong to the app: preferences, caches and
/// saved window state. They are removed even when the library is kept.
pub fn extra_paths() -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let library = home.join("Library");
    let bundle = "dev.flowlist.Flowlist";
    [
        library.join("Preferences").join(format!("{bundle}.plist")),
        library
            .join("LaunchAgents")
            .join("dev.flowlist.serve.plist"),
        library.join("Caches").join(bundle),
        library
            .join("Saved Application State")
            .join(format!("{bundle}.savedState")),
        library.join("HTTPStorages").join(bundle),
        library.join("WebKit").join(bundle),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect()
}

pub fn app_bundle() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let exe = fs::canonicalize(&exe).unwrap_or(exe);
    exe.ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(Path::to_path_buf)
}

/// Human-readable plan for confirmation prompts and logs.
pub fn describe(plan: &Plan) -> Vec<String> {
    let mut lines = Vec::new();
    match &plan.app {
        Some(app) => lines.push(format!("App:     {}", app.display())),
        None => lines.push("App:     (no app bundle; leaving the build in place)".to_string()),
    }
    if plan.cli.is_empty() {
        lines.push("Command: (no flowlist link found)".to_string());
    } else {
        for link in &plan.cli {
            lines.push(format!("Command: {}", link.display()));
        }
    }
    if let Some(data) = &plan.data {
        lines.push(format!("Library: {} (deleted)", data.display()));
    } else {
        lines.push("Library: kept".to_string());
    }
    for extra in &plan.extras {
        lines.push(format!("Support: {}", extra.display()));
    }
    lines
}

/// Spawn the delayed cleanup helper. The caller must quit the app immediately
/// after this returns successfully.
pub fn schedule(plan: &Plan) -> Result<(), String> {
    // Stop the login-time mobile server before its plist disappears.
    let _ = crate::serve::agent::remove();
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(app) = &plan.app {
        paths.push(app.clone());
    }
    paths.extend(plan.cli.iter().cloned());
    if let Some(data) = &plan.data {
        paths.push(data.clone());
    }
    paths.extend(plan.extras.iter().cloned());
    if paths.is_empty() {
        return Ok(());
    }

    let script = r#"sleep 1
log="$HOME/Library/Logs/Flowlist-uninstall.log"
mkdir -p "$(dirname "$log")"
: > "$log"
for path in "$@"; do
    if ! /bin/rm -rf -- "$path" 2>>"$log"; then
        echo "failed: $path" >> "$log"
    fi
done
"#;

    let mut command = std::process::Command::new("/bin/sh");
    command.arg("-c").arg(script).arg("flowlist-uninstall");
    for path in &paths {
        command.arg(path);
    }
    command
        .spawn()
        .map_err(|error| format!("Could not start the uninstall helper: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeping_data_only_removes_the_app_and_support_files() {
        let root = PathBuf::from("/tmp/flowlist-test-library");
        let plan = plan(false, &root);
        assert_eq!(plan.data, None);
        assert!(
            plan.app.is_none()
                || plan
                    .app
                    .as_deref()
                    .is_some_and(|path| path.extension().is_some_and(|ext| ext == "app"))
        );
    }

    #[test]
    fn purging_data_includes_the_library() {
        let root = PathBuf::from("/tmp/flowlist-test-library");
        let plan = plan(true, &root);
        assert_eq!(plan.data.as_deref(), Some(root.as_path()));
    }
}
