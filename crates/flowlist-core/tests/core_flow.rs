//! End-to-end tests for the command, history and persistence layers.

use chrono::Utc;
use flowlist_core::command::{
    Command, ContainerPlacement, ImportMode, NewProject, NewTask, ProjectPatch, TaskPatch,
    TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Position};
use flowlist_core::ids::{FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{
    Condition, FilterNode, GroupBy, IconSpec, Priority, ProjectStatus, RepeatMode, RepeatRule,
    SortBy,
    TagStatus, Task,
};
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
            fullscreen_reminder: false,
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

    let ids: Vec<String> = query::tasks_on_date(store.db(), today, Utc::now(), true)
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
        IconSpec::builtin(flowlist_core::model::ProjectIcon::Project),
        "a new project starts with the project icon"
    );
    project.icon = IconSpec::builtin(flowlist_core::model::ProjectIcon::Rocket);
    project.attachments = vec!["/tmp/spec.pdf".to_string()];
    let created = store
        .execute(Command::CreateProject(project))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&created).unwrap();
    assert_eq!(
        store.db().projects[&project_id].icon,
        IconSpec::builtin(flowlist_core::model::ProjectIcon::Rocket)
    );
    assert_eq!(
        store.db().projects[&project_id].attachments,
        vec!["/tmp/spec.pdf".to_string()]
    );

    store
        .execute(Command::UpdateProject {
            id: project_id.clone(),
            patch: ProjectPatch {
                icon: Some(IconSpec::builtin(flowlist_core::model::ProjectIcon::Star)),
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
        IconSpec::builtin(flowlist_core::model::ProjectIcon::Star)
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
        IconSpec::builtin(flowlist_core::model::ProjectIcon::Project),
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
            fullscreen_reminder: false,
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
fn today_keeps_slipped_plans_separate_from_planned_today() {
    use chrono::Duration;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));
    let make = |store: &mut Store, title: &str, planned: Option<Stamp>, due: Option<Stamp>| {
        let mut task = NewTask::new(title);
        task.planned = planned;
        task.due = due;
        let id = store
            .execute(Command::CreateTask(task))
            .unwrap()
            .created[0]
            .clone();
        store.db().resolve_task(&id).unwrap()
    };

    let slipped = make(&mut store, "Slipped plan", Some(day(-2)), None);
    let planned_today = make(&mut store, "Plan today", Some(day(0)), Some(day(5)));
    let overdue = make(
        &mut store,
        "Overdue and planned",
        Some(day(-1)),
        Some(day(-1)),
    );

    let groups = query::today(store.db(), true, Utc::now());
    let titles: Vec<&str> = groups.iter().map(|group| group.title.as_str()).collect();
    assert_eq!(titles, ["Past plans", "Overdue", "Today"]);
    let past: Vec<TaskId> = groups[0].rows.iter().map(|row| row.id.clone()).collect();
    assert_eq!(past, [slipped]);
    let today_ids: Vec<TaskId> = groups[2].rows.iter().map(|row| row.id.clone()).collect();
    assert!(today_ids.contains(&planned_today));
    assert!(!today_ids.contains(&overdue));

    // One chip hides both carried-over groups.
    let hidden = query::today(store.db(), false, Utc::now());
    assert_eq!(hidden.len(), 1);
    assert_eq!(hidden[0].title, "Today");
}

#[test]
fn forecast_keeps_each_date_type_independent() {
    use chrono::Duration;
    use flowlist_core::query;
    use flowlist_core::query::forecast;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));
    let make = |store: &mut Store, title: &str, planned: Option<Stamp>, due: Option<Stamp>| {
        let mut task = NewTask::new(title);
        task.planned = planned;
        task.due = due;
        let id = store
            .execute(Command::CreateTask(task))
            .unwrap()
            .created[0]
            .clone();
        store.db().resolve_task(&id).unwrap()
    };

    let both = make(
        &mut store,
        "Planned today, due later",
        Some(day(0)),
        Some(day(5)),
    );
    let slipped = make(&mut store, "Slipped plan", Some(day(-1)), None);
    let future = make(&mut store, "Far plan", Some(day(9)), None);
    let near_future = make(&mut store, "Later this month", None, Some(day(20)));
    let far_future = make(&mut store, "Next month", None, Some(day(45)));

    let data = forecast::forecast(store.db(), Utc::now());
    assert!(data.planned_today.contains(&both));
    assert!(data
        .days
        .get(&(today + Duration::days(5)))
        .is_some_and(|day| day.due.contains(&both)));
    assert!(data.past_planned.contains(&slipped));
    assert!(data.later.contains(&future));
    assert!(data.later.contains(&near_future));
    assert!(!data.later.contains(&far_future));
    assert!(data.next_month.contains(&far_future));

    let on_planned = query::tasks_on_date(store.db(), today, Utc::now(), true);
    assert!(on_planned.contains(&both));
    let on_due = query::tasks_on_date(store.db(), today + Duration::days(5), Utc::now(), true);
    assert!(on_due.contains(&both));
}


#[test]
fn planned_dates_inherit_from_project_and_parent_with_overrides() {
    use chrono::Duration;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));

    let mut project = NewProject::new("Inherited plan");
    project.planned = Some(day(0));
    let project_id = store
        .execute(Command::CreateProject(project))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&project_id).unwrap();

    let mut from_project = NewTask::new("From project");
    from_project.project = Some(project_id.clone());
    let from_project_id = store
        .execute(Command::CreateTask(from_project))
        .unwrap()
        .created[0]
        .clone();
    let from_project_id = store.db().resolve_task(&from_project_id).unwrap();
    assert_eq!(
        store.db().effective_planned(&from_project_id),
        Some(day(0))
    );

    let mut own = NewTask::new("Own plan wins");
    own.project = Some(project_id.clone());
    own.planned = Some(day(2));
    let own_id = store
        .execute(Command::CreateTask(own))
        .unwrap()
        .created[0]
        .clone();
    let own_id = store.db().resolve_task(&own_id).unwrap();
    assert_eq!(store.db().effective_planned(&own_id), Some(day(2)));

    let mut parent = NewTask::new("Parent plan");
    parent.project = Some(project_id.clone());
    parent.planned = Some(day(1));
    let parent_id = store
        .execute(Command::CreateTask(parent))
        .unwrap()
        .created[0]
        .clone();
    let parent_id = store.db().resolve_task(&parent_id).unwrap();

    let mut child = NewTask::new("Child inherits parent");
    child.parent = Some(parent_id.clone());
    let child_id = store
        .execute(Command::CreateTask(child))
        .unwrap()
        .created[0]
        .clone();
    let child_id = store.db().resolve_task(&child_id).unwrap();
    assert_eq!(store.db().effective_planned(&child_id), Some(day(1)));

    let mut child_override = NewTask::new("Child overrides");
    child_override.parent = Some(parent_id.clone());
    child_override.planned = Some(day(3));
    let child_override_id = store
        .execute(Command::CreateTask(child_override))
        .unwrap()
        .created[0]
        .clone();
    let child_override_id = store.db().resolve_task(&child_override_id).unwrap();
    assert_eq!(
        store.db().effective_planned(&child_override_id),
        Some(day(3))
    );

    // Inbox stays unscheduled-only: a planned date never files a task away.
    let mut inbox_task = NewTask::new("Planned inbox task");
    inbox_task.planned = Some(day(0));
    let inbox_id = store
        .execute(Command::CreateTask(inbox_task))
        .unwrap()
        .created[0]
        .clone();
    let inbox_id = store.db().resolve_task(&inbox_id).unwrap();
    assert!(store
        .db()
        .inbox_unscheduled_outline(false)
        .iter()
        .any(|row| row.task == inbox_id));

    // Today sees the inherited plan.
    let groups = query::today(store.db(), true, Utc::now());
    let today_ids: Vec<TaskId> = groups
        .iter()
        .flat_map(|group| group.rows.iter().map(|row| row.id.clone()))
        .collect();
    assert!(today_ids.contains(&from_project_id));

    let forecast = flowlist_core::query::forecast::forecast(store.db(), Utc::now());
    assert!(forecast.planned_today.contains(&from_project_id));

    // Clearing the project plan removes the inherited date.
    store
        .execute(Command::UpdateProject {
            id: project_id.clone(),
            patch: ProjectPatch {
                planned: Some(None),
                ..Default::default()
            },
        })
        .unwrap();
    assert_eq!(store.db().effective_planned(&from_project_id), None);
    assert_eq!(store.db().effective_planned(&own_id), Some(day(2)));
}


#[test]
fn project_due_and_defer_constrain_tasks() {
    use chrono::Duration;
    use flowlist_core::query;
    use flowlist_core::query::availability;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));

    let mut project = NewProject::new("Project deadline");
    project.defer = Some(day(0));
    project.due = Some(day(4));
    let project_id = store
        .execute(Command::CreateProject(project))
        .unwrap()
        .created[0]
        .clone();
    let project_id = store.db().resolve_project(&project_id).unwrap();

    let mut plain = NewTask::new("Follows project");
    plain.project = Some(project_id.clone());
    let plain_id = store
        .execute(Command::CreateTask(plain))
        .unwrap()
        .created[0]
        .clone();
    let plain_id = store.db().resolve_task(&plain_id).unwrap();
    assert_eq!(store.db().effective_defer(&plain_id), Some(day(0)));
    assert_eq!(store.db().effective_due(&plain_id), Some(day(4)));

    // Earlier task dates win; later ones are clamped by the container.
    let mut early = NewTask::new("Earlier due");
    early.project = Some(project_id.clone());
    early.due = Some(day(2));
    let early_id = store
        .execute(Command::CreateTask(early))
        .unwrap()
        .created[0]
        .clone();
    let early_id = store.db().resolve_task(&early_id).unwrap();
    assert_eq!(store.db().effective_due(&early_id), Some(day(2)));

    // A task due later on the same day is not after its project's day.
    let mut same_day = NewTask::new("Same day, later time");
    same_day.project = Some(project_id.clone());
    same_day.due = Some(Stamp::on(
        (today + Duration::days(4)),
        chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
    ));
    let same_day_id = store
        .execute(Command::CreateTask(same_day))
        .unwrap()
        .created[0]
        .clone();
    let same_day_id = store.db().resolve_task(&same_day_id).unwrap();
    assert_eq!(
        store.db().tasks[&same_day_id]
            .due
            .map(|stamp| stamp.local_date()),
        Some(today + Duration::days(4))
    );

    // A task with a later due cannot be moved in silently.
    let mut late = NewTask::new("Later due");
    late.due = Some(day(6));
    let late_id = store
        .execute(Command::CreateTask(late))
        .unwrap()
        .created[0]
        .clone();
    let late_id = store.db().resolve_task(&late_id).unwrap();
    let placement = flowlist_core::command::TaskPlacement::Project {
        project: project_id.clone(),
        position: flowlist_core::db::Position::End,
    };
    let conflicts =
        flowlist_core::command::date_conflicts_for_move(store.db(), &[late_id.clone()], &placement);
    assert_eq!(conflicts.len(), 1);
    assert!(conflicts[0].due.is_some());
    assert!(conflicts[0]
        .container
        .as_ref()
        .is_some_and(|(name, parent)| name.contains("Project deadline") && !parent));
    assert!(store
        .execute(Command::MoveTasks {
            ids: vec![late_id.clone()],
            placement: placement.clone(),
        })
        .is_err());
    store
        .execute(Command::MoveTasksAdjusted {
            ids: vec![late_id.clone()],
            placement,
        })
        .unwrap();
    assert_eq!(store.db().tasks[&late_id].due, Some(day(4)));

    let mut later_defer = NewTask::new("Later defer");
    later_defer.project = Some(project_id.clone());
    later_defer.defer = Some(day(2));
    let later_defer_id = store
        .execute(Command::CreateTask(later_defer))
        .unwrap()
        .created[0]
        .clone();
    let later_defer_id = store.db().resolve_task(&later_defer_id).unwrap();
    assert_eq!(store.db().effective_defer(&later_defer_id), Some(day(2)));

    // Parent action groups constrain children too.
    let mut parent = NewTask::new("Parent due");
    parent.project = Some(project_id.clone());
    parent.due = Some(day(1));
    let parent_id = store
        .execute(Command::CreateTask(parent))
        .unwrap()
        .created[0]
        .clone();
    let parent_id = store.db().resolve_task(&parent_id).unwrap();
    let mut child = NewTask::new("Child follows parent");
    child.parent = Some(parent_id.clone());
    let child_id = store
        .execute(Command::CreateTask(child))
        .unwrap()
        .created[0]
        .clone();
    let child_id = store.db().resolve_task(&child_id).unwrap();
    assert_eq!(store.db().effective_due(&child_id), Some(day(1)));

    // The same preflight works for an inspector project assignment.
    let mut patch_late = NewTask::new("Patch late");
    patch_late.due = Some(day(6));
    let patch_late_id = store
        .execute(Command::CreateTask(patch_late))
        .unwrap()
        .created[0]
        .clone();
    let patch_late_id = store.db().resolve_task(&patch_late_id).unwrap();
    let patch = flowlist_core::command::TaskPatch {
        project: Some(Some(project_id.clone())),
        ..Default::default()
    };
    assert_eq!(
        flowlist_core::command::date_conflicts_for_task_patch(
            store.db(),
            &patch_late_id,
            &patch
        )
        .len(),
        1
    );
    store
        .execute(Command::UpdateTaskAdjusted {
            id: patch_late_id.clone(),
            patch,
        })
        .unwrap();
    assert_eq!(store.db().tasks[&patch_late_id].due, Some(day(4)));

    // Moving a project deadline earlier asks to clamp its tasks.
    let project_patch = flowlist_core::command::ProjectPatch {
        due: Some(Some(day(2))),
        ..Default::default()
    };
    let project_conflicts = flowlist_core::command::date_conflicts_for_project_patch(
        store.db(),
        &project_id,
        &project_patch,
    );
    assert!(!project_conflicts.is_empty());
    assert!(project_conflicts[0]
        .container
        .as_ref()
        .is_some_and(|(name, parent)| name.contains("Project deadline") && !parent));
    store
        .execute(Command::UpdateProjectAdjusted {
            id: project_id.clone(),
            patch: project_patch,
        })
        .unwrap();
    assert_eq!(store.db().tasks[&patch_late_id].due, Some(day(2)));
    // Put the deadline back for the remaining assertions.
    store
        .execute(Command::UpdateProject {
            id: project_id.clone(),
            patch: flowlist_core::command::ProjectPatch {
                due: Some(Some(day(4))),
                ..Default::default()
            },
        })
        .unwrap();

    // A project due today pulls its undated action into Today.
    let mut due_today_project = NewProject::new("Due today project");
    due_today_project.due = Some(day(0));
    let due_today_project_id = store
        .execute(Command::CreateProject(due_today_project))
        .unwrap()
        .created[0]
        .clone();
    let due_today_project_id = store
        .db()
        .resolve_project(&due_today_project_id)
        .unwrap();
    let mut due_today_task = NewTask::new("Due by project today");
    due_today_task.project = Some(due_today_project_id);
    let due_today_task_id = store
        .execute(Command::CreateTask(due_today_task))
        .unwrap()
        .created[0]
        .clone();
    let due_today_task_id = store.db().resolve_task(&due_today_task_id).unwrap();
    let groups = query::today(store.db(), true, Utc::now());
    assert!(groups
        .iter()
        .flat_map(|group| group.rows.iter())
        .any(|row| row.id == due_today_task_id));

    // Forecast uses the effective due date.
    let forecast = query::forecast::forecast(store.db(), Utc::now());
    assert!(forecast
        .days
        .get(&(today + Duration::days(4)))
        .is_some_and(|day| day.due.contains(&plain_id)));

    // A project defer date blocks availability, matching the project gate.
    let mut deferred_project = NewProject::new("Deferred project");
    deferred_project.defer = Some(day(1));
    let deferred_project_id = store
        .execute(Command::CreateProject(deferred_project))
        .unwrap()
        .created[0]
        .clone();
    let deferred_project_id = store.db().resolve_project(&deferred_project_id).unwrap();
    let mut blocked = NewTask::new("Blocked by project");
    blocked.project = Some(deferred_project_id);
    let blocked_id = store
        .execute(Command::CreateTask(blocked))
        .unwrap()
        .created[0]
        .clone();
    let blocked_id = store.db().resolve_task(&blocked_id).unwrap();
    assert!(!availability::availability(store.db(), &blocked_id, Utc::now()).is_available());
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

#[test]
fn converting_subtasks_flattens_them_into_one_off_steps() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let parent = add_task(&mut store, "Parent");
    let mut first = NewTask::new("First child");
    first.parent = Some(parent.clone());
    let first = store
        .execute(Command::CreateTask(first))
        .unwrap()
        .created[0]
        .clone();
    let first = store.db().resolve_task(&first).unwrap();
    let mut second = NewTask::new("Second child");
    second.parent = Some(parent.clone());
    let second = store
        .execute(Command::CreateTask(second))
        .unwrap()
        .created[0]
        .clone();
    let second = store.db().resolve_task(&second).unwrap();
    let mut nested = NewTask::new("Nested child");
    nested.parent = Some(second.clone());
    let nested = store
        .execute(Command::CreateTask(nested))
        .unwrap()
        .created[0]
        .clone();
    let nested = store.db().resolve_task(&nested).unwrap();

    store
        .execute(Command::CompleteTasks {
            ids: vec![first.clone()],
            completed: true,
        })
        .unwrap();

    store
        .execute(Command::ConvertSubtasksToOneOff {
            id: parent.clone(),
        })
        .unwrap();

    let task = &store.db().tasks[&parent];
    assert!(task.steps_one_off);
    assert!(!task.sequential);
    assert!(task.child_order.is_empty());
    assert_eq!(
        task.checklist
            .iter()
            .map(|item| (item.title.as_str(), item.completed))
            .collect::<Vec<_>>(),
        vec![("First child", true), ("Second child", false), ("Nested child", false)]
    );
    assert!(!store.db().tasks.contains_key(&first));
    assert!(!store.db().tasks.contains_key(&second));
    assert!(!store.db().tasks.contains_key(&nested));
}

#[test]
fn sequential_children_must_be_completed_in_order() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let mut parent = NewTask::new("Sequential parent");
    parent.sequential = true;
    let parent = store
        .execute(Command::CreateTask(parent))
        .unwrap()
        .created[0]
        .clone();
    let parent = store.db().resolve_task(&parent).unwrap();

    let mut first = NewTask::new("First");
    first.parent = Some(parent.clone());
    let first = store
        .execute(Command::CreateTask(first))
        .unwrap()
        .created[0]
        .clone();
    let first = store.db().resolve_task(&first).unwrap();
    let mut second = NewTask::new("Second");
    second.parent = Some(parent.clone());
    let second = store
        .execute(Command::CreateTask(second))
        .unwrap()
        .created[0]
        .clone();
    let second = store.db().resolve_task(&second).unwrap();

    let error = store
        .execute(Command::CompleteTasks {
            ids: vec![second.clone()],
            completed: true,
        })
        .unwrap_err();
    assert!(error.to_string().contains("Finish “First” first"));

    store
        .execute(Command::CompleteTasks {
            ids: vec![first.clone()],
            completed: true,
        })
        .unwrap();
    store
        .execute(Command::CompleteTasks {
            ids: vec![second.clone()],
            completed: true,
        })
        .unwrap();
    assert!(store.db().tasks[&second].completed);
}

#[test]
fn setting_step_mode_converts_in_one_command() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let parent = add_task(&mut store, "Parent");
    for title in ["First", "Second", "Third"] {
        let mut child = NewTask::new(title);
        child.parent = Some(parent.clone());
        store.execute(Command::CreateTask(child)).unwrap();
    }

    store
        .execute(Command::SetTaskStepMode {
            id: parent.clone(),
            sequential: false,
            steps_one_off: true,
        })
        .unwrap();
    let task = &store.db().tasks[&parent];
    assert!(task.steps_one_off);
    assert!(!task.sequential);
    assert!(task.child_order.is_empty());
    assert_eq!(
        task.checklist
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["First", "Second", "Third"]
    );

    store
        .execute(Command::SetTaskStepMode {
            id: parent.clone(),
            sequential: true,
            steps_one_off: false,
        })
        .unwrap();
    let task = &store.db().tasks[&parent];
    assert!(!task.steps_one_off);
    assert!(task.sequential);
    assert!(task.checklist.is_empty());
    assert_eq!(
        task.child_order
            .iter()
            .map(|id| store.db().tasks[id].title.as_str())
            .collect::<Vec<_>>(),
        vec!["First", "Second", "Third"]
    );
}

#[test]
fn converting_one_off_steps_back_to_subtasks() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let parent = add_task(&mut store, "Parent");
    store
        .execute(Command::AddChecklistItem {
            task: parent.clone(),
            title: "First".into(),
            position: None,
        })
        .unwrap();
    store
        .execute(Command::AddChecklistItem {
            task: parent.clone(),
            title: "Second".into(),
            position: None,
        })
        .unwrap();
    store
        .execute(Command::UpdateTask {
            id: parent.clone(),
            patch: TaskPatch {
                steps_one_off: Some(true),
                ..TaskPatch::default()
            },
        })
        .unwrap();

    store
        .execute(Command::ConvertOneOffToSubtasks { id: parent.clone() })
        .unwrap();

    let task = &store.db().tasks[&parent];
    assert!(!task.steps_one_off);
    assert!(task.checklist.is_empty());
    assert_eq!(
        task.child_order
            .iter()
            .map(|id| store.db().tasks[id].title.as_str())
            .collect::<Vec<_>>(),
        vec!["First", "Second"]
    );
}

#[test]
fn projectless_tasks_group_under_no_project() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    add_task(&mut store, "Loose task");

    let groups = flowlist_core::query::run(
        store.db(),
        &FilterNode::All(Vec::new()),
        SortBy::Manual,
        GroupBy::Project,
        false,
        Utc::now(),
    );
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].key, "inbox");
    assert_eq!(groups[0].title, "No project");
}

#[test]
fn every_project_icon_round_trips_by_slug() {
    use flowlist_core::model::ProjectIcon;
    let mut slugs = std::collections::BTreeSet::new();
    for icon in ProjectIcon::ALL {
        assert_eq!(ProjectIcon::from_slug(icon.slug()), Some(icon));
        assert!(slugs.insert(icon.slug()), "duplicate icon slug");
    }
    assert_eq!(ProjectIcon::ALL.len(), 54);
    // The retired running figure still resolves so old libraries load.
    assert_eq!(
        ProjectIcon::from_slug("running"),
        Some(ProjectIcon::Running)
    );
}

#[test]
fn fullscreen_reminders_follow_task_project_and_folder_marks() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();

    let folder = store
        .execute(Command::CreateFolder {
            name: "Focus".into(),
            parent: None,
            position: ContainerPosition::End,
            fullscreen_reminder: true,
        })
        .unwrap()
        .created
        .first()
        .cloned()
        .unwrap();
    let folder = store.db().resolve_folder(&folder).unwrap();
    let project = add_project_in(&mut store, "Launch", Some(&folder));

    let mut task = NewTask::new("Call Ana");
    task.project = Some(project.clone());
    let task = store
        .execute(Command::CreateTask(task))
        .unwrap()
        .created
        .first()
        .cloned()
        .unwrap();
    let task = store.db().resolve_task(&task).unwrap();

    assert!(store.db().wants_fullscreen_reminder(&task));

    store
        .execute(Command::UpdateTask {
            id: task.clone(),
            patch: TaskPatch {
                fullscreen_reminder: Some(true),
                ..Default::default()
            },
        })
        .unwrap();
    assert!(store.db().wants_fullscreen_reminder(&task));

    store
        .execute(Command::UpdateProject {
            id: project,
            patch: ProjectPatch {
                fullscreen_reminder: Some(true),
                ..Default::default()
            },
        })
        .unwrap();
    assert!(store.db().wants_fullscreen_reminder(&task));
}

#[test]
fn a_view_can_filter_on_fullscreen_reminder_inheritance() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let folder = store
        .execute(Command::CreateFolder {
            name: "Alerts".into(),
            parent: None,
            position: ContainerPosition::End,
            fullscreen_reminder: true,
        })
        .unwrap()
        .created
        .first()
        .cloned()
        .unwrap();
    let folder = store.db().resolve_folder(&folder).unwrap();
    let project = add_project_in(&mut store, "Ops", Some(&folder));
    let mut marked = NewTask::new("Inherited alert");
    marked.project = Some(project.clone());
    marked.due = Some(Stamp::date_only(
        local_date(Utc::now()) + chrono::Duration::days(1),
    ));
    let marked = store
        .execute(Command::CreateTask(marked))
        .unwrap()
        .created
        .first()
        .cloned()
        .unwrap();
    let marked = store.db().resolve_task(&marked).unwrap();
    let mut undated = NewTask::new("Undated alert");
    undated.project = Some(project);
    let undated = store
        .execute(Command::CreateTask(undated))
        .unwrap()
        .created
        .first()
        .cloned()
        .unwrap();
    let undated = store.db().resolve_task(&undated).unwrap();
    let plain = add_task(&mut store, "Quiet task");
    let at = Utc::now();

    let ids_for = |value: bool| {
        let filter = FilterNode::condition(Condition::FullscreenReminder { value });
        flowlist_core::query::run(
            store.db(),
            &filter,
            SortBy::Manual,
            GroupBy::None,
            false,
            at,
        )
        .into_iter()
        .flat_map(|group| group.rows)
        .map(|row| row.id)
        .collect::<Vec<_>>()
    };

    assert_eq!(ids_for(true), vec![marked.clone()]);
    let without = ids_for(false);
    assert!(without.contains(&plain));
    assert!(without.contains(&undated));
    assert!(!without.contains(&marked));
}

#[test]
fn fullscreen_reminder_can_follow_due_or_use_its_own_time() {
    use chrono::{NaiveDate, NaiveTime};

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let due_day = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
    let reminder_day = NaiveDate::from_ymd_opt(2026, 11, 30).unwrap();
    let task = add_task(&mut store, "Morning reminder");
    store
        .execute(Command::UpdateTask {
            id: task.clone(),
            patch: TaskPatch {
                due: Some(Some(Stamp::on(due_day, NaiveTime::from_hms_opt(9, 0, 0).unwrap()))),
                fullscreen_reminder: Some(true),
                ..Default::default()
            },
        })
        .unwrap();
    let due = store.db().reminder_stamp(&task).unwrap();
    assert_eq!(due.local_date(), due_day);

    store
        .execute(Command::UpdateTask {
            id: task.clone(),
            patch: TaskPatch {
                reminder_match_due: Some(false),
                reminder_at: Some(Some(Stamp::on(
                    reminder_day,
                    NaiveTime::from_hms_opt(7, 30, 0).unwrap(),
                ))),
                ..Default::default()
            },
        })
        .unwrap();
    let own = store.db().reminder_stamp(&task).unwrap();
    assert_eq!(own.local_date(), reminder_day);
    assert_eq!(own.local_time(), NaiveTime::from_hms_opt(7, 30, 0).unwrap());
}

#[test]
fn planned_dates_are_independent_of_availability() {
    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let id = add_task(&mut store, "Plan the launch");
    let today = local_date(Utc::now());
    let stamp = Stamp::date_only(today);

    store
        .execute(Command::UpdateTask {
            id: id.clone(),
            patch: TaskPatch {
                planned: Some(Some(stamp)),
                ..TaskPatch::default()
            },
        })
        .unwrap();

    let db = store.db();
    assert_eq!(db.tasks[&id].planned, Some(stamp));
    // A planned date is informational: the task is still available.
    assert!(flowlist_core::query::availability::availability(db, &id, Utc::now()).is_available());
    let forecast = flowlist_core::query::forecast::forecast(db, Utc::now());
    assert!(forecast.planned_today.contains(&id));

    // Sorts and groups can read it.
    let row = flowlist_core::query::row_for(db, &id, Utc::now());
    let mut rows = vec![row];
    flowlist_core::query::sort::sort_rows(db, &mut rows, SortBy::Planned);
    let groups = flowlist_core::query::regroup(db, rows, GroupBy::PlannedDate, Utc::now());
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].rows.len(), 1);

    // Clearing it removes the forecast entry.
    store
        .execute(Command::UpdateTask {
            id: id.clone(),
            patch: TaskPatch {
                planned: Some(None),
                ..TaskPatch::default()
            },
        })
        .unwrap();
    let forecast = flowlist_core::query::forecast::forecast(store.db(), Utc::now());
    assert!(!forecast.planned_today.contains(&id));
}

#[test]
fn forecast_puts_project_dates_above_the_tasks() {
    use chrono::Duration;
    use flowlist_core::query;

    let (_dir, paths) = setup();
    let mut store = Store::open(paths).unwrap();
    let today = local_date(Utc::now());
    let day = |offset: i64| Stamp::date_only(today + Duration::days(offset));

    let mut due = NewProject::new("Due today");
    due.due = Some(day(0));
    due.planned = Some(day(0));
    let due_id = store
        .execute(Command::CreateProject(due))
        .unwrap()
        .created[0]
        .clone();
    let due_id = store.db().resolve_project(&due_id).unwrap();

    let mut deferred = NewProject::new("Deferred tomorrow");
    deferred.defer = Some(day(1));
    let deferred_id = store
        .execute(Command::CreateProject(deferred))
        .unwrap()
        .created[0]
        .clone();
    let deferred_id = store.db().resolve_project(&deferred_id).unwrap();

    let mut held = NewProject::new("On hold tomorrow");
    held.due = Some(day(1));
    let held_id = store
        .execute(Command::CreateProject(held))
        .unwrap()
        .created[0]
        .clone();
    let held_id = store.db().resolve_project(&held_id).unwrap();
    store
        .execute(Command::SetProjectStatus {
            id: held_id.clone(),
            status: ProjectStatus::OnHold,
        })
        .unwrap();

    let mut done = NewProject::new("Done today");
    done.due = Some(day(0));
    let done_id = store
        .execute(Command::CreateProject(done))
        .unwrap()
        .created[0]
        .clone();
    let done_id = store.db().resolve_project(&done_id).unwrap();
    store
        .execute(Command::SetProjectStatus {
            id: done_id.clone(),
            status: ProjectStatus::Done,
        })
        .unwrap();

    let mut later = NewProject::new("Later");
    later.due = Some(day(20));
    let later_id = store
        .execute(Command::CreateProject(later))
        .unwrap()
        .created[0]
        .clone();
    let later_id = store.db().resolve_project(&later_id).unwrap();

    let mut next = NewProject::new("Next month");
    next.due = Some(day(40));
    let next_id = store
        .execute(Command::CreateProject(next))
        .unwrap()
        .created[0]
        .clone();
    let next_id = store.db().resolve_project(&next_id).unwrap();

    let mut past = NewProject::new("Past due");
    past.due = Some(day(-2));
    let past_id = store
        .execute(Command::CreateProject(past))
        .unwrap()
        .created[0]
        .clone();
    let past_id = store.db().resolve_project(&past_id).unwrap();

    let mut alpha_task = NewTask::new("Alpha task");
    alpha_task.project = Some(due_id.clone());
    alpha_task.due = Some(day(0));
    let alpha_task_id = store
        .execute(Command::CreateTask(alpha_task))
        .unwrap()
        .created[0]
        .clone();
    let alpha_task_id = store.db().resolve_task(&alpha_task_id).unwrap();
    let mut other_task = NewTask::new("Other task");
    other_task.due = Some(day(0));
    let other_task_id = store
        .execute(Command::CreateTask(other_task))
        .unwrap()
        .created[0]
        .clone();
    let other_task_id = store.db().resolve_task(&other_task_id).unwrap();

    let groups = query::forecast_view(store.db(), Utc::now(), true);
    let group = |key: &str| groups.iter().find(|group| group.key == key).unwrap();
    // Due and planned on the same day still make one project row.
    assert_eq!(
        group("forecast:today")
            .projects
            .iter()
            .filter(|id| **id == due_id)
            .count(),
        1
    );
    assert!(group("forecast:past").projects.contains(&past_id));
    assert!(group("forecast:tomorrow").projects.contains(&deferred_id));
    assert!(group("forecast:tomorrow").projects.contains(&held_id));
    assert!(!groups.iter().any(|group| group.projects.contains(&done_id)));
    assert!(group("forecast:later").projects.contains(&later_id));
    assert!(group("forecast:next-month").projects.contains(&next_id));
    assert!(query::projects_on_date(store.db(), today, true).contains(&due_id));
    // The task from the project above the section comes before an unrelated one.
    let today_rows = &group("forecast:today").rows;
    assert_eq!(today_rows[0].id, alpha_task_id);
    assert!(today_rows.iter().any(|row| row.id == other_task_id));
    assert!(query::projects_on_date(store.db(), today + Duration::days(1), true)
        .contains(&deferred_id));
    assert!(query::forecast_past_projects(store.db(), Utc::now(), true).contains(&past_id));
    assert!(query::forecast_future_projects(
        store.db(),
        today + Duration::days(7),
        true
    )
    .contains(&later_id));

    // The Planned toggle hides a planned-only project without hiding its due
    // siblings.
    let mut planned_only = NewProject::new("Planned only");
    planned_only.planned = Some(day(0));
    let planned_id = store
        .execute(Command::CreateProject(planned_only))
        .unwrap()
        .created[0]
        .clone();
    let planned_id = store.db().resolve_project(&planned_id).unwrap();
    let groups = query::forecast_view(store.db(), Utc::now(), true);
    assert!(groups
        .iter()
        .find(|group| group.key == "forecast:planned-today")
        .is_some_and(|group| group.projects.contains(&planned_id)));
    let groups = query::forecast_view(store.db(), Utc::now(), false);
    assert!(!groups.iter().any(|group| group.projects.contains(&planned_id)));
}
