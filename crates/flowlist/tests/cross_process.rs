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
#[test]
fn notification_flags_round_trip_through_the_cli() {
    let (_dir, paths) = library();
    let date = (chrono::Utc::now() + chrono::Duration::days(2))
        .format("%Y-%m-%d 10:00")
        .to_string();
    let created = json(&run_cli(
        &paths.root,
        &[
            "add",
            "Notify me",
            "--due",
            "2026-12-01 17:00",
            "--estimate",
            "1h",
            "--alerts",
            "due:-30m, defer",
            "--remove-notification",
            "planned",
            "--custom-notification",
            &date,
            "--json",
        ],
    ));
    assert_eq!(created["alerts_text"], "due:-30m, defer");
    assert_eq!(
        created["removed_notifications"],
        serde_json::json!(["planned"])
    );
    assert!(
        created["custom_notifications"][0]
            .as_str()
            .unwrap()
            .contains("10:00")
    );

    let id = created["id"].as_str().unwrap().to_string();
    let restored = json(&run_cli(
        &paths.root,
        &[
            "edit",
            &id,
            "--restore-notification",
            "planned",
            "--clear-custom-notifications",
            "--json",
        ],
    ));
    assert_eq!(restored["removed_notifications"], serde_json::json!([]));
    assert_eq!(restored["custom_notifications"], serde_json::json!([]));

    let project = json(&run_cli(
        &paths.root,
        &["project", "add", "Notify project", "--json"],
    ));
    let project_id = project["id"].as_str().unwrap().to_string();
    let edited = json(&run_cli(
        &paths.root,
        &[
            "project",
            "edit",
            &project_id,
            "--due",
            "2026-12-02 09:00",
            "--remove-notification",
            "due",
            "--custom-notification",
            "2026-12-03",
            "--alerts",
            "due:+15m",
            "--json",
        ],
    ));
    assert_eq!(edited["removed_notifications"], serde_json::json!(["due"]));
    assert_eq!(edited["alerts_text"], "due:+15m");
    assert!(
        edited["custom_notifications"][0]
            .as_str()
            .unwrap()
            .starts_with("2026-12-03")
    );
}
