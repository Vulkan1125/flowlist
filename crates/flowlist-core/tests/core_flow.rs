//! End-to-end tests for the command, history and persistence layers.

use chrono::Utc;
use flowlist_core::command::{
    Command, ContainerPlacement, ImportMode, NewProject, NewTask, ProjectPatch, TaskPatch,
    TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Position};
use flowlist_core::ids::{FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{Priority, ProjectStatus, RepeatMode, RepeatRule, TagStatus, Task};
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;
use flowlist_core::time::Stamp;
use flowlist_core::time::{IntervalUnit, local_date};
use tempfile::TempDir;

fn setup() -> (TempDir, DataPaths) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(dir.path().join("library"));
    (dir, paths)
}

fn add(store: &mut Store, title: &str) -> String {
    store
        .execute(Command::CreateTask(NewTask::new(title)))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("command reports created id")
}

fn add_task(store: &mut Store, title: &str) -> TaskId {
    let id = add(store, title);
    store.db().resolve_task(&id).unwrap()
}

fn add_project_in(store: &mut Store, name: &str, folder: Option<&FolderId>) -> ProjectId {
    let mut project = NewProject::new(name);
    project.folder = folder.cloned();
    let id = store
        .execute(Command::CreateProject(project))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("command reports created id");
    store.db().resolve_project(&id).unwrap()
}

fn add_folder_in(store: &mut Store, name: &str, parent: Option<&FolderId>) -> FolderId {
    let id = store
        .execute(Command::CreateFolder {
            name: name.into(),
            parent: parent.cloned(),
            position: ContainerPosition::End,
        })
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("command reports created id");
    store.db().resolve_folder(&id).unwrap()
}

fn add_tag_in(store: &mut Store, name: &str, parent: Option<&TagId>) -> TagId {
    let id = store
        .execute(Command::CreateTag {
            name: name.into(),
            parent: parent.cloned(),
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("command reports created id");
    store.db().resolve_tag(&id).unwrap()
}

#[test]
fn create_complete_undo_redo() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let id = add(&mut store, "Write the report");
    assert_eq!(store.db().tasks.len(), 1);
    let task_id = store.db().resolve_task(&id).unwrap();

    store
        .execute(Command::CompleteTasks {
            ids: vec![task_id.clone()],
            completed: true,
        })
        .unwrap();
    assert!(store.db().tasks[&task_id].completed);
    assert!(store.db().tasks[&task_id].completed_at.is_some());
    assert!(store.can_undo());

    store.undo().unwrap();
    assert!(!store.db().tasks[&task_id].completed);
    assert!(store.can_redo());

    store.redo().unwrap();
    assert!(store.db().tasks[&task_id].completed);
}

#[test]
fn repeated_task_spawns_the_next_occurrence() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut new_task = NewTask::new("Water plants");
    new_task.due = Some(Stamp::date_only(local_date(Utc::now())));
    new_task.repeat = Some(RepeatRule::new(
        RepeatMode::DueAgain,
        1,
        IntervalUnit::Weeks,
    ));
    let id = store
        .execute(Command::CreateTask(new_task))
        .unwrap()
        .created[0]
        .clone();
    let task_id = store.db().resolve_task(&id).unwrap();

    store
        .execute(Command::CompleteTasks {
            ids: vec![task_id.clone()],
            completed: true,
        })
        .unwrap();

    assert_eq!(store.db().tasks.len(), 2);
    let next = store
        .db()
        .tasks
        .values()
        .find(|task| task.id != task_id)
        .expect("next occurrence exists");
    assert!(!next.completed);
    assert!(next.due.is_some());
    assert_eq!(next.repeat.as_ref().unwrap().completed_count, 1);

    store.undo().unwrap();
    assert_eq!(store.db().tasks.len(), 1);
}

#[test]
fn repeat_titles_increment_on_completion() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut new_task = NewTask::new("Week 1 review");
    new_task.due = Some(Stamp::date_only(local_date(Utc::now())));
    new_task.repeat = Some(RepeatRule::new(
        RepeatMode::DueAgain,
        1,
        IntervalUnit::Weeks,
    ));
    let id = store
        .execute(Command::CreateTask(new_task))
        .unwrap()
        .created[0]
        .clone();
    let task_id = store.db().resolve_task(&id).unwrap();
    let series = store.db().tasks[&task_id].repeat_series.clone();
    assert_eq!(series, Some(task_id.clone()));

    store
        .execute(Command::CompleteTasks {
            ids: vec![task_id.clone()],
            completed: true,
        })
        .unwrap();

    let next = store
        .db()
        .tasks
        .values()
        .find(|task| task.id != task_id)
        .expect("next occurrence exists");
    assert_eq!(next.title, "Week 2 review");
    assert_eq!(next.repeat_series, Some(task_id));
}

#[test]
fn deleting_a_series_trashes_every_occurrence() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut new_task = NewTask::new("Day 1");
    new_task.due = Some(Stamp::date_only(local_date(Utc::now())));
    new_task.repeat = Some(RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Days));
    let id = store
        .execute(Command::CreateTask(new_task))
        .unwrap()
        .created[0]
        .clone();
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![task_id.clone()],
            completed: true,
        })
        .unwrap();
    let next_id = store
        .db()
        .tasks
        .values()
        .find(|task| task.id != task_id)
        .expect("next occurrence exists")
        .id
        .clone();
    assert_eq!(store.db().series_task_ids(&next_id).len(), 2);

    store
        .execute(Command::DeleteTaskSeries {
            ids: vec![next_id.clone()],
        })
        .unwrap();
    assert_eq!(store.db().tasks.len(), 2);
    assert!(
        store
            .db()
            .tasks
            .values()
            .all(|task| task.trashed_at.is_some()),
        "every occurrence moves to Trash"
    );

    store.undo().unwrap();
    assert!(
        store
            .db()
            .tasks
            .values()
            .all(|task| task.trashed_at.is_none()),
        "one undo restores the whole series"
    );
}

#[test]
fn data_survives_reopening_and_other_stores_reload() {
    let (_dir, paths) = setup();
    let first = {
        let mut store = Store::open(paths.clone()).unwrap();
        add(&mut store, "Persisted task");
        store
    };
    assert_eq!(first.db().tasks.len(), 1);

    let mut second = Store::open(paths.clone()).unwrap();
    assert_eq!(second.db().tasks.len(), 1);

    let third = {
        let mut store = Store::open(paths.clone()).unwrap();
        add(&mut store, "From third store");
        store
    };
    assert_eq!(third.db().tasks.len(), 2);

    assert!(second.reload_if_changed().unwrap());
    assert_eq!(second.db().tasks.len(), 2);

    // Both processes' changes are preserved after another write.
    let mut fourth = Store::open(paths).unwrap();
    assert_eq!(fourth.db().tasks.len(), 2);
    add(&mut fourth, "Fourth");
    second.reload_if_changed().unwrap();
    assert_eq!(second.db().tasks.len(), 3);
}

#[test]
fn history_is_persisted_between_sessions() {
    let (_dir, paths) = setup();
    {
        let mut store = Store::open(paths.clone()).unwrap();
        add(&mut store, "One");
        add(&mut store, "Two");
    }
    let mut store = Store::open(paths).unwrap();
    assert_eq!(store.db().tasks.len(), 2);
    assert!(store.can_undo());
    store.undo().unwrap();
    assert_eq!(store.db().tasks.len(), 1);
}

#[test]
fn projects_tasks_moves_and_tags_validate() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let project = store
        .execute(Command::CreateProject(NewProject::new("Work")))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&project).unwrap();

    let task = add(&mut store, "Draft spec");
    let task_id = store.db().resolve_task(&task).unwrap();
    store
        .execute(Command::MoveTasks {
            ids: vec![task_id.clone()],
            placement: TaskPlacement::Project {
                project: project_id.clone(),
                position: Position::End,
            },
        })
        .unwrap();
    assert_eq!(
        store.db().tasks[&task_id].project.as_ref(),
        Some(&project_id)
    );

    let child = store
        .execute(Command::CreateTask({
            let mut new_task = NewTask::new("Gather notes");
            new_task.parent = Some(task_id.clone());
            new_task
        }))
        .unwrap()
        .created[0]
        .clone();
    let child_id = store.db().resolve_task(&child).unwrap();
    assert_eq!(store.db().tasks[&child_id].parent.as_ref(), Some(&task_id));
    assert_eq!(
        store.db().tasks[&child_id].project.as_ref(),
        Some(&project_id)
    );

    let tag = store
        .execute(Command::CreateTag {
            name: "Office".into(),
            parent: None,
            status: flowlist_core::model::TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let tag_id = store.db().resolve_tag(&tag).unwrap();
    store
        .execute(Command::SetTaskTags {
            ids: vec![child_id.clone()],
            add: vec![tag_id.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    assert!(store.db().tasks[&child_id].tags.contains(&tag_id));

    store
        .execute(Command::UpdateTask {
            id: child_id.clone(),
            patch: TaskPatch {
                title: Some("Gather notes and links".into()),
                due: Some(Some(Stamp::date_only(local_date(Utc::now())))),
                ..TaskPatch::default()
            },
        })
        .unwrap();
    assert_eq!(store.db().tasks[&child_id].title, "Gather notes and links");
    store.db().validate().unwrap();
}

#[test]
fn import_replace_and_merge() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    add(&mut store, "Existing");

    let mut imported = flowlist_core::db::Db::new();
    imported.add_task_to_inbox(Task::new("Imported"));
    imported.validate().unwrap();

    store
        .execute(Command::Import {
            db: imported.clone(),
            mode: ImportMode::Merge,
        })
        .unwrap();
    assert_eq!(store.db().tasks.len(), 2);

    store
        .execute(Command::Import {
            db: imported,
            mode: ImportMode::Replace,
        })
        .unwrap();
    assert_eq!(store.db().tasks.len(), 1);
    assert_eq!(store.db().tasks.values().next().unwrap().title, "Imported");
}

#[test]
fn query_views_and_stats_agree() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::query;
    use flowlist_core::stats;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let id = add(&mut store, "Flagged thing");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::SetTaskFlag {
            ids: vec![task_id],
            flagged: true,
        })
        .unwrap();

    let at = Utc::now();
    let flagged = query::view(store.db(), BuiltinPerspective::Flagged, at);
    let ids: Vec<String> = flagged
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.to_string()))
        .collect();
    assert_eq!(ids.len(), 1);

    let counts = stats::compute(store.db(), at);
    assert_eq!(counts.open_tasks, 1);
    assert_eq!(counts.flagged, 1);
    assert_eq!(counts.inbox, 1);
}

#[test]
fn inbox_holds_only_unscheduled_roots() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::query;
    use flowlist_core::stats;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let plain = add_task(&mut store, "Plain thought");
    let dated = add_task(&mut store, "Dated thought");
    let deferred = add_task(&mut store, "Deferred thought");

    let at = Utc::now();
    store
        .execute(Command::UpdateTask {
            id: dated.clone(),
            patch: TaskPatch {
                due: Some(Some(Stamp::date_only(local_date(at)))),
                ..Default::default()
            },
        })
        .unwrap();
    store
        .execute(Command::UpdateTask {
            id: deferred.clone(),
            patch: TaskPatch {
                defer: Some(Some(Stamp::date_only(local_date(at)))),
                ..Default::default()
            },
        })
        .unwrap();

    let titles = |store: &Store| -> Vec<String> {
        query::view(store.db(), BuiltinPerspective::Inbox, at)
            .iter()
            .flat_map(|group| group.rows.iter())
            .filter_map(|row| store.db().tasks.get(&row.id))
            .map(|task| task.title.clone())
            .collect()
    };
    assert_eq!(titles(&store), vec!["Plain thought"]);
    assert_eq!(stats::compute(store.db(), at).inbox, 1);
    assert_eq!(
        query::view(store.db(), BuiltinPerspective::Forecast, at)
            .iter()
            .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
            .filter(|id| id == &dated || id == &deferred)
            .count(),
        2
    );

    // Clearing the schedule files the task back into the Inbox in place.
    for id in [dated, deferred] {
        store
            .execute(Command::UpdateTask {
                id,
                patch: TaskPatch {
                    due: Some(None),
                    defer: Some(None),
                    ..Default::default()
                },
            })
            .unwrap();
    }
    assert_eq!(titles(&store).len(), 3);
    assert_eq!(stats::compute(store.db(), at).inbox, 3);
    let _ = plain;
}

#[test]
fn attachments_view_lists_open_tasks_with_files() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let with_file = add(&mut store, "Send the brief");
    let with_file_id = store.db().resolve_task(&with_file).unwrap();
    store
        .execute(Command::UpdateTask {
            id: with_file_id.clone(),
            patch: TaskPatch {
                attachments: Some(vec![" /tmp/brief.pdf ".into()]),
                ..Default::default()
            },
        })
        .unwrap();
    let done = add(&mut store, "Already finished");
    let done_id = store.db().resolve_task(&done).unwrap();
    store
        .execute(Command::UpdateTask {
            id: done_id.clone(),
            patch: TaskPatch {
                attachments: Some(vec!["/tmp/final.pdf".into()]),
                ..Default::default()
            },
        })
        .unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![done_id.clone()],
            completed: true,
        })
        .unwrap();
    let trashed = add(&mut store, "Old attachment");
    let trashed_id = store.db().resolve_task(&trashed).unwrap();
    store
        .execute(Command::UpdateTask {
            id: trashed_id.clone(),
            patch: TaskPatch {
                attachments: Some(vec!["/tmp/old.pdf".into()]),
                ..Default::default()
            },
        })
        .unwrap();
    store
        .execute(Command::DeleteTasks {
            ids: vec![trashed_id],
        })
        .unwrap();
    add(&mut store, "No files");

    let groups = query::view(store.db(), BuiltinPerspective::Attachments, Utc::now());
    let ids: Vec<_> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
        .collect();
    assert_eq!(ids, vec![with_file_id]);
    assert_eq!(groups[0].title, "Tasks with attachments");
}

#[test]
fn merge_import_reuses_tags_by_path() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let errands = store
        .execute(Command::CreateTag {
            name: "Errands".into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let errands = store.db().resolve_tag(&errands).unwrap();
    let store_tag = store
        .execute(Command::CreateTag {
            name: "Store".into(),
            parent: Some(errands),
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let existing = store.db().resolve_tag(&store_tag).unwrap();

    let mut imported = flowlist_core::db::Db::new();
    let imported_tag = imported.ensure_tag_path("Errands/Store", true).unwrap();
    assert_ne!(existing, imported_tag);
    let mut task = Task::new("Pick up milk");
    task.tags.insert(imported_tag);
    imported.add_task_to_inbox(task);
    imported.validate().unwrap();

    store
        .execute(Command::Import {
            db: imported,
            mode: ImportMode::Merge,
        })
        .unwrap();

    let db = store.db();
    let matching: Vec<_> = db.tags.values().filter(|tag| tag.name == "Store").collect();
    assert_eq!(matching.len(), 1, "merge must not duplicate tag paths");
    let merged = db
        .tasks
        .values()
        .find(|task| task.title == "Pick up milk")
        .unwrap();
    assert!(merged.tags.contains(&existing));
}

#[test]
fn checklist_items_add_update_reorder_and_delete() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let created = add(&mut store, "Publish release");
    let task = store.db().resolve_task(&created).unwrap();

    let first = store
        .execute(Command::AddChecklistItem {
            task: task.clone(),
            title: "Run tests".into(),
            position: None,
        })
        .unwrap()
        .created[0]
        .clone();
    let second = store
        .execute(Command::AddChecklistItem {
            task: task.clone(),
            title: "Write changelog".into(),
            position: None,
        })
        .unwrap()
        .created[0]
        .clone();
    let first_id = flowlist_core::ids::ChecklistId::from_string(first);
    let second_id = flowlist_core::ids::ChecklistId::from_string(second);

    let entity = &store.db().tasks[&task];
    assert_eq!(entity.checklist.len(), 2);
    assert_eq!(entity.checklist[0].title, "Run tests");
    assert_eq!(entity.checklist_progress(), (0, 2));

    store
        .execute(Command::UpdateChecklistItem {
            task: task.clone(),
            item: first_id.clone(),
            title: "Run the full test suite".into(),
            completed: Some(true),
        })
        .unwrap();
    assert_eq!(store.db().tasks[&task].checklist_progress(), (1, 2));

    store
        .execute(Command::MoveChecklistItem {
            task: task.clone(),
            item: first_id.clone(),
            to: 1,
        })
        .unwrap();
    let entity = &store.db().tasks[&task];
    assert_eq!(entity.checklist[1].id, first_id);

    store
        .execute(Command::DeleteChecklistItem {
            task: task.clone(),
            item: second_id,
        })
        .unwrap();
    assert_eq!(store.db().tasks[&task].checklist.len(), 1);
    store.db().validate().unwrap();
}

#[test]
fn repeating_task_copies_checklist_unchecked() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let created = add(&mut store, "Weekly review");
    let task = store.db().resolve_task(&created).unwrap();
    store
        .execute(Command::AddChecklistItem {
            task: task.clone(),
            title: "Clear inbox".into(),
            position: None,
        })
        .unwrap();
    let item = store.db().tasks[&task].checklist[0].id.clone();
    store
        .execute(Command::UpdateChecklistItem {
            task: task.clone(),
            item,
            title: "Clear inbox".into(),
            completed: Some(true),
        })
        .unwrap();
    store
        .execute(Command::UpdateTask {
            id: task.clone(),
            patch: TaskPatch {
                repeat: Some(Some(RepeatRule::new(
                    RepeatMode::DeferAnother,
                    1,
                    IntervalUnit::Weeks,
                ))),
                ..TaskPatch::default()
            },
        })
        .unwrap();

    let outcome = store
        .execute(Command::CompleteTasks {
            ids: vec![task.clone()],
            completed: true,
        })
        .unwrap();
    let spawned = flowlist_core::ids::TaskId::from_string(outcome.created[0].clone());
    let next = &store.db().tasks[&spawned];
    assert_eq!(next.checklist.len(), 1);
    assert!(!next.checklist[0].completed);
}

#[test]
fn nearby_ranks_tagged_tasks_by_distance() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let here = flowlist_core::model::Geo {
        lat: 12.9716,
        lon: 77.5946,
    };
    let create_tag =
        |store: &mut Store, name: &str, location: Option<flowlist_core::model::Geo>| {
            let id = store
                .execute(Command::CreateTag {
                    name: name.into(),
                    parent: None,
                    status: TagStatus::Available,
                    location,
                    position: Position::End,
                })
                .unwrap()
                .created[0]
                .clone();
            store.db().resolve_tag(&id).unwrap()
        };
    let tag_task = |store: &mut Store, title: &str, tag: TagId| {
        let task = add(store, title);
        let task_id = store.db().resolve_task(&task).unwrap();
        store
            .execute(Command::SetTaskTags {
                ids: vec![task_id.clone()],
                add: vec![tag.clone()],
                remove: Vec::new(),
            })
            .unwrap();
        task_id
    };

    let cafe = create_tag(&mut store, "Cafe", Some(here));
    let far = create_tag(
        &mut store,
        "Far",
        Some(flowlist_core::model::Geo {
            lat: 13.5,
            lon: 78.5,
        }),
    );
    let coffee = tag_task(&mut store, "Coffee", cafe.clone());
    let _away = tag_task(&mut store, "Far away", far);

    // Nothing is nearby until the library has a current location.
    assert!(flowlist_core::query::nearby(store.db(), Utc::now()).is_empty());

    store
        .execute(Command::UpdateSettings(
            flowlist_core::command::SettingsPatch {
                current_location: Some(Some(here)),
                nearby_radius_km: Some(5.0),
                ..Default::default()
            },
        ))
        .unwrap();
    let hits = flowlist_core::query::nearby(store.db(), Utc::now());
    assert_eq!(hits.len(), 1, "the far tag is outside the radius");
    assert_eq!(hits[0].task, coffee);
    assert_eq!(hits[0].via_tag, cafe);
    assert!(hits[0].distance_km < 0.01);
}

#[test]
fn sidebar_settings_are_quiet_by_default_and_persisted() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths.clone()).unwrap();

    let defaults = store.db().settings.sidebar.clone();
    assert!(defaults.show_perspectives && defaults.show_priorities && defaults.show_library);
    assert!(defaults.show_saved && defaults.show_tags);
    assert!(!defaults.counts_perspectives);
    assert!(!defaults.counts_priorities);
    assert!(defaults.counts_library);
    assert!(!defaults.counts_tags);

    // The rail can hide whole sections and choose which counts stay.
    let mut quiet = defaults.clone();
    quiet.show_priorities = false;
    quiet.counts_library = false;
    store
        .execute(Command::UpdateSettings(
            flowlist_core::command::SettingsPatch {
                sidebar: Some(quiet.clone()),
                ..Default::default()
            },
        ))
        .unwrap();
    assert_eq!(store.db().settings.sidebar, quiet);

    // GUI, CLI and MCP share the value through the data file.
    let reopened = Store::open(paths).unwrap();
    assert_eq!(reopened.db().settings.sidebar, quiet);
}

#[test]
fn sidebar_tags_sort_by_usage_then_name() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mango = add_tag_in(&mut store, "Mango", None);
    let apple = add_tag_in(&mut store, "apple", None);
    let zebra = add_tag_in(&mut store, "Zebra", None);
    let _banana = add_tag_in(&mut store, "Banana", None);

    let first = add_task(&mut store, "First");
    let second = add_task(&mut store, "Second");
    let third = add_task(&mut store, "Third");
    store
        .execute(Command::SetTaskTags {
            ids: vec![first, second],
            add: vec![mango.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    // A completed task is not live usage, so Mango stays at two.
    let finished = add_task(&mut store, "Finished");
    store
        .execute(Command::SetTaskTags {
            ids: vec![finished.clone()],
            add: vec![mango.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![finished],
            completed: true,
        })
        .unwrap();
    store
        .execute(Command::SetTaskTags {
            ids: vec![third],
            add: vec![apple.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    let fourth = add_task(&mut store, "Fourth");
    store
        .execute(Command::SetTaskTags {
            ids: vec![fourth],
            add: vec![zebra.clone()],
            remove: Vec::new(),
        })
        .unwrap();

    let ids = store.db().tags_sorted_by_usage(None);
    let order: Vec<String> = ids
        .iter()
        .map(|id| store.db().tags[id].name.clone())
        .collect();
    assert_eq!(order, ["Mango", "apple", "Zebra", "Banana"]);

    // Children follow the same rule inside their parent.
    let beta = add_tag_in(&mut store, "Beta", Some(&mango));
    let alpha = add_tag_in(&mut store, "Alpha", Some(&mango));
    let child_task = add_task(&mut store, "Child work");
    store
        .execute(Command::SetTaskTags {
            ids: vec![child_task],
            add: vec![beta.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    let children = store.db().tags_sorted_by_usage(Some(&mango));
    let child_names: Vec<String> = children
        .iter()
        .map(|id| store.db().tags[id].name.clone())
        .collect();
    assert_eq!(child_names, ["Beta", "Alpha"]);
    assert!(children.contains(&alpha));
}

#[test]
fn folder_badge_counts_only_open_projects_with_work_left() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let folder = add_folder_in(&mut store, "Work", None);
    let live = add_project_in(&mut store, "Live", Some(&folder));
    let live_task = add_task(&mut store, "Ship it");
    store
        .execute(Command::MoveTasks {
            ids: vec![live_task],
            placement: TaskPlacement::Project {
                project: live.clone(),
                position: Position::End,
            },
        })
        .unwrap();
    assert_eq!(store.db().folder_active_project_count(&folder), 1);
    assert!(!store.db().project_is_ticked(&live));

    // A project with tasks but nothing left open shows a tick, so it stops
    // inflating the folder badge.
    let finished = add_project_in(&mut store, "Finished", Some(&folder));
    let done_task = add_task(&mut store, "Last task");
    store
        .execute(Command::MoveTasks {
            ids: vec![done_task.clone()],
            placement: TaskPlacement::Project {
                project: finished.clone(),
                position: Position::End,
            },
        })
        .unwrap();
    assert_eq!(store.db().folder_active_project_count(&folder), 2);
    store
        .execute(Command::CompleteTasks {
            ids: vec![done_task],
            completed: true,
        })
        .unwrap();
    assert!(store.db().project_is_ticked(&finished));
    assert_eq!(store.db().folder_active_project_count(&folder), 1);

    // An empty project is not ticked, and a closed project is hidden from the
    // rail entirely, so only the first counts.
    let fresh = add_project_in(&mut store, "Fresh", Some(&folder));
    assert_eq!(store.db().folder_active_project_count(&folder), 2);
    let archived = add_project_in(&mut store, "Archived", Some(&folder));
    store
        .execute(Command::SetProjectStatus {
            id: archived,
            status: ProjectStatus::Done,
        })
        .unwrap();
    assert_eq!(store.db().folder_active_project_count(&folder), 2);

    // Nested folders roll their live projects into the parent's badge.
    let nested = add_folder_in(&mut store, "Client", Some(&folder));
    let nested_live = add_project_in(&mut store, "Nested live", Some(&nested));
    let nested_task = add_task(&mut store, "Nested work");
    store
        .execute(Command::MoveTasks {
            ids: vec![nested_task],
            placement: TaskPlacement::Project {
                project: nested_live,
                position: Position::End,
            },
        })
        .unwrap();
    assert_eq!(store.db().folder_active_project_count(&folder), 3);
    assert_eq!(store.db().folder_active_project_count(&nested), 1);
    assert!(!store.db().project_is_ticked(&fresh));
}

#[test]
fn tag_location_edit_is_undoable() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let tag = store
        .execute(Command::CreateTag {
            name: "Errands".into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let tag_id = store.db().resolve_tag(&tag).unwrap();
    let geo = flowlist_core::model::Geo {
        lat: 12.9716,
        lon: 77.5946,
    };
    store
        .execute(Command::UpdateTag {
            id: tag_id.clone(),
            patch: flowlist_core::command::TagPatch {
                location: Some(Some(geo)),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(store.db().tags[&tag_id].location, Some(geo));

    store.undo().unwrap();
    assert_eq!(store.db().tags[&tag_id].location, None);
    store.redo().unwrap();
    assert_eq!(store.db().tags[&tag_id].location, Some(geo));
}

#[test]
fn moving_root_tag_under_parent_updates_both_lists() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let parent = store
        .execute(Command::CreateTag {
            name: "Parent".into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let parent_id = store.db().resolve_tag(&parent).unwrap();
    let child = store
        .execute(Command::CreateTag {
            name: "Child".into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let child_id = store.db().resolve_tag(&child).unwrap();

    store
        .execute(Command::UpdateTag {
            id: child_id.clone(),
            patch: flowlist_core::command::TagPatch {
                parent: Some(Some(parent_id.clone())),
                ..Default::default()
            },
        })
        .unwrap();

    assert_eq!(store.db().tags[&child_id].parent, Some(parent_id.clone()));
    assert!(store.db().tags[&parent_id].child_order.contains(&child_id));
    assert!(store.db().tag_roots.contains(&parent_id));
    assert!(!store.db().tag_roots.contains(&child_id));
    store.db().validate().unwrap();
}

#[test]
fn trashed_tasks_stay_out_of_every_other_perspective() {
    use flowlist_core::model::{BuiltinPerspective, Priority};
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let project = store
        .execute(Command::CreateProject(NewProject::new("Work")))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&project).unwrap();
    let tag = store
        .execute(Command::CreateTag {
            name: "Office".into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            position: Position::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let tag_id = store.db().resolve_tag(&tag).unwrap();

    let id = add(&mut store, "Throw me away");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::MoveTasks {
            ids: vec![task_id.clone()],
            placement: TaskPlacement::Project {
                project: project_id.clone(),
                position: Position::End,
            },
        })
        .unwrap();
    store
        .execute(Command::SetTaskTags {
            ids: vec![task_id.clone()],
            add: vec![tag_id.clone()],
            remove: Vec::new(),
        })
        .unwrap();
    store
        .execute(Command::UpdateTask {
            id: task_id.clone(),
            patch: TaskPatch {
                flagged: Some(true),
                priority: Some(Some(Priority::High)),
                due: Some(Some(Stamp::date_only(local_date(Utc::now())))),
                ..Default::default()
            },
        })
        .unwrap();

    store
        .execute(Command::DeleteTasks {
            ids: vec![task_id.clone()],
        })
        .unwrap();

    let ids = |groups: Vec<query::Group>| {
        groups
            .into_iter()
            .flat_map(|group| group.rows)
            .map(|row| row.id)
            .collect::<Vec<_>>()
    };
    let at = Utc::now();
    for perspective in BuiltinPerspective::ALL {
        if perspective == BuiltinPerspective::Trash {
            continue;
        }
        assert!(
            !ids(query::view(store.db(), perspective, at)).contains(&task_id),
            "{perspective:?} lists a trashed task"
        );
    }
    assert!(!ids(query::priority(store.db(), Priority::High, at)).contains(&task_id));
    assert!(
        !store
            .db()
            .project_outline(&project_id, true)
            .iter()
            .any(|row| row.task == task_id)
    );
    assert!(!store.db().tasks_tagged(&tag_id, true).contains(&task_id));
    assert_eq!(
        ids(query::view(store.db(), BuiltinPerspective::Trash, at)),
        vec![task_id]
    );
}

#[test]
fn deleting_moves_to_trash_and_restores_or_purges() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let id = add(&mut store, "Buy milk");
    let task_id = store.db().resolve_task(&id).unwrap();

    store
        .execute(Command::DeleteTasks {
            ids: vec![task_id.clone()],
        })
        .unwrap();
    assert!(store.db().tasks[&task_id].is_trashed());
    assert!(store.db().trashed_roots().contains(&task_id));
    let inbox = query::view(store.db(), BuiltinPerspective::Inbox, Utc::now());
    assert!(
        inbox
            .iter()
            .flat_map(|group| &group.rows)
            .all(|row| row.id != task_id)
    );

    store
        .execute(Command::RestoreTasks {
            ids: vec![task_id.clone()],
        })
        .unwrap();
    assert!(!store.db().tasks[&task_id].is_trashed());
    assert!(!store.db().trashed_roots().contains(&task_id));

    store
        .execute(Command::PurgeTasks {
            ids: vec![task_id.clone()],
        })
        .unwrap();
    assert!(!store.db().tasks.contains_key(&task_id));

    store.undo().unwrap();
    assert!(store.db().tasks.contains_key(&task_id));
    assert!(!store.db().tasks[&task_id].is_trashed());

    store
        .execute(Command::DeleteTasks {
            ids: vec![task_id.clone()],
        })
        .unwrap();

    // A cutoff before the delete stamp keeps the task; a cutoff after it
    // removes the task permanently.
    store
        .execute(Command::PurgeExpiredTrash {
            before: Utc::now() - chrono::Duration::days(1),
        })
        .unwrap();
    assert!(store.db().tasks.contains_key(&task_id));
    store
        .execute(Command::PurgeExpiredTrash {
            before: Utc::now() + chrono::Duration::days(1),
        })
        .unwrap();
    assert!(!store.db().tasks.contains_key(&task_id));
}

#[test]
fn priorities_filter_and_route_to_the_sidebar_query() {
    use flowlist_core::model::{Condition, FilterNode, GroupBy, Priority, SortBy};
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let mut high = NewTask::new("Ship the release");
    high.priority = Some(Priority::High);
    let high_id = store.execute(Command::CreateTask(high)).unwrap().created[0].clone();
    let mut low = NewTask::new("Tidy the desk");
    low.priority = Some(Priority::Low);
    let low_id = store.execute(Command::CreateTask(low)).unwrap().created[0].clone();

    let at = Utc::now();
    let high_groups = query::priority(store.db(), Priority::High, at);
    let ids: Vec<String> = high_groups
        .iter()
        .flat_map(|group| &group.rows)
        .map(|row| row.id.to_string())
        .collect();
    assert_eq!(ids, vec![high_id.clone()]);
    assert!(!ids.contains(&low_id));

    let filter = FilterNode::condition(Condition::Priority {
        value: Priority::Low,
    });
    let groups = query::run(
        store.db(),
        &filter,
        SortBy::Manual,
        GroupBy::None,
        false,
        at,
    );
    let ids: Vec<String> = groups
        .iter()
        .flat_map(|group| &group.rows)
        .map(|row| row.id.to_string())
        .collect();
    assert_eq!(ids, vec![low_id]);
}

#[test]
fn each_project_keeps_its_own_review_cadence() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());

    let mut weekly = NewProject::new("Weekly");
    weekly.review_interval_days = Some(7);
    let weekly = store
        .execute(Command::CreateProject(weekly))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("created id");
    let weekly = store.db().resolve_project(&weekly).unwrap();

    let mut monthly = NewProject::new("Monthly");
    monthly.review_interval_days = Some(30);
    let monthly = store
        .execute(Command::CreateProject(monthly))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("created id");
    let monthly = store.db().resolve_project(&monthly).unwrap();

    assert_eq!(store.db().projects[&weekly].review_interval_days, 7);
    assert_eq!(store.db().projects[&monthly].review_interval_days, 30);
    assert_eq!(
        store.db().projects[&weekly].next_review,
        today + chrono::Duration::days(7)
    );
    assert_eq!(
        store.db().projects[&monthly].next_review,
        today + chrono::Duration::days(30)
    );

    // Changing one project's cadence leaves the other alone.
    store
        .execute(Command::UpdateProject {
            id: weekly.clone(),
            patch: ProjectPatch {
                review_interval_days: Some(14),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(store.db().projects[&weekly].review_interval_days, 14);
    assert_eq!(store.db().projects[&monthly].review_interval_days, 30);
}

#[test]
fn project_kinds_round_trip_and_legacy_sequential_still_blocks() {
    use flowlist_core::model::{ProjectKind, ProjectStatus};

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let mut new_project = NewProject::new("Launch");
    new_project.kind = ProjectKind::Sequential;
    let outcome = store.execute(Command::CreateProject(new_project)).unwrap();
    let project_id = store.db().resolve_project(&outcome.created[0]).unwrap();
    let project = &store.db().projects[&project_id];
    assert_eq!(project.effective_kind(), ProjectKind::Sequential);
    assert!(project.is_sequential());

    store
        .execute(Command::UpdateProject {
            id: project_id.clone(),
            patch: ProjectPatch {
                kind: Some(ProjectKind::SingleActions),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(
        store.db().projects[&project_id].effective_kind(),
        ProjectKind::SingleActions
    );
    assert!(!store.db().projects[&project_id].is_sequential());
    assert_eq!(
        store.db().projects[&project_id].status,
        ProjectStatus::Active
    );
}

#[test]
fn tasks_on_date_uses_the_local_due_date() {
    use chrono::Duration;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let mut task = NewTask::new("Dentist");
    task.due = Some(Stamp::date_only(today));
    let due_id = store.execute(Command::CreateTask(task)).unwrap().created[0].clone();
    let mut later = NewTask::new("Later");
    later.defer = Some(Stamp::date_only(today + Duration::days(3)));
    let later_id = store.execute(Command::CreateTask(later)).unwrap().created[0].clone();

    let ids: Vec<String> = query::tasks_on_date(store.db(), today, Utc::now())
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(ids, vec![due_id]);
    assert!(!ids.contains(&later_id));
}

#[test]
fn attachments_and_project_icons_round_trip() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let id = add(&mut store, "Read the brief");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::UpdateTask {
            id: task_id.clone(),
            patch: TaskPatch {
                attachments: Some(vec![
                    "/tmp/brief.pdf".to_string(),
                    "/tmp/sketch.png".to_string(),
                ]),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(store.db().tasks[&task_id].attachments.len(), 2);

    let mut project = NewProject::new("Launch");
    assert_eq!(
        project.icon,
        flowlist_core::model::ProjectIcon::Project,
        "a new project starts with the project icon"
    );
    project.icon = flowlist_core::model::ProjectIcon::Rocket;
    project.attachments = vec!["/tmp/spec.pdf".to_string()];
    let created = store
        .execute(Command::CreateProject(project))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&created).unwrap();
    assert_eq!(
        store.db().projects[&project_id].icon,
        flowlist_core::model::ProjectIcon::Rocket
    );
    assert_eq!(
        store.db().projects[&project_id].attachments,
        vec!["/tmp/spec.pdf".to_string()]
    );

    store
        .execute(Command::UpdateProject {
            id: project_id.clone(),
            patch: ProjectPatch {
                icon: Some(flowlist_core::model::ProjectIcon::Star),
                attachments: Some(vec![
                    " /tmp/spec.pdf ".to_string(),
                    String::new(),
                    "/tmp/mock.png".to_string(),
                ]),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(
        store.db().projects[&project_id].icon,
        flowlist_core::model::ProjectIcon::Star
    );
    assert_eq!(
        store.db().projects[&project_id].attachments,
        vec!["/tmp/spec.pdf".to_string(), "/tmp/mock.png".to_string()]
    );
}

#[test]
fn convert_task_to_project_is_one_undo_step() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut new_parent = NewTask::new("Launch the site");
    new_parent.attachments = vec!["/tmp/brief.pdf".to_string()];
    let parent = store
        .execute(Command::CreateTask(new_parent))
        .unwrap()
        .created[0]
        .clone();
    let parent_id = store.db().resolve_task(&parent).unwrap();
    for title in ["Write copy", "Design hero"] {
        let mut child = NewTask::new(title);
        child.parent = Some(parent_id.clone());
        store.execute(Command::CreateTask(child)).unwrap();
    }
    store
        .execute(Command::AddChecklistItem {
            task: parent_id.clone(),
            title: "Book the launch slot".to_string(),
            position: None,
        })
        .unwrap();
    let children = store.db().tasks[&parent_id].child_order.clone();
    assert_eq!(children.len(), 2);

    let outcome = store
        .execute(Command::ConvertTaskToProject {
            id: parent_id.clone(),
            folder: None,
        })
        .unwrap();
    let project_id = store.db().resolve_project(&outcome.created[0]).unwrap();
    assert!(!store.db().tasks.contains_key(&parent_id));
    assert_eq!(
        store.db().projects[&project_id].icon,
        flowlist_core::model::ProjectIcon::Project,
        "converting a task keeps the default project icon"
    );
    // Two subtasks plus the checklist item, which became a task.
    assert_eq!(store.db().projects[&project_id].task_order.len(), 3);
    assert_eq!(
        store.db().projects[&project_id].attachments,
        vec!["/tmp/brief.pdf".to_string()]
    );
    for child in &children {
        assert_eq!(store.db().tasks[child].project, Some(project_id.clone()));
        assert_eq!(store.db().tasks[child].parent, None);
    }
    let checklist_task = store.db().projects[&project_id].task_order[2].clone();
    assert_eq!(
        store.db().tasks[&checklist_task].title,
        "Book the launch slot"
    );

    store.undo().unwrap();
    assert!(store.db().tasks.contains_key(&parent_id));
    assert!(!store.db().projects.contains_key(&project_id));
    for child in &children {
        assert_eq!(store.db().tasks[child].parent, Some(parent_id.clone()));
    }
}

#[test]
fn changed_perspective_lists_recent_edits() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let id = add(&mut store, "Fresh task");
    let task_id = store.db().resolve_task(&id).unwrap();
    let groups = flowlist_core::query::view(
        store.db(),
        flowlist_core::model::BuiltinPerspective::Changed,
        Utc::now(),
    );
    let rows: Vec<_> = groups.iter().flat_map(|group| &group.rows).collect();
    assert!(rows.iter().any(|row| row.id == task_id));
}

#[test]
fn archive_lists_only_finished_projects() {
    use flowlist_core::ids::ProjectId;
    use flowlist_core::model::{BuiltinPerspective, ProjectStatus};

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let create = |store: &mut Store, name: &str| -> ProjectId {
        let outcome = store
            .execute(Command::CreateProject(NewProject::new(name)))
            .unwrap();
        store.db().resolve_project(&outcome.created[0]).unwrap()
    };

    let active = create(&mut store, "Active");
    let done = create(&mut store, "Done");
    let dropped = create(&mut store, "Dropped");
    for (id, status) in [
        (done.clone(), ProjectStatus::Done),
        (dropped.clone(), ProjectStatus::Dropped),
    ] {
        store
            .execute(Command::SetProjectStatus { id, status })
            .unwrap();
    }
    let mut task = NewTask::new("Finish the write-up");
    task.project = Some(done.clone());
    let task_id = store.execute(Command::CreateTask(task)).unwrap().created[0].clone();
    let task_id = store.db().resolve_task(&task_id).unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![task_id.clone()],
            completed: true,
        })
        .unwrap();

    let archived = flowlist_core::query::archived_projects(store.db());
    assert_eq!(archived, vec![done.clone(), dropped.clone()]);
    assert!(!archived.contains(&active));

    // The shared view answers in tasks, including the completed ones, so the
    // CLI and MCP server can read the same archive.
    let groups = flowlist_core::query::view(store.db(), BuiltinPerspective::Archive, Utc::now());
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].title, "Done");
    assert!(groups[0].rows.iter().any(|row| row.id == task_id));

    // Reopening a project takes it out of the archive and is undoable.
    store
        .execute(Command::SetProjectStatus {
            id: done.clone(),
            status: ProjectStatus::Active,
        })
        .unwrap();
    assert_eq!(
        flowlist_core::query::archived_projects(store.db()),
        vec![dropped]
    );
    store.undo().unwrap();
    assert!(flowlist_core::query::archived_projects(store.db()).contains(&done));
}

#[test]
fn history_labels_use_entity_names() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let id = add(&mut store, "Plan the launch");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::UpdateTask {
            id: task_id.clone(),
            patch: TaskPatch {
                notes: Some("Bring the deck".into()),
                ..Default::default()
            },
        })
        .unwrap();

    assert_eq!(
        store.undo_label().as_deref(),
        Some("Undo Edit task “Plan the launch”")
    );
    assert_eq!(store.redo_label(), None);

    store.undo().unwrap();
    assert_eq!(
        store.redo_label().as_deref(),
        Some("Redo Edit task “Plan the launch”")
    );

    // The no-database form used by other callers never leaks an id either.
    let label = Command::UpdateTask {
        id: task_id,
        patch: TaskPatch::default(),
    }
    .label();
    assert_eq!(label, "Edit task");
}

#[test]
fn assigning_an_unknown_project_path_creates_the_project() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let id = add(&mut store, "Draft the brief");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::UpdateTask {
            id: task_id.clone(),
            patch: TaskPatch {
                project_path: Some(Some("Work / Client launch".into())),
                ..Default::default()
            },
        })
        .unwrap();

    let project = store
        .db()
        .find_project_by_path("Work / Client launch")
        .expect("typing a new path creates the project");
    assert_eq!(store.db().tasks[&task_id].project, Some(project.clone()));
    assert_eq!(
        store.undo_label().as_deref(),
        Some("Undo Edit task “Draft the brief”")
    );

    // One undo removes the assignment and the project it created together.
    store.undo().unwrap();
    assert_eq!(store.db().tasks[&task_id].project, None);
    assert!(!store.db().projects.contains_key(&project));
}

#[test]
fn deleting_a_project_trashes_its_tasks_and_undo_restores_both() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let project = store
        .execute(Command::CreateProject(NewProject::new("Launch")))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("project id");
    let project_id = store.db().resolve_project(&project).unwrap();

    let mut task = NewTask::new("Draft the brief");
    task.project = Some(project_id.clone());
    let task = store
        .execute(Command::CreateTask(task))
        .unwrap()
        .created
        .first()
        .cloned()
        .expect("task id");
    let task_id = store.db().resolve_task(&task).unwrap();

    store
        .execute(Command::DeleteProject {
            id: project_id.clone(),
        })
        .unwrap();

    assert!(!store.db().projects.contains_key(&project_id));
    assert!(store.db().tasks[&task_id].is_trashed());
    assert_eq!(
        store.undo_label().as_deref(),
        Some("Undo Delete project “Launch”")
    );

    store.undo().unwrap();
    assert!(store.db().projects.contains_key(&project_id));
    assert!(!store.db().tasks[&task_id].is_trashed());
    assert_eq!(
        store.db().tasks[&task_id].project.as_ref(),
        Some(&project_id)
    );
}

#[test]
fn dragging_a_project_into_a_folder_and_back_is_undoable() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let project_id = store
        .execute(Command::CreateProject(NewProject::new("Launch")))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&project_id).unwrap();
    let folder_id = store
        .execute(Command::CreateFolder {
            name: "Work".into(),
            parent: None,
            position: ContainerPosition::End,
        })
        .unwrap()
        .created[0]
        .clone();
    let folder_id = store.db().resolve_folder(&folder_id).unwrap();
    assert!(store.db().projects[&project_id].folder.is_none());

    store
        .execute(Command::MoveProject {
            id: project_id.clone(),
            placement: ContainerPlacement {
                parent: Some(folder_id.clone()),
                position: ContainerPosition::End,
            },
        })
        .unwrap();
    assert_eq!(
        store.db().projects[&project_id].folder.as_ref(),
        Some(&folder_id)
    );
    assert!(store.db().folders[&folder_id].child_order.contains(
        &flowlist_core::ids::ContainerId::Project(project_id.clone())
    ));
    assert!(
        !store
            .db()
            .root
            .contains(&flowlist_core::ids::ContainerId::Project(
                project_id.clone()
            ))
    );

    // Undo puts the project back at the Library root.
    store.undo().unwrap();
    assert!(store.db().projects[&project_id].folder.is_none());
    assert!(
        store
            .db()
            .root
            .contains(&flowlist_core::ids::ContainerId::Project(
                project_id.clone()
            ))
    );

    // Redo files it again, so dropping on the LIBRARY heading has something to
    // move out.
    store.redo().unwrap();
    assert_eq!(
        store.db().projects[&project_id].folder.as_ref(),
        Some(&folder_id)
    );

    store
        .execute(Command::MoveProject {
            id: project_id.clone(),
            placement: ContainerPlacement {
                parent: None,
                position: ContainerPosition::End,
            },
        })
        .unwrap();
    assert!(store.db().projects[&project_id].folder.is_none());
    assert!(
        store
            .db()
            .root
            .contains(&flowlist_core::ids::ContainerId::Project(
                project_id.clone()
            ))
    );

    store.undo().unwrap();
    assert_eq!(
        store.db().projects[&project_id].folder.as_ref(),
        Some(&folder_id)
    );
}

#[test]
fn changed_perspective_hides_completed_tasks() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let open = add(&mut store, "Still open");
    let done = add(&mut store, "Already done");
    let open_id = store.db().resolve_task(&open).unwrap();
    let done_id = store.db().resolve_task(&done).unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![done_id.clone()],
            completed: true,
        })
        .unwrap();

    let groups = query::view(store.db(), BuiltinPerspective::Changed, Utc::now());
    let ids: Vec<String> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.to_string()))
        .collect();
    assert_eq!(ids, vec![open_id.to_string()]);

    // Undoing completion brings it back into Changed.
    store.undo().unwrap();
    let groups = query::view(store.db(), BuiltinPerspective::Changed, Utc::now());
    let ids: Vec<String> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.to_string()))
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&done_id.to_string()));
}

#[test]
fn today_gathers_overdue_and_the_current_day() {
    use chrono::Duration;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));

    let mut overdue = NewTask::new("Overdue report");
    overdue.due = Some(day(-2));
    let overdue_id = store.execute(Command::CreateTask(overdue)).unwrap().created[0].clone();
    let overdue_id = store.db().resolve_task(&overdue_id).unwrap();

    let mut due_today = NewTask::new("Due today");
    due_today.due = Some(day(0));
    let due_id = store
        .execute(Command::CreateTask(due_today))
        .unwrap()
        .created[0]
        .clone();
    let due_id = store.db().resolve_task(&due_id).unwrap();

    let mut tomorrow = NewTask::new("Tomorrow");
    tomorrow.due = Some(day(1));
    let tomorrow_id = store
        .execute(Command::CreateTask(tomorrow))
        .unwrap()
        .created[0]
        .clone();
    let tomorrow_id = store.db().resolve_task(&tomorrow_id).unwrap();

    let mut deferred = NewTask::new("Deferred until today");
    deferred.defer = Some(day(0));
    let deferred_id = store
        .execute(Command::CreateTask(deferred))
        .unwrap()
        .created[0]
        .clone();
    let deferred_id = store.db().resolve_task(&deferred_id).unwrap();

    let mut later = NewTask::new("Deferred until later");
    later.defer = Some(day(3));
    let later_id = store.execute(Command::CreateTask(later)).unwrap().created[0].clone();
    let later_id = store.db().resolve_task(&later_id).unwrap();

    let mut done = NewTask::new("Finished today");
    done.due = Some(day(0));
    let done_id = store.execute(Command::CreateTask(done)).unwrap().created[0].clone();
    let done_id = store.db().resolve_task(&done_id).unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![done_id.clone()],
            completed: true,
        })
        .unwrap();

    let at = Utc::now();
    let groups = query::today(store.db(), true, at);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].title, "Overdue");
    assert_eq!(groups[0].rows[0].id, overdue_id);
    assert_eq!(groups[1].title, "Today");
    let today_ids: Vec<flowlist_core::ids::TaskId> =
        groups[1].rows.iter().map(|row| row.id.clone()).collect();
    assert!(today_ids.contains(&due_id));
    assert!(today_ids.contains(&deferred_id));
    assert!(!today_ids.contains(&tomorrow_id));
    assert!(!today_ids.contains(&later_id));
    assert!(!today_ids.contains(&done_id));

    // The header toggle drops the overdue group but keeps the day.
    let hidden = query::today(store.db(), false, at);
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].title, "Today");

    // The shared built-in view includes overdue by default.
    let builtin = query::view(
        store.db(),
        flowlist_core::model::BuiltinPerspective::Today,
        at,
    );
    assert_eq!(builtin.len(), 2);
}

#[test]
fn attachment_links_sit_alongside_file_paths() {
    use flowlist_core::model::BuiltinPerspective;
    use flowlist_core::model::attachment;
    use flowlist_core::query;

    assert!(attachment::is_web_link("https://example.com/brief"));
    assert!(attachment::is_web_link("HTTP://example.com"));
    assert!(!attachment::is_web_link("/tmp/brief.pdf"));
    assert!(!attachment::is_web_link("https://"));
    assert_eq!(
        attachment::attachment_name("https://www.example.com/brief/"),
        "example.com/brief"
    );
    assert_eq!(attachment::attachment_name("/tmp/brief.pdf"), "brief.pdf");
    assert_eq!(
        attachment::normalize_web_link("example.com/brief"),
        Some("https://example.com/brief".to_string())
    );
    assert_eq!(
        attachment::normalize_web_link("https://example.com"),
        Some("https://example.com".to_string())
    );
    assert_eq!(attachment::normalize_web_link("two words"), None);
    assert_eq!(attachment::normalize_web_link("https://"), None);

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let id = add(&mut store, "Read the brief");
    let task_id = store.db().resolve_task(&id).unwrap();
    store
        .execute(Command::UpdateTask {
            id: task_id.clone(),
            patch: TaskPatch {
                attachments: Some(vec![
                    "/tmp/brief.pdf".into(),
                    "https://example.com/brief".into(),
                ]),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(
        store.db().tasks[&task_id].attachments,
        vec!["/tmp/brief.pdf", "https://example.com/brief"]
    );

    // The Attachments perspective counts links the same as files.
    let groups = query::view(store.db(), BuiltinPerspective::Attachments, Utc::now());
    let ids: Vec<_> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
        .collect();
    assert_eq!(ids, vec![task_id]);
}

#[test]
fn deleting_a_folder_takes_its_projects_and_tasks() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let work = add_folder_in(&mut store, "Work", None);
    let client = add_folder_in(&mut store, "Client", Some(&work));
    let bigco = add_folder_in(&mut store, "BigCo", Some(&client));
    let top = add_project_in(&mut store, "Top", Some(&work));
    let nested = add_project_in(&mut store, "Nested", Some(&bigco));
    let outside = add_project_in(&mut store, "Outside", None);
    assert_eq!(store.db().project_count(&work), 2);

    let mut nested_task = NewTask::new("Nested task");
    nested_task.project = Some(nested.clone());
    let nested_task = store
        .execute(Command::CreateTask(nested_task))
        .unwrap()
        .created[0]
        .clone();
    let nested_task = store.db().resolve_task(&nested_task).unwrap();
    let outside_task = add_task(&mut store, "Outside task");

    store
        .execute(Command::DeleteFolder { id: work.clone() })
        .unwrap();

    assert!(!store.db().folders.contains_key(&work));
    assert!(!store.db().folders.contains_key(&client));
    assert!(!store.db().folders.contains_key(&bigco));
    assert!(!store.db().projects.contains_key(&top));
    assert!(!store.db().projects.contains_key(&nested));
    assert!(store.db().projects.contains_key(&outside));
    assert!(store.db().tasks[&nested_task].is_trashed());
    assert!(!store.db().tasks[&outside_task].is_trashed());

    // One Undo brings back every folder, project and task together.
    store.undo().unwrap();
    assert!(store.db().folders.contains_key(&work));
    assert!(store.db().folders.contains_key(&client));
    assert!(store.db().folders.contains_key(&bigco));
    assert_eq!(store.db().project_count(&work), 2);
    assert!(!store.db().tasks[&nested_task].is_trashed());
    assert_eq!(store.db().projects[&nested].folder, Some(bigco));
}

#[test]
fn deleting_a_folder_without_projects_still_removes_its_tree() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let work = add_folder_in(&mut store, "Work", None);
    let client = add_folder_in(&mut store, "Client", Some(&work));
    store
        .execute(Command::DeleteFolder { id: work.clone() })
        .unwrap();
    assert!(store.db().folders.is_empty());
    assert!(store.db().root.is_empty());

    store.undo().unwrap();
    assert!(store.db().folders.contains_key(&work));
    assert!(store.db().folders.contains_key(&client));
    assert_eq!(store.db().folders[&client].parent, Some(work));
}

#[test]
fn json_null_in_a_patch_clears_the_field() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut task = NewTask::new("Clear the priority");
    task.priority = Some(Priority::High);
    let id = store
        .execute(Command::CreateTask(task))
        .unwrap()
        .created[0]
        .clone();
    let task_id = store.db().resolve_task(&id).unwrap();

    // A phone sends the patch over JSON, so an explicit null has to mean
    // "clear this field" rather than "leave it alone".
    let command: Command = serde_json::from_str(&format!(
        r#"{{"update_task":{{"id":"{id}","patch":{{"priority":null,"due":null}}}}}}"#
    ))
    .unwrap();
    store.execute(command).unwrap();

    assert_eq!(store.db().tasks[&task_id].priority, None);
    assert!(store.db().tasks[&task_id].due.is_none());
}

#[test]
fn series_priority_reaches_every_occurrence_and_future_spawns() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut new_task = NewTask::new("Water plants");
    new_task.due = Some(Stamp::date_only(local_date(Utc::now())));
    new_task.repeat = Some(RepeatRule::new(
        RepeatMode::DueAgain,
        1,
        IntervalUnit::Weeks,
    ));
    let root = store
        .execute(Command::CreateTask(new_task))
        .unwrap()
        .created[0]
        .clone();
    let root = store.db().resolve_task(&root).unwrap();

    store
        .execute(Command::SetTaskPriority {
            ids: vec![root.clone()],
            priority: Some(Priority::Low),
            series: true,
        })
        .unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![root.clone()],
            completed: true,
        })
        .unwrap();

    let first = store
        .db()
        .tasks
        .values()
        .find(|task| !task.completed)
        .expect("next occurrence");
    assert_eq!(first.priority, Some(Priority::Low));
    let first_id = first.id.clone();

    // Changing one occurrence alone must not rewrite the series default.
    store
        .execute(Command::SetTaskPriority {
            ids: vec![first_id.clone()],
            priority: Some(Priority::High),
            series: false,
        })
        .unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![first_id],
            completed: true,
        })
        .unwrap();

    let second = store
        .db()
        .tasks
        .values()
        .find(|task| !task.completed)
        .expect("second next occurrence");
    assert_eq!(second.priority, Some(Priority::Low));
    let second_id = second.id.clone();

    store
        .execute(Command::SetTaskPriority {
            ids: vec![second_id],
            priority: Some(Priority::Medium),
            series: true,
        })
        .unwrap();

    let series = store.db().series_task_ids(&root);
    assert_eq!(series.len(), 3);
    for id in series {
        assert_eq!(store.db().tasks[&id].priority, Some(Priority::Medium));
    }
}

#[test]
fn defer_after_due_is_rejected_on_create_and_update() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let today = local_date(Utc::now());
    let due = Stamp::date_only(today);
    let late_defer = Stamp::date_only(today + chrono::Duration::days(1));

    let mut impossible = NewTask::new("Impossible dates");
    impossible.due = Some(due);
    impossible.defer = Some(late_defer);
    assert!(store.execute(Command::CreateTask(impossible)).is_err());

    let mut valid = NewTask::new("Valid dates");
    valid.due = Some(due);
    valid.defer = Some(Stamp::date_only(today - chrono::Duration::days(1)));
    let id = store
        .execute(Command::CreateTask(valid))
        .unwrap()
        .created[0]
        .clone();
    let task_id = store.db().resolve_task(&id).unwrap();

    assert!(
        store
            .execute(Command::UpdateTask {
                id: task_id.clone(),
                patch: TaskPatch {
                    defer: Some(Some(late_defer)),
                    ..TaskPatch::default()
                },
            })
            .is_err()
    );
    // The rejected edit left the original defer date in place.
    assert_eq!(
        store.db().tasks[&task_id].defer,
        Some(Stamp::date_only(today - chrono::Duration::days(1)))
    );
}

