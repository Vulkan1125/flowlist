//! JSON shapes shared by the CLI (`--json`) and the MCP server. Keeping one
//! renderer means scripts and agents see identical field names.

use serde_json::{Value, json};

use flowlist_core::db::Db;
use flowlist_core::ids::{ProjectId, TagId, TaskId};
use flowlist_core::model::ProjectStatus;
use flowlist_core::query::Group;
use flowlist_core::query::availability;
use flowlist_core::stats::Stats;
use flowlist_core::time::now;

pub fn task_value(db: &Db, id: &TaskId) -> Value {
    let Some(task) = db.tasks.get(id) else {
        return Value::Null;
    };
    let status = availability::availability(db, id, now());
    let project_path = db
        .effective_project(id)
        .map(|project| db.project_path(&project));
    let tags: Vec<String> = task
        .tags
        .iter()
        .filter_map(|tag| tag_path(db, tag))
        .collect();
    json!({
        "id": task.id.to_string(),
        "short_id": task.id.short(),
        "title": task.title,
        "notes": task.notes,
        "project": task.project.as_ref().map(ToString::to_string),
        "project_path": project_path,
        "parent": task.parent.as_ref().map(ToString::to_string),
        "tags": tags,
        "checklist": task.checklist.iter().map(|item| json!({
            "id": item.id.to_string(),
            "short_id": item.id.short(),
            "title": item.title,
            "completed": item.completed,
        })).collect::<Vec<_>>(),
        "checklist_completed": task.checklist_progress().0,
        "checklist_total": task.checklist_progress().1,
        "due": task.due.map(|stamp| stamp.to_input_string()),
        "due_at": task.due.map(|stamp| stamp.at.to_rfc3339()),
        "defer": task.defer.map(|stamp| stamp.to_input_string()),
        "defer_at": task.defer.map(|stamp| stamp.at.to_rfc3339()),
        "planned": task.planned.map(|stamp| stamp.to_input_string()),
        "planned_at": task.planned.map(|stamp| stamp.at.to_rfc3339()),
        "estimate_min": task.estimate_min,
        "flagged": task.flagged,
        "priority": task.priority.map(|priority| priority.label()),
        "priority_key": task.priority.map(|priority| priority.slug()),
        "trashed_at": task.trashed_at.map(|at| at.to_rfc3339()),
        "trashed": task.is_trashed(),
        "completed": task.completed,
        "completed_at": task.completed_at.map(|at| at.to_rfc3339()),
        "repeat": task.repeat.as_ref().map(|rule| rule.description()),
        "repeat_series": task.repeat_series.as_ref().map(|id| id.to_string()),
        "sequential": task.sequential,
        "attachments": task.attachments,
        "availability": status.label(),
        "availability_detail": status.detail(db),
        "depth": db.task_depth(id),
        "created_at": task.created_at.to_rfc3339(),
        "modified_at": task.modified_at.to_rfc3339(),
    })
}

pub fn groups_value(db: &Db, groups: &[Group]) -> Value {
    json!({
        "groups": groups.iter().map(|group| json!({
            "key": group.key,
            "title": group.title,
            "tasks": group.rows.iter().map(|row| {
                let mut value = task_value(db, &row.id);
                if let Some(object) = value.as_object_mut() {
                    object.insert("group".into(), json!(group.title));
                    object.insert("depth".into(), json!(row.depth));
                }
                value
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "count": groups.iter().map(|group| group.rows.len()).sum::<usize>(),
    })
}

pub fn project_value(db: &Db, id: &ProjectId) -> Value {
    let Some(project) = db.projects.get(id) else {
        return Value::Null;
    };
    let (completed, total) = db.project_progress(id);
    json!({
        "id": project.id.to_string(),
        "short_id": project.id.short(),
        "name": project.name,
        "path": db.project_path(id),
        "notes": project.notes,
        "status": project.status.label(),
        "status_key": status_key(project.status),
        "folder": project.folder.as_ref().map(ToString::to_string),
        "folder_path": project.folder.as_ref().map(|folder| db.folder_path(folder)),
        "tags": project.tags.iter().filter_map(|tag| tag_path(db, tag)).collect::<Vec<_>>(),
        "due": project.due.map(|stamp| stamp.to_input_string()),
        "defer": project.defer.map(|stamp| stamp.to_input_string()),
        "review_interval_days": project.review_interval_days,
        "next_review": project.next_review.format("%Y-%m-%d").to_string(),
        "review_due": project.is_due_for_review(flowlist_core::time::local_date(now())),
        "sequential": project.sequential,
        "kind": project.effective_kind().label(),
        "kind_key": project.effective_kind().slug(),
        "icon": project.icon.slug(),
        "attachments": project.attachments,
        "completed_tasks": completed,
        "total_tasks": total,
        "created_at": project.created_at.to_rfc3339(),
        "modified_at": project.modified_at.to_rfc3339(),
    })
}

pub fn project_values(db: &Db, ids: &[ProjectId]) -> Value {
    Value::Array(ids.iter().map(|id| project_value(db, id)).collect())
}

pub fn tag_value(db: &Db, id: &TagId) -> Value {
    let Some(tag) = db.tags.get(id) else {
        return Value::Null;
    };
    json!({
        "id": tag.id.to_string(),
        "short_id": tag.id.short(),
        "name": tag.name,
        "path": db.tag_path(id),
        "status": tag.status.label(),
        "status_key": tag_status_key(tag.status),
        "parent": tag.parent.as_ref().map(ToString::to_string),
        "location": tag.location.map(|geo| json!({"lat": geo.lat, "lon": geo.lon})),
        "usage": db.tag_usage_count(id, true),
    })
}

pub fn tag_values(db: &Db, ids: &[TagId]) -> Value {
    Value::Array(ids.iter().map(|id| tag_value(db, id)).collect())
}

pub fn stats_value(stats: &Stats) -> Value {
    json!({
        "total_tasks": stats.total_tasks,
        "open_tasks": stats.open_tasks,
        "completed_tasks": stats.completed_tasks,
        "inbox": stats.inbox,
        "available": stats.available,
        "blocked": stats.blocked,
        "deferred": stats.deferred,
        "flagged": stats.flagged,
        "due_today": stats.due_today,
        "overdue": stats.overdue,
        "completed_today": stats.completed_today,
        "active_projects": stats.active_projects,
        "on_hold_projects": stats.on_hold_projects,
        "projects_due_review": stats.projects_due_review,
        "tags": stats.tags,
        "folders": stats.folders,
        "trashed_tasks": stats.trashed_tasks,
    })
}

pub fn search_value(_db: &Db, hits: &[flowlist_core::search::SearchHit]) -> Value {
    json!({
        "hits": hits.iter().map(|hit| json!({
            "kind": hit.kind.label(),
            "id": hit.id,
            "short_id": hit.short_id(),
            "title": hit.title,
            "subtitle": hit.subtitle,
            "score": hit.score,
        })).collect::<Vec<_>>(),
        "count": hits.len(),
    })
}

pub fn tag_path(db: &Db, id: &TagId) -> Option<String> {
    db.tags.get(id).map(|_| db.tag_path(id))
}

pub fn status_key(status: ProjectStatus) -> &'static str {
    match status {
        ProjectStatus::Active => "active",
        ProjectStatus::OnHold => "on_hold",
        ProjectStatus::Done => "done",
        ProjectStatus::Dropped => "dropped",
    }
}

pub fn tag_status_key(status: flowlist_core::model::TagStatus) -> &'static str {
    use flowlist_core::model::TagStatus;
    match status {
        TagStatus::Available => "available",
        TagStatus::Blocked => "blocked",
        TagStatus::OnHold => "on_hold",
    }
}
