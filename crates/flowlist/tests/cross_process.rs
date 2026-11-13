//! The GUI, CLI and MCP server all point at one library. These tests spawn the
//! real binary so the lock, atomic save and reload path run across processes,
//! not just between two `Store` values in one test process.

use std::path::Path;
use std::process::{Command, Output};

use flowlist_core::command::Command as CoreCommand;
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;
use serde_json::Value;
use tempfile::TempDir;

fn run_cli(root: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_flowlist"))
        .arg("--data-dir")
        .arg(root)
        .args(args)
        .output()
        .expect("run the flowlist binary");
    assert!(
        output.status.success(),
        "flowlist {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("command emits JSON")
}

fn library() -> (TempDir, DataPaths) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path().join("library"));
    (dir, paths)
}

#[test]
fn a_long_lived_store_reloads_a_write_from_the_cli() {
    let (_dir, paths) = library();
    let mut gui = Store::open(paths.clone()).unwrap();
    assert!(gui.db().tasks.is_empty());

    run_cli(&paths.root, &["add", "Written by the CLI", "--json"]);
    assert!(
        gui.reload_if_changed().unwrap(),
        "the GUI notices the other process"
    );
    assert_eq!(gui.db().tasks.len(), 1);
    assert!(
        gui.db()
            .tasks
            .values()
            .any(|task| task.title == "Written by the CLI")
    );
    // The other process's undo step becomes available in the GUI.
    assert!(gui.can_undo());

    // Reloading again is a no-op, not an endless repaint.
    assert!(!gui.reload_if_changed().unwrap());
}

#[test]
fn the_cli_sees_a_write_from_a_long_lived_store() {
    let (_dir, paths) = library();
    let mut gui = Store::open(paths.clone()).unwrap();
    run_cli(&paths.root, &["add", "Ship the release", "--json"]);
    assert!(gui.reload_if_changed().unwrap());

    let id = gui.db().tasks.values().next().unwrap().id.clone();
    gui.execute(CoreCommand::CompleteTasks {
        ids: vec![id],
        completed: true,
    })
    .unwrap();

    let listed = json(&run_cli(&paths.root, &["list", "--completed", "--json"]));
    let titles: Vec<&str> = listed["groups"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|group| group["tasks"].as_array().into_iter().flatten())
        .filter_map(|task| task["title"].as_str())
        .collect();
    assert_eq!(titles, ["Ship the release"]);

    // The GUI's own history keeps the undo step after the reload dance.
    gui.undo().unwrap();
    assert!(!gui.db().tasks.values().next().unwrap().completed);
}
