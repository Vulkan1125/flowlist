//! A TaskPaper-compatible outline format for quick bulk editing and import.
//!
//! Export shape:
//!
//! ```text
//! # Inbox
//! - Buy milk @due(2026-11-13) @tags(Errands/Store)
//!
//! # Folder: Work
//! # Project: Work/Website
//! - Redesign @due(2026-11-20 17:00) @estimate(1h 30m) @flag
//! 	- Draft copy
//! ```
//!
//! Import also understands plain TaskPaper project headers (`Project:`).

use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::ids::{ContainerId, FolderId, ProjectId, TagId, TaskId};
use crate::model::{RepeatRule, Task};
use crate::parse::dates::parse_stamp;
use crate::parse::duration::parse_duration;
use crate::parse::repeat::parse_repeat;
use crate::time::Stamp;
use crate::time::{format_duration, now};

pub fn export(db: &Db) -> String {
    let mut out = String::new();
    out.push_str("# Flowlist TaskPaper\n\n");
    out.push_str("# Inbox\n");
    for row in db.inbox_outline(true) {
        if let Some(task) = db.tasks.get(&row.task) {
            out.push_str(&task_line(db, task, row.depth));
        }
    }
    out.push('\n');
    for container in &db.root {
        write_container(db, container, 0, &mut out);
    }
    out
}

fn write_container(db: &Db, container: &ContainerId, depth: usize, out: &mut String) {
    let indent = "\t".repeat(depth);
    match container {
        ContainerId::Folder(id) => {
            let Some(folder) = db.folders.get(id) else {
                return;
            };
            out.push_str(&format!(
                "{indent}# Folder: {}\n",
                folder_slash_path(db, id)
            ));
            for child in &folder.child_order {
                write_container(db, child, depth + 1, out);
            }
            out.push('\n');
        }
        ContainerId::Project(id) => {
            let Some(_project) = db.projects.get(id) else {
                return;
            };
            out.push_str(&format!(
                "{indent}# Project: {}\n",
                project_slash_path(db, id)
            ));
            for row in db.project_outline(id, true) {
                if let Some(task) = db.tasks.get(&row.task) {
                    out.push_str(&task_line(db, task, row.depth));
                }
            }
            out.push('\n');
        }
    }
}

fn task_line(db: &Db, task: &Task, depth: u32) -> String {
    let mut line = format!("{}- {}", "\t".repeat(depth as usize + 1), task.title);
    if let Some(due) = task.due {
        line.push_str(&format!(" @due({})", due.to_input_string()));
    }
    if let Some(defer) = task.defer {
        line.push_str(&format!(" @defer({})", defer.to_input_string()));
    }
    if let Some(estimate) = task.estimate_min {
        line.push_str(&format!(" @estimate({})", format_duration(estimate)));
    }
    if task.flagged {
        line.push_str(" @flag");
    }
    if task.sequential {
        line.push_str(" @sequential");
    }
    if !task.tags.is_empty() {
        let paths: Vec<String> = task
            .tags
            .iter()
            .filter_map(|id| tag_slash_path(db, id))
            .collect();
        if !paths.is_empty() {
            line.push_str(&format!(" @tags({})", paths.join(", ")));
        }
    }
    if let Some(repeat) = &task.repeat {
        line.push_str(&format!(" @repeat({})", repeat.description()));
    }
    if let Some(completed) = task.completed_at {
        line.push_str(&format!(
            " @done({})",
            completed.with_timezone(&chrono::Local).format("%Y-%m-%d")
        ));
    }
    if !task.notes.is_empty() {
        line.push_str(&format!(" @note({})", escape_note(&task.notes)));
    }
    line.push('\n');
    line
}

fn folder_slash_path(db: &Db, folder: &FolderId) -> String {
    let mut parts = Vec::new();
    let mut current = Some(folder.clone());
    while let Some(id) = current {
        let Some(folder) = db.folders.get(&id) else {
            break;
        };
        parts.push(folder.name.clone());
        current = folder.parent.clone();
    }
    parts.reverse();
    parts.join("/")
}

fn project_slash_path(db: &Db, project: &ProjectId) -> String {
    let Some(project) = db.projects.get(project) else {
        return String::new();
    };
    match &project.folder {
        Some(folder) => format!("{}/{}", folder_slash_path(db, folder), project.name),
        None => project.name.clone(),
    }
}

fn tag_slash_path(db: &Db, tag: &TagId) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = Some(tag.clone());
    while let Some(id) = current {
        let tag = db.tags.get(&id)?;
        parts.push(tag.name.clone());
        current = tag.parent.clone();
    }
    parts.reverse();
    Some(parts.join("/"))
}

fn escape_note(notes: &str) -> String {
    notes.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape_note(notes: &str) -> String {
    let mut out = String::new();
    let mut chars = notes.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('\\') => {
                    chars.next();
                    out.push('\\');
                }
                _ => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[derive(Default)]
struct Attrs {
    due: Option<Stamp>,
    defer: Option<Stamp>,
    estimate: Option<u32>,
    flagged: bool,
    sequential: bool,
    tags: Vec<String>,
    repeat: Option<RepeatRule>,
    done: Option<Stamp>,
    note: String,
}

pub fn import(text: &str) -> Result<Db> {
    let mut db = Db::new();
    let mut context: Option<ProjectId> = None;
    let mut stack: Vec<(usize, TaskId)> = Vec::new();
    let reference = now();

    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(path) = trimmed.strip_prefix("# Folder:") {
            db.ensure_folder_path(path.trim())?;
            context = None;
            stack.clear();
            continue;
        }
        if let Some(path) = trimmed.strip_prefix("# Project:") {
            let id = db.ensure_project_path(path.trim())?;
            context = Some(id);
            stack.clear();
            continue;
        }
        if trimmed.eq_ignore_ascii_case("# inbox") {
            context = None;
            stack.clear();
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if !trimmed.starts_with("- ") && trimmed.ends_with(':') {
            let name = trimmed.trim_end_matches(':').trim();
            if !name.is_empty() {
                let id = db.ensure_project_path(name)?;
                context = Some(id);
                stack.clear();
            }
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- ") else {
            continue;
        };
        let indent = indent_level(raw);
        let (title, attrs) = parse_task_line(rest, reference)?;
        if title.trim().is_empty() {
            return Err(Error::Import("task has an empty title".into()));
        }

        let mut task = Task::new(title.trim().to_string());
        task.notes = attrs.note;
        task.due = attrs.due;
        task.defer = attrs.defer;
        task.estimate_min = attrs.estimate;
        task.flagged = attrs.flagged;
        task.sequential = attrs.sequential;
        task.repeat = attrs.repeat;
        if let Some(done) = attrs.done {
            task.completed = true;
            task.completed_at = Some(done.at);
        }
        for path in attrs.tags {
            let tag = db.ensure_tag_path(&path, true)?;
            task.tags.insert(tag);
        }

        while stack.last().is_some_and(|(level, _)| *level >= indent) {
            stack.pop();
        }
        let parent = stack.last().map(|(_, id)| id.clone());
        let project = match &parent {
            Some(parent) => db.tasks.get(parent).and_then(|task| task.project.clone()),
            None => context.clone(),
        };
        task.parent = parent.clone();
        task.project = project.clone();
        let id = task.id.clone();
        db.tasks.insert(id.clone(), task);

        match parent {
            Some(parent) => {
                if let Some(parent_task) = db.tasks.get_mut(&parent) {
                    parent_task.child_order.push(id.clone());
                }
            }
            None => match &project {
                Some(project_id) => {
                    if let Some(project) = db.projects.get_mut(project_id) {
                        project.task_order.push(id.clone());
                    }
                }
                None => db.inbox_order.push(id.clone()),
            },
        }
        stack.push((indent, id));
    }

    db.touch();
    db.validate()
        .map_err(|error| Error::Import(format!("imported outline is invalid: {error}")))?;
    Ok(db)
}

fn indent_level(line: &str) -> usize {
    let width: usize = line
        .chars()
        .take_while(|ch| *ch == '\t' || *ch == ' ')
        .map(|ch| if ch == '\t' { 2 } else { 1 })
        .sum();
    width / 2
}

fn parse_task_line(input: &str, reference: DateTime<Utc>) -> Result<(String, Attrs)> {
    let chars: Vec<char> = input.chars().collect();
    let mut title = String::new();
    let mut attrs = Attrs::default();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch != '@' {
            title.push(ch);
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut key = String::new();
        while j < chars.len()
            && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '-')
        {
            key.push(chars[j]);
            j += 1;
        }
        if key.is_empty() {
            title.push('@');
            i += 1;
            continue;
        }
        if j < chars.len() && chars[j] == '(' {
            j += 1;
            let mut value = String::new();
            let mut depth = 1;
            while j < chars.len() && depth > 0 {
                match chars[j] {
                    '(' => {
                        depth += 1;
                        value.push('(');
                    }
                    ')' => {
                        depth -= 1;
                        if depth > 0 {
                            value.push(')');
                        }
                    }
                    c => value.push(c),
                }
                j += 1;
            }
            match key.to_lowercase().as_str() {
                "due" => attrs.due = Some(parse_stamp(&value, reference)?),
                "defer" => attrs.defer = Some(parse_stamp(&value, reference)?),
                "estimate" | "est" => attrs.estimate = Some(parse_duration(&value)?),
                "tags" | "tag" => {
                    for path in value.split(',') {
                        let path = path.trim();
                        if !path.is_empty() {
                            attrs.tags.push(path.to_string());
                        }
                    }
                }
                "repeat" => attrs.repeat = Some(parse_repeat(&value, reference)?),
                "done" | "completed" => {
                    attrs.done = Some(parse_stamp(&value, reference)?);
                }
                "note" => attrs.note = unescape_note(&value),
                other => {
                    title.push_str(&format!("@{other}({value})"));
                }
            }
            i = j;
            continue;
        }
        match key.to_lowercase().as_str() {
            "flag" | "flagged" => attrs.flagged = true,
            "sequential" => attrs.sequential = true,
            other => {
                title.push('@');
                title.push_str(other);
            }
        }
        i = j;
    }
    Ok((title.trim().to_string(), attrs))
}

/// Ensure every task is reachable in the imported structure. `Db::validate`
/// already enforces this; the helper exists so tests can assert the shape.
pub fn outline_ids(db: &Db) -> BTreeSet<TaskId> {
    let mut ids = BTreeSet::new();
    for row in db.flatten_containers() {
        if let ContainerId::Project(project) = row.container {
            for task in db.project_outline(&project, true) {
                ids.insert(task.task);
            }
        }
    }
    for task in db.inbox_outline(true) {
        ids.insert(task.task);
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;

    #[test]
    fn roundtrips_outline_fields() {
        let mut db = Db::new();
        let project = db.add_project(Project::new("Work"));
        let mut task = Task::new("Redesign site");
        task.project = Some(project.clone());
        task.due = Some(Stamp::date_only(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
        ));
        task.estimate_min = Some(90);
        task.flagged = true;
        task.notes = "line one\nline two".to_string();
        let tag = db.ensure_tag_path("Office/Deep Work", true).unwrap();
        task.tags.insert(tag);
        let id = task.id.clone();
        db.tasks.insert(id.clone(), task);
        db.projects
            .get_mut(&project)
            .unwrap()
            .task_order
            .push(id.clone());
        db.validate().unwrap();

        let text = export(&db);
        let restored = import(&text).unwrap();
        let restored_task = restored
            .tasks
            .values()
            .find(|task| task.title == "Redesign site")
            .expect("task roundtrips");
        assert_eq!(restored_task.estimate_min, Some(90));
        assert!(restored_task.flagged);
        assert_eq!(restored_task.notes, "line one\nline two");
        assert_eq!(restored.tags.len(), 2);
        assert_eq!(restored.projects.len(), 1);
    }

    #[test]
    fn imports_plain_taskpaper() {
        let text = "Errands:\n\t- Buy milk\n\t- Post letter\n";
        let db = import(text).unwrap();
        assert_eq!(db.projects.len(), 1);
        assert_eq!(db.tasks.len(), 2);
        assert_eq!(db.inbox_order.len(), 0);
    }

    #[test]
    fn imports_nested_tasks() {
        let text = "# Project: Home\n- Clean\n\t- Kitchen\n\t\t- Oven\n";
        let db = import(text).unwrap();
        assert_eq!(db.tasks.len(), 3);
        let clean = db.tasks.values().find(|t| t.title == "Clean").unwrap();
        assert_eq!(clean.child_order.len(), 1);
        let kitchen = db.tasks.get(&clean.child_order[0]).unwrap();
        assert_eq!(kitchen.child_order.len(), 1);
    }
}
