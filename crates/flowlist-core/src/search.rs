//! Fuzzy-ish search across tasks, projects, folders, tags and perspectives.

use crate::db::Db;
use crate::ids::TaskId;
use crate::model::ProjectStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchKind {
    Task,
    Project,
    Folder,
    Tag,
    Perspective,
}

impl SearchKind {
    pub fn label(self) -> &'static str {
        match self {
            SearchKind::Task => "task",
            SearchKind::Project => "project",
            SearchKind::Folder => "folder",
            SearchKind::Tag => "tag",
            SearchKind::Perspective => "perspective",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub kind: SearchKind,
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub score: i64,
}

impl SearchHit {
    pub fn short_id(&self) -> &str {
        let end = self.id.len().min(8);
        &self.id[..end]
    }

    pub fn is_task(&self) -> bool {
        matches!(self.kind, SearchKind::Task)
    }
}

pub fn search(db: &Db, query: &str, limit: usize) -> Vec<SearchHit> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<SearchHit> = Vec::new();

    for task in db.tasks.values() {
        if task.is_trashed() {
            continue;
        }
        let title_score = score(&task.title, &needle) * 3;
        let notes_score = score(&task.notes, &needle);
        let checklist_score = task
            .checklist
            .iter()
            .map(|item| score(&item.title, &needle))
            .max()
            .unwrap_or(0);
        let tag_score = task
            .tags
            .iter()
            .filter_map(|id| db.tags.get(id))
            .map(|tag| score(&tag.name, &needle))
            .max()
            .unwrap_or(0);
        let total = title_score + notes_score + tag_score + checklist_score;
        if total == 0 {
            continue;
        }
        let location = match db.effective_project(&task.id) {
            Some(project) => db.project_path(&project),
            None => "Inbox".to_string(),
        };
        let status = if task.completed {
            "completed".to_string()
        } else if task.flagged {
            "flagged".to_string()
        } else {
            "open".to_string()
        };
        hits.push(SearchHit {
            kind: SearchKind::Task,
            id: task.id.to_string(),
            title: task.title.clone(),
            subtitle: format!("{location} · {status}"),
            score: total,
        });
    }

    for project in db.projects.values() {
        let total = score(&project.name, &needle) * 2 + score(&project.notes, &needle);
        if total > 0 {
            hits.push(SearchHit {
                kind: SearchKind::Project,
                id: project.id.to_string(),
                title: db.project_path(&project.id),
                subtitle: project.status.label().to_string(),
                score: total,
            });
        }
    }

    for folder in db.folders.values() {
        let total = score(&folder.name, &needle);
        if total > 0 {
            hits.push(SearchHit {
                kind: SearchKind::Folder,
                id: folder.id.to_string(),
                title: db.folder_path(&folder.id),
                subtitle: "folder".to_string(),
                score: total,
            });
        }
    }

    for tag in db.tags.values() {
        let total = score(&tag.name, &needle);
        if total > 0 {
            hits.push(SearchHit {
                kind: SearchKind::Tag,
                id: tag.id.to_string(),
                title: db.tag_path(&tag.id),
                subtitle: tag.status.label().to_lowercase(),
                score: total,
            });
        }
    }

    for perspective in db.perspectives.values() {
        let total = score(&perspective.name, &needle);
        if total > 0 {
            hits.push(SearchHit {
                kind: SearchKind::Perspective,
                id: perspective.id.to_string(),
                title: perspective.name.clone(),
                subtitle: "custom perspective".to_string(),
                score: total,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    hits.truncate(limit.max(1));
    hits
}

/// Search restricted to tasks, for the CLI's `flowlist list --search`.
pub fn task_ids(db: &Db, query: &str, limit: usize) -> Vec<TaskId> {
    search(db, query, limit)
        .into_iter()
        .filter(|hit| hit.is_task())
        .filter_map(|hit| db.tasks.keys().find(|id| id.as_str() == hit.id).cloned())
        .collect()
}

pub fn project_status_label(status: ProjectStatus) -> &'static str {
    status.label()
}

fn score(haystack: &str, needle: &str) -> i64 {
    if haystack.is_empty() {
        return 0;
    }
    let lower = haystack.to_lowercase();
    if lower == needle {
        100
    } else if lower.starts_with(needle) {
        80
    } else if lower.contains(needle) {
        60
    } else if is_subsequence(&lower, needle) {
        25
    } else {
        0
    }
}

/// "bmlk" matches "Buy milk" — helpful when typing quickly.
fn is_subsequence(haystack: &str, needle: &str) -> bool {
    let mut chars = haystack.chars();
    'outer: for wanted in needle.chars() {
        for candidate in chars.by_ref() {
            if candidate == wanted {
                continue 'outer;
            }
        }
        return false;
    }
    true
}
