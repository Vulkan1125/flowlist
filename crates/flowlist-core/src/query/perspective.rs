//! Evaluating saved filters against tasks.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Months, NaiveDate, Utc};

use crate::db::Db;
use crate::ids::TagId;
use crate::model::{
    AvailabilityFilter, Condition, DateField, DateRangeFilter, DateUnit, DueWindow, FilterNode,
    FilterStatus, ItemKindFilter, Project, ProjectKind, ProjectStatus, ProjectStatusFilter,
    TagStatus, TagStatusFilter, Task, TaskStatus,
};
use crate::query::availability::{self, Availability};
use crate::time::{Stamp, local_date};

/// Evaluate a saved filter against a project. Custom perspectives can list
/// projects alongside actions, so every rule has a project meaning too.
pub fn matches_project(
    db: &Db,
    project: &Project,
    node: &FilterNode,
    at: DateTime<Utc>,
) -> bool {
    match node {
        FilterNode::All(nodes) => nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .all(|node| matches_project(db, project, node, at)),
        FilterNode::Any(nodes) => nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .any(|node| matches_project(db, project, node, at)),
        FilterNode::None(nodes) => !nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .any(|node| matches_project(db, project, node, at)),
        FilterNode::Not(inner) => !matches_project(db, project, inner, at),
        FilterNode::Condition(condition) => project_condition_matches(db, project, condition, at),
        FilterNode::Comment(_) | FilterNode::Disabled(_) => true,
    }
}

fn project_condition_matches(
    db: &Db,
    project: &Project,
    condition: &Condition,
    at: DateTime<Utc>,
) -> bool {
    match condition {
        Condition::Project { id } => &project.id == id,
        Condition::Folder {
            id,
            include_children,
        } => db.project_is_in_folder(&project.id, id, *include_children),
        Condition::Tag {
            id,
            include_children,
        } => {
            let wanted = tag_set(db, std::slice::from_ref(id), *include_children);
            project.tags.iter().any(|tag| wanted.contains(tag))
        }
        Condition::InInbox => false,
        Condition::Flagged { value } => project.flagged == *value,
        Condition::Priority { .. } => false,
        Condition::Status { value } => match value {
            TaskStatus::Any => true,
            TaskStatus::Completed => project.status == ProjectStatus::Done,
            TaskStatus::Available => project_is_available(db, project, at),
            TaskStatus::Blocked => false,
            TaskStatus::Deferred => project.defer.is_some_and(|stamp| stamp.at > at),
        },
        Condition::Due { window } => window_matches(project.due, *window, at),
        Condition::Deferred { window } => window_matches(project.defer, *window, at),
        Condition::Planned { window } => window_matches(project.planned, *window, at),
        Condition::HasRepeat { .. } => false,
        Condition::FullscreenReminder { value } => project.fullscreen_reminder == *value,
        Condition::Estimate { .. } => false,
        Condition::Text { value } => {
            let needle = value.trim().to_lowercase();
            needle.is_empty()
                || project.name.to_lowercase().contains(&needle)
                || project.notes.to_lowercase().contains(&needle)
        }
        Condition::StatusKind { value } => match value {
            FilterStatus::Flagged => project.flagged,
            FilterStatus::DueSoon => {
                if project.status.is_closed() {
                    return false;
                }
                let days = db.settings.due_soon_days as i64;
                project
                    .due
                    .is_some_and(|due| due.local_date() <= local_date(at) + Duration::days(days))
            }
        },
        Condition::Availability { value } => match value {
            AvailabilityFilter::FirstAvailable | AvailabilityFilter::Available => {
                project_is_available(db, project, at)
            }
            AvailabilityFilter::Remaining => project.status.is_open(),
            AvailabilityFilter::Completed => project.status == ProjectStatus::Done,
            AvailabilityFilter::Dropped => project.status == ProjectStatus::Dropped,
        },
        Condition::HasEstimate { .. } => false,
        Condition::EstimateLessThan { .. } | Condition::EstimateGreaterThan { .. } => false,
        Condition::HasDate { field, value } => {
            project_date_for(project, *field).is_some() == *value
        }
        Condition::DateRange { field, range } => project_date_for(project, *field)
            .is_some_and(|date| range_matches(date, range, at)),
        Condition::IsUntagged { value } => project.tags.is_empty() == *value,
        Condition::TagStatus { value } => project_tag_status_matches(db, project, *value, at),
        Condition::TagsAny {
            ids,
            include_children,
        } => {
            let wanted = tag_set(db, ids, *include_children);
            project.tags.iter().any(|tag| wanted.contains(tag))
        }
        Condition::TagsAll {
            ids,
            include_children,
        } => {
            if ids.is_empty() {
                return true;
            }
            let wanted = tag_set(db, ids, *include_children);
            wanted.iter().all(|tag| project.tags.contains(tag))
        }
        Condition::ItemKind { value } => match value {
            ItemKindFilter::Project | ItemKindFilter::ProjectOrGroup => true,
            ItemKindFilter::Group | ItemKindFilter::NotProjectOrGroup => false,
        },
        Condition::InSingleActionsList => project.kind == ProjectKind::SingleActions,
        Condition::ProjectStatus { value } => project_status_rule_matches(db, project, *value, at),
        Condition::Within {
            project: target,
            folder,
            include_children,
        } => match (target, folder) {
            (Some(id), _) => &project.id == id,
            (None, Some(id)) => db.project_is_in_folder(&project.id, id, *include_children),
            (None, None) => true,
        },
        Condition::Search { terms } => {
            let mut haystack = format!("{}\n{}", project.name, project.notes).to_lowercase();
            if let Some(folder) = &project.folder {
                haystack.push('\n');
                haystack.push_str(&db.folder_path(folder).to_lowercase());
            }
            for tag in &project.tags {
                if let Some(tag) = db.tags.get(tag) {
                    haystack.push('\n');
                    haystack.push_str(&tag.name.to_lowercase());
                }
            }
            terms.iter().all(|term| {
                let needle = term.trim().to_lowercase();
                needle.is_empty() || haystack.contains(&needle)
            })
        }
    }
}

fn project_is_available(db: &Db, project: &Project, at: DateTime<Utc>) -> bool {
    if project.status != ProjectStatus::Active {
        return false;
    }
    if project.defer.is_some_and(|stamp| stamp.at > at) {
        return false;
    }
    if project.kind == ProjectKind::SingleActions {
        return project_has_available_action(db, project, at);
    }
    let has_remaining = db
        .tasks
        .values()
        .any(|task| !task.completed && db.effective_project(&task.id).as_ref() == Some(&project.id));
    !has_remaining || project_has_available_action(db, project, at)
}

fn project_has_available_action(db: &Db, project: &Project, at: DateTime<Utc>) -> bool {
    db.tasks.values().any(|task| {
        !task.completed
            && db.effective_project(&task.id).as_ref() == Some(&project.id)
            && availability::availability(db, &task.id, at).is_available()
    })
}

fn project_status_rule_matches(
    db: &Db,
    project: &Project,
    value: ProjectStatusFilter,
    at: DateTime<Utc>,
) -> bool {
    match value {
        ProjectStatusFilter::Active => project.status == ProjectStatus::Active,
        ProjectStatusFilter::ActiveOrOnHold => project.status.is_open(),
        ProjectStatusFilter::OnHold => project.status == ProjectStatus::OnHold,
        ProjectStatusFilter::Completed => project.status == ProjectStatus::Done,
        ProjectStatusFilter::Dropped => project.status == ProjectStatus::Dropped,
        ProjectStatusFilter::ActiveWithNoRemainingActions => {
            project.status == ProjectStatus::Active
                && !db.tasks.values().any(|task| {
                    !task.completed
                        && db.effective_project(&task.id).as_ref() == Some(&project.id)
                })
        }
        ProjectStatusFilter::ActiveWithFutureDefer => {
            project.status == ProjectStatus::Active
                && project.defer.is_some_and(|stamp| stamp.at > at)
        }
    }
}

fn project_tag_status_matches(
    db: &Db,
    project: &Project,
    value: TagStatusFilter,
    at: DateTime<Utc>,
) -> bool {
    project.tags.iter().any(|id| {
        let Some(tag) = db.tags.get(id) else {
            return false;
        };
        match value {
            TagStatusFilter::ActiveOrOnHold => {
                matches!(tag.status, TagStatus::Available | TagStatus::OnHold)
            }
            TagStatusFilter::OnHold => tag.status == TagStatus::OnHold,
            TagStatusFilter::Dropped => tag.status == TagStatus::Dropped,
            TagStatusFilter::ActiveWithAvailableActions => {
                tag.status == TagStatus::Available && tag_has_available_action(db, id, at)
            }
            TagStatusFilter::ActiveWithNoAvailableActions => {
                tag.status == TagStatus::Available && !tag_has_available_action(db, id, at)
            }
        }
    })
}

fn project_date_for(project: &Project, field: DateField) -> Option<DateTime<Utc>> {
    match field {
        DateField::Due => project.due.map(|stamp| stamp.at),
        DateField::Planned => project.planned.map(|stamp| stamp.at),
        DateField::Defer => project.defer.map(|stamp| stamp.at),
        DateField::Completed => project.completed_at.or_else(|| {
            (project.status == ProjectStatus::Done).then_some(project.modified_at)
        }),
        DateField::Dropped => project.dropped_at.or_else(|| {
            (project.status == ProjectStatus::Dropped).then_some(project.modified_at)
        }),
        DateField::Added => Some(project.created_at),
        DateField::Changed => Some(project.modified_at),
    }
}

pub fn matches(db: &Db, task: &Task, node: &FilterNode, at: DateTime<Utc>) -> bool {
    match node {
        FilterNode::All(nodes) => nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .all(|node| matches(db, task, node, at)),
        FilterNode::Any(nodes) => nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .any(|node| matches(db, task, node, at)),
        FilterNode::None(nodes) => !nodes
            .iter()
            .filter(|node| !node.is_ignorable())
            .any(|node| matches(db, task, node, at)),
        FilterNode::Not(inner) => !matches(db, task, inner, at),
        FilterNode::Condition(condition) => condition_matches(db, task, condition, at),
        FilterNode::Comment(_) | FilterNode::Disabled(_) => true,
    }
}

pub fn condition_matches(db: &Db, task: &Task, condition: &Condition, at: DateTime<Utc>) -> bool {
    match condition {
        Condition::Project { id } => db.effective_project(&task.id).as_ref() == Some(id),
        Condition::Folder {
            id,
            include_children,
        } => db
            .effective_project(&task.id)
            .is_some_and(|project| db.project_is_in_folder(&project, id, *include_children)),
        Condition::Tag {
            id,
            include_children,
        } => {
            let wanted = tag_set(db, std::slice::from_ref(id), *include_children);
            effective_tags(db, task)
                .iter()
                .any(|tag| wanted.contains(tag))
        }
        Condition::InInbox => db.effective_project(&task.id).is_none(),
        Condition::Flagged { value } => task.flagged == *value,
        Condition::Priority { value } => task.priority == Some(*value),
        Condition::Status { value } => status_matches(db, task, *value, at),
        Condition::Due { window } => window_matches(db.effective_due(&task.id), *window, at),
        Condition::Deferred { window } => {
            window_matches(db.effective_defer(&task.id), *window, at)
        }
        Condition::Planned { window } => {
            window_matches(db.effective_planned(&task.id), *window, at)
        }
        Condition::HasRepeat { value } => task.repeat.is_some() == *value,
        Condition::FullscreenReminder { value } => {
            // A mark without a due or explicit reminder has nothing to fire,
            // so it is not an effective full-screen reminder.
            let effective = db.wants_fullscreen_reminder(&task.id)
                && db.reminder_stamp(&task.id).is_some();
            effective == *value
        }
        Condition::Estimate {
            min_minutes,
            max_minutes,
        } => {
            let Some(estimate) = task.estimate_min else {
                return false;
            };
            min_minutes.is_none_or(|min| estimate >= min)
                && max_minutes.is_none_or(|max| estimate <= max)
        }
        Condition::Text { value } => {
            let needle = value.trim().to_lowercase();
            if needle.is_empty() {
                return true;
            }
            search_text(db, task).contains(&needle)
        }

        Condition::StatusKind { value } => match value {
            FilterStatus::Flagged => task.flagged,
            FilterStatus::DueSoon => {
                if task.completed || project_closed(db, task) {
                    return false;
                }
                let days = db.settings.due_soon_days.max(0) as i64;
                db.effective_due(&task.id).is_some_and(|due| {
                    let date = due.local_date();
                    date <= local_date(at) + Duration::days(days)
                })
            }
        },
        Condition::Availability { value } => availability_matches(db, task, *value, at),
        Condition::HasEstimate { value } => task.estimate_min.is_some() == *value,
        Condition::EstimateLessThan { minutes } => {
            task.estimate_min.is_some_and(|estimate| estimate < *minutes)
        }
        Condition::EstimateGreaterThan { minutes } => {
            task.estimate_min.is_some_and(|estimate| estimate > *minutes)
        }
        Condition::HasDate { field, value } => date_for(db, task, *field, at).is_some() == *value,
        Condition::DateRange { field, range } => date_for(db, task, *field, at)
            .is_some_and(|date| range_matches(date, range, at)),
        Condition::IsUntagged { value } => effective_tags(db, task).is_empty() == *value,
        Condition::TagStatus { value } => tag_status_matches(db, task, *value, at),
        Condition::TagsAny {
            ids,
            include_children,
        } => {
            let wanted = tag_set(db, ids, *include_children);
            effective_tags(db, task)
                .iter()
                .any(|tag| wanted.contains(tag))
        }
        Condition::TagsAll {
            ids,
            include_children,
        } => {
            if ids.is_empty() {
                return true;
            }
            let wanted = tag_set(db, ids, *include_children);
            let tagged = effective_tags(db, task);
            wanted.iter().all(|tag| tagged.contains(tag))
        }
        Condition::ItemKind { value } => item_kind_matches(db, task, *value),
        Condition::InSingleActionsList => db
            .effective_project(&task.id)
            .and_then(|id| db.projects.get(&id))
            .is_some_and(|project| project.kind == crate::model::ProjectKind::SingleActions),
        Condition::ProjectStatus { value } => project_status_matches(db, task, *value, at),
        Condition::Within {
            project,
            folder,
            include_children,
        } => match (project, folder) {
            (Some(id), _) => db.effective_project(&task.id).as_ref() == Some(id),
            (None, Some(id)) => db
                .effective_project(&task.id)
                .is_some_and(|project| db.project_is_in_folder(&project, id, *include_children)),
            (None, None) => true,
        },
        Condition::Search { terms } => {
            let haystack = search_text(db, task);
            terms.iter().all(|term| {
                let needle = term.trim().to_lowercase();
                needle.is_empty() || haystack.contains(&needle)
            })
        }
    }
}

fn tag_set(db: &Db, ids: &[TagId], include_children: bool) -> BTreeSet<TagId> {
    let mut wanted: BTreeSet<TagId> = ids.iter().cloned().collect();
    if include_children {
        for id in ids {
            wanted.extend(db.tag_descendants(id));
        }
    }
    wanted
}

/// Every tag that applies to an action: its own tags, tags on parent actions,
/// and tags on the project.
pub(crate) fn effective_tags(db: &Db, task: &Task) -> BTreeSet<TagId> {
    let mut tags = task.tags.clone();
    let mut current = task.parent.clone();
    let mut guard = 0;
    while let Some(id) = current {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(parent) = db.tasks.get(&id) else {
            break;
        };
        tags.extend(parent.tags.iter().cloned());
        current = parent.parent.clone();
    }
    if let Some(project_id) = db.effective_project(&task.id) {
        if let Some(project) = db.projects.get(&project_id) {
            tags.extend(project.tags.iter().cloned());
        }
    }
    tags
}

fn project_dropped(db: &Db, task: &Task) -> bool {
    db.effective_project(&task.id)
        .and_then(|id| db.projects.get(&id))
        .is_some_and(|project| project.status == ProjectStatus::Dropped)
}

fn project_closed(db: &Db, task: &Task) -> bool {
    db.effective_project(&task.id)
        .and_then(|id| db.projects.get(&id))
        .is_some_and(|project| project.status.is_closed())
}

fn availability_matches(
    db: &Db,
    task: &Task,
    value: AvailabilityFilter,
    at: DateTime<Utc>,
) -> bool {
    let availability = availability::availability(db, &task.id, at);
    match value {
        AvailabilityFilter::FirstAvailable => task_is_first_available(db, task, at),
        AvailabilityFilter::Available => availability.is_available(),
        AvailabilityFilter::Remaining => !task.completed && !project_closed(db, task),
        AvailabilityFilter::Completed => {
            task.completed
                || db
                    .effective_project(&task.id)
                    .and_then(|id| db.projects.get(&id))
                    .is_some_and(|project| project.status == ProjectStatus::Done)
        }
        AvailabilityFilter::Dropped => project_dropped(db, task),
    }
}

fn task_is_first_available(db: &Db, task: &Task, at: DateTime<Utc>) -> bool {
    if !availability::availability(db, &task.id, at).is_available() {
        return false;
    }
    let Some(project_id) = db.effective_project(&task.id) else {
        // The Inbox has no project structure, so every available action is
        // also first available there.
        return true;
    };
    let Some(project) = db.projects.get(&project_id) else {
        return true;
    };
    if project.kind == ProjectKind::SingleActions {
        return true;
    }
    db.project_outline(&project_id, true)
        .into_iter()
        .map(|row| row.task)
        .find(|id| {
            db.tasks.get(id).is_some_and(|candidate| {
                !candidate.completed
                    && availability::availability(db, id, at).is_available()
            })
        })
        .as_ref()
        == Some(&task.id)
}

fn item_kind_matches(db: &Db, task: &Task, value: ItemKindFilter) -> bool {
    let task_has_children = !task.child_order.is_empty()
        || db
            .tasks
            .values()
            .any(|other| other.parent.as_ref() == Some(&task.id));
    match value {
        // Custom perspectives list actions, not project rows. A project row is
        // never emitted, so this rule only matches once project-level
        // perspectives arrive. The other three are meaningful today.
        ItemKindFilter::Project => false,
        ItemKindFilter::Group => task_has_children,
        ItemKindFilter::ProjectOrGroup => task_has_children,
        ItemKindFilter::NotProjectOrGroup => !task_has_children,
    }
}

fn project_status_matches(
    db: &Db,
    task: &Task,
    value: ProjectStatusFilter,
    at: DateTime<Utc>,
) -> bool {
    let Some(project) = db
        .effective_project(&task.id)
        .and_then(|id| db.projects.get(&id))
    else {
        return false;
    };
    match value {
        ProjectStatusFilter::Active => project.status == ProjectStatus::Active,
        ProjectStatusFilter::ActiveOrOnHold => {
            matches!(project.status, ProjectStatus::Active | ProjectStatus::OnHold)
        }
        ProjectStatusFilter::OnHold => project.status == ProjectStatus::OnHold,
        ProjectStatusFilter::Completed => project.status == ProjectStatus::Done,
        ProjectStatusFilter::Dropped => project.status == ProjectStatus::Dropped,
        ProjectStatusFilter::ActiveWithNoRemainingActions => {
            project.status == ProjectStatus::Active
                && !db
                    .tasks
                    .values()
                    .any(|task| !task.completed && db.effective_project(&task.id).as_ref() == Some(&project.id))
        }
        ProjectStatusFilter::ActiveWithFutureDefer => {
            project.status == ProjectStatus::Active
                && project.defer.is_some_and(|stamp| stamp.at > at)
        }
    }
}

fn tag_status_matches(
    db: &Db,
    task: &Task,
    value: TagStatusFilter,
    at: DateTime<Utc>,
) -> bool {
    let tags = effective_tags(db, task);
    tags.iter().any(|id| {
        let Some(tag) = db.tags.get(id) else {
            return false;
        };
        match value {
            TagStatusFilter::ActiveOrOnHold => {
                matches!(tag.status, TagStatus::Available | TagStatus::OnHold)
            }
            TagStatusFilter::OnHold => tag.status == TagStatus::OnHold,
            TagStatusFilter::Dropped => tag.status == TagStatus::Dropped,
            TagStatusFilter::ActiveWithAvailableActions => {
                tag.status == TagStatus::Available
                    && tag_has_available_action(db, id, at)
            }
            TagStatusFilter::ActiveWithNoAvailableActions => {
                tag.status == TagStatus::Available
                    && !tag_has_available_action(db, id, at)
            }
        }
    })
}

fn tag_has_available_action(db: &Db, tag: &TagId, at: DateTime<Utc>) -> bool {
    let wanted = tag_set(db, std::slice::from_ref(tag), true);
    db.tasks.values().any(|task| {
        !task.completed
            && availability::availability(db, &task.id, at).is_available()
            && effective_tags(db, task)
                .iter()
                .any(|candidate| wanted.contains(candidate))
    })
}

fn date_for(
    db: &Db,
    task: &Task,
    field: DateField,
    _at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match field {
        DateField::Due => db.effective_due(&task.id).map(|stamp| stamp.at),
        DateField::Planned => db.effective_planned(&task.id).map(|stamp| stamp.at),
        DateField::Defer => db.effective_defer(&task.id).map(|stamp| stamp.at),
        DateField::Completed => task.completed_at,
        DateField::Dropped => task.trashed_at.or_else(|| {
            db.effective_project(&task.id)
                .and_then(|id| db.projects.get(&id))
                .filter(|project| project.status == ProjectStatus::Dropped)
                .and_then(|project| project.dropped_at)
        }),
        DateField::Added => Some(task.created_at),
        DateField::Changed => Some(task.modified_at),
    }
}

fn range_matches(date: DateTime<Utc>, range: &DateRangeFilter, at: DateTime<Utc>) -> bool {
    let today = local_date(at);
    let date_only = local_date(date);
    match range {
        DateRangeFilter::On { date: target } => date_only == *target,
        DateRangeFilter::Yesterday => date_only == today - Duration::days(1),
        DateRangeFilter::Today => date_only == today,
        DateRangeFilter::Tomorrow => date_only == today + Duration::days(1),
        DateRangeFilter::InPast { amount, unit } => {
            if *unit == DateUnit::Hours {
                let edge = at - Duration::hours(*amount as i64);
                date <= at && date >= edge
            } else {
                let edge = local_date(shift(at, *unit, -(*amount as i64)));
                date_only <= today && date_only >= edge
            }
        }
        DateRangeFilter::InNext { amount, unit } => {
            if *unit == DateUnit::Hours {
                let edge = at + Duration::hours(*amount as i64);
                date >= at && date <= edge
            } else {
                let edge = local_date(shift(at, *unit, *amount as i64));
                date_only >= today && date_only <= edge
            }
        }
        DateRangeFilter::Between { from, to } => {
            from.is_none_or(|from| date_only >= from) && to.is_none_or(|to| date_only <= to)
        }
    }
}

fn shift(at: DateTime<Utc>, unit: DateUnit, amount: i64) -> DateTime<Utc> {
    let months = |count: u32| {
        if amount < 0 {
            at.checked_sub_months(Months::new(count))
        } else {
            at.checked_add_months(Months::new(count))
        }
        .unwrap_or(at)
    };
    match unit {
        DateUnit::Hours => at + Duration::hours(amount),
        DateUnit::Days => at + Duration::days(amount),
        DateUnit::Weeks => at + Duration::weeks(amount),
        DateUnit::Months => months(amount.unsigned_abs() as u32),
        DateUnit::Years => months((amount.unsigned_abs() * 12) as u32),
    }
}

fn search_text(db: &Db, task: &Task) -> String {
    let mut text = format!("{}\n{}", task.title, task.notes).to_lowercase();
    if let Some(project_id) = db.effective_project(&task.id) {
        if let Some(project) = db.projects.get(&project_id) {
            text.push('\n');
            text.push_str(&project.name.to_lowercase());
            text.push('\n');
            text.push_str(&project.notes.to_lowercase());
            if let Some(folder_id) = &project.folder {
                text.push('\n');
                text.push_str(&db.folder_path(folder_id).to_lowercase());
            }
        }
    }
    for tag in effective_tags(db, task) {
        if let Some(tag) = db.tags.get(&tag) {
            text.push('\n');
            text.push_str(&tag.name.to_lowercase());
        }
    }
    let mut current = task.parent.clone();
    let mut guard = 0;
    while let Some(id) = current {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(parent) = db.tasks.get(&id) else {
            break;
        };
        text.push('\n');
        text.push_str(&parent.title.to_lowercase());
        current = parent.parent.clone();
    }
    text
}

fn status_matches(db: &Db, task: &Task, status: TaskStatus, at: DateTime<Utc>) -> bool {
    match status {
        TaskStatus::Any => true,
        TaskStatus::Completed => task.completed,
        TaskStatus::Available => availability::availability(db, &task.id, at).is_available(),
        TaskStatus::Blocked => {
            !task.completed && !availability::availability(db, &task.id, at).is_available()
        }
        TaskStatus::Deferred => {
            if task.is_deferred(at) {
                return true;
            }
            if let Some(project_id) = &task.project {
                if let Some(project) = db.projects.get(project_id) {
                    return project.defer.is_some_and(|stamp| stamp.at > at);
                }
            }
            matches!(
                availability::availability(db, &task.id, at),
                Availability::Deferred(_) | Availability::ProjectDeferred(_, _)
            )
        }
    }
}

fn window_matches(stamp: Option<Stamp>, window: DueWindow, at: DateTime<Utc>) -> bool {
    let today = local_date(at);
    let date = stamp.map(|stamp| stamp.local_date());
    match window {
        DueWindow::Overdue => date.is_some_and(|date| date < today),
        DueWindow::Today => date == Some(today),
        DueWindow::Tomorrow => date == Some(today + Duration::days(1)),
        DueWindow::Next7Days => {
            date.is_some_and(|date| date >= today && date <= today + Duration::days(7))
        }
        DueWindow::Next30Days => {
            date.is_some_and(|date| date >= today && date <= today + Duration::days(30))
        }
        DueWindow::Later => date.map_or(true, |date| date > today),
        DueWindow::NoDate => date.is_none(),
        DueWindow::On(target) => date == Some(target),
        DueWindow::Before(target) => date.is_some_and(|date| date < target),
        DueWindow::After(target) => date.is_some_and(|date| date > target),
    }
}

/// Parse the `--due` style shortcuts used by the CLI into a window.
pub fn window_from_keyword(value: &str, today: NaiveDate) -> Option<DueWindow> {
    match value.trim().to_lowercase().as_str() {
        "overdue" => Some(DueWindow::Overdue),
        "today" => Some(DueWindow::Today),
        "tomorrow" => Some(DueWindow::Tomorrow),
        "next7" | "next-7" | "week" => Some(DueWindow::Next7Days),
        "next30" | "next-30" | "month" => Some(DueWindow::Next30Days),
        "later" | "not-yet" | "future" => Some(DueWindow::Later),
        "none" | "nodate" | "no-date" => Some(DueWindow::NoDate),
        "before" => Some(DueWindow::Before(today)),
        "after" => Some(DueWindow::After(today)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FolderId, ProjectId, TaskId};
    use crate::model::{Folder, ItemKindFilter, Project, ProjectKind, Tag, Task};
    use crate::time::Stamp;

    fn db_with_task(task: Task) -> (Db, Task) {
        let mut db = Db::new();
        let id = db.add_task_to_inbox(task.clone());
        let mut task = db.tasks[&id].clone();
        task.id = id;
        (db, task)
    }

    #[test]
    fn nested_none_excludes_matching_items() {
        let mut task = Task::new("flagged");
        task.flagged = true;
        let (db, task) = db_with_task(task);
        let filter = FilterNode::all(vec![
            FilterNode::condition(Condition::StatusKind {
                value: FilterStatus::Flagged,
            }),
            FilterNode::none(vec![FilterNode::condition(Condition::InInbox)]),
        ]);
        let at = Utc::now();
        assert!(!matches(&db, &task, &filter, at));

        let mut plain = Task::new("plain");
        plain.flagged = true;
        let (db, plain) = db_with_task(plain);
        let filter = FilterNode::all(vec![
            FilterNode::condition(Condition::StatusKind {
                value: FilterStatus::Flagged,
            }),
            FilterNode::none(vec![FilterNode::condition(Condition::InInbox)]),
        ]);
        // A plain inbox task is still excluded, because InInbox matches.
        assert!(!matches(&db, &plain, &filter, at));
    }

    #[test]
    fn comments_and_disabled_rules_do_not_affect_an_any_group() {
        let (db, task) = db_with_task(Task::new("one"));
        let filter = FilterNode::any(vec![
            FilterNode::comment("why"),
            FilterNode::disabled(FilterNode::condition(Condition::InInbox)),
        ]);
        // No enabled child matches, so the Any group is false.
        assert!(!matches(&db, &task, &filter, Utc::now()));
    }

    #[test]
    fn tag_rules_match_any_all_and_untagged() {
        let mut db = Db::new();
        let mut work = Tag::new("Work");
        work.id = crate::ids::TagId::new();
        let work_id = work.id.clone();
        let mut home = Tag::new("Home");
        home.id = crate::ids::TagId::new();
        let home_id = home.id.clone();
        db.tags.insert(work_id.clone(), work);
        db.tags.insert(home_id.clone(), home);

        let mut both = Task::new("both");
        both.tags.insert(work_id.clone());
        both.tags.insert(home_id.clone());
        let both_id = db.add_task_to_inbox(both);
        let both = db.tasks[&both_id].clone();

        let any = FilterNode::condition(Condition::TagsAny {
            ids: vec![work_id.clone()],
            include_children: false,
        });
        assert!(matches(&db, &both, &any, Utc::now()));
        let all = FilterNode::condition(Condition::TagsAll {
            ids: vec![work_id, home_id],
            include_children: false,
        });
        assert!(matches(&db, &both, &all, Utc::now()));

        let loose = Task::new("loose");
        let loose_id = db.add_task_to_inbox(loose);
        let loose = db.tasks[&loose_id].clone();
        let untagged = FilterNode::condition(Condition::IsUntagged { value: true });
        assert!(matches(&db, &loose, &untagged, Utc::now()));
        assert!(!matches(&db, &both, &untagged, Utc::now()));
    }

    #[test]
    fn date_range_rules_match_today_and_between() {
        let today = local_date(Utc::now());
        let mut task = Task::new("today");
        task.due = Some(Stamp::date_only(today));
        let (db, task) = db_with_task(task);

        let today_rule = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::Today,
        });
        assert!(matches(&db, &task, &today_rule, Utc::now()));

        let between = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::Between {
                from: Some(today - Duration::days(1)),
                to: Some(today + Duration::days(1)),
            },
        });
        assert!(matches(&db, &task, &between, Utc::now()));

        let next = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::InNext {
                amount: 7,
                unit: DateUnit::Days,
            },
        });
        assert!(matches(&db, &task, &next, Utc::now()));
    }

    #[test]
    fn project_status_structure_and_containment_rules() {
        let mut db = Db::new();
        let mut project = Project::new("Errands");
        project.status = ProjectStatus::OnHold;
        project.kind = ProjectKind::SingleActions;
        let project_id = db.add_project(project);

        let mut task = Task::new("Buy milk");
        task.project = Some(project_id.clone());
        let task_id = db.add_task_to_inbox(task);
        db.tasks.get_mut(&task_id).unwrap().project = Some(project_id.clone());
        let task = db.tasks[&task_id].clone();

        let on_hold = FilterNode::condition(Condition::ProjectStatus {
            value: ProjectStatusFilter::OnHold,
        });
        assert!(matches(&db, &task, &on_hold, Utc::now()));

        let single = FilterNode::condition(Condition::InSingleActionsList);
        assert!(matches(&db, &task, &single, Utc::now()));

        let within = FilterNode::condition(Condition::Within {
            project: Some(project_id),
            folder: None,
            include_children: false,
        });
        assert!(matches(&db, &task, &within, Utc::now()));

        let kind = FilterNode::condition(Condition::ItemKind {
            value: ItemKindFilter::NotProjectOrGroup,
        });
        assert!(matches(&db, &task, &kind, Utc::now()));
    }

    #[test]
    fn project_rules_match_projects_and_appear_as_project_groups() {
        let mut db = Db::new();
        let mut project = Project::new("Paused work");
        project.status = ProjectStatus::OnHold;
        let project_id = db.add_project(project);
        let filter = FilterNode::all(vec![
            FilterNode::condition(Condition::ItemKind {
                value: ItemKindFilter::Project,
            }),
            FilterNode::condition(Condition::ProjectStatus {
                value: ProjectStatusFilter::OnHold,
            }),
        ]);
        let at = Utc::now();
        let project = db.projects.get(&project_id).unwrap();
        assert!(matches_project(&db, project, &filter, at));

        let groups = crate::query::run(&db, &filter, crate::model::SortBy::Manual, crate::model::GroupBy::None, false, at);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].key, "projects");
        assert_eq!(groups[0].projects, vec![project_id]);
        assert!(groups[0].rows.is_empty());
    }

    fn project_with_task(db: &mut Db, name: &str, folder: &FolderId, title: &str) -> (ProjectId, TaskId) {
        let project_id = db.add_project(Project::new(name));
        db.projects.get_mut(&project_id).unwrap().folder = Some(folder.clone());
        let mut task = Task::new(title);
        task.project = Some(project_id.clone());
        let task_id = db.add_task_to_inbox(task);
        db.inbox_order.retain(|id| id != &task_id);
        db.tasks.get_mut(&task_id).unwrap().project = Some(project_id.clone());
        db.projects
            .get_mut(&project_id)
            .unwrap()
            .task_order
            .push(task_id.clone());
        (project_id, task_id)
    }

    #[test]
    fn project_grouping_nests_matching_actions_under_their_project() {
        let mut db = Db::new();
        let work = db.add_folder(Folder::new("Work"));
        let home = db.add_folder(Folder::new("Home"));
        let (alpha, alpha_task) = project_with_task(&mut db, "Alpha", &work, "Write the brief");
        let (beta, beta_task) = project_with_task(&mut db, "Beta", &home, "Buy paint");
        let filter = FilterNode::all(Vec::new());
        let at = Utc::now();

        let groups = crate::query::run_projects(
            &db,
            &filter,
            crate::model::SortBy::Manual,
            crate::model::GroupBy::None,
            crate::model::GroupBy::Folder,
            crate::model::SortBy::Manual,
            false,
            at,
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].title, "Work");
        assert_eq!(groups[0].projects, vec![alpha.clone()]);
        assert_eq!(
            groups[0].rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
            vec![alpha_task]
        );
        assert_eq!(groups[1].title, "Home");
        assert_eq!(groups[1].projects, vec![beta.clone()]);
        assert_eq!(
            groups[1].rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
            vec![beta_task]
        );
    }

    #[test]
    fn project_sort_orders_the_projects_block() {
        let mut db = Db::new();
        let folder = db.add_folder(Folder::new("Work"));
        let (beta, _) = project_with_task(&mut db, "Beta", &folder, "Second");
        let (alpha, _) = project_with_task(&mut db, "Alpha", &folder, "First");
        let filter = FilterNode::condition(Condition::ItemKind {
            value: ItemKindFilter::Project,
        });
        let groups = crate::query::run_projects(
            &db,
            &filter,
            crate::model::SortBy::Manual,
            crate::model::GroupBy::None,
            crate::model::GroupBy::None,
            crate::model::SortBy::Name,
            false,
            Utc::now(),
        );
        assert_eq!(groups[0].projects, vec![alpha, beta]);
    }

    #[test]
    fn first_available_shows_one_action_in_a_parallel_project() {
        let mut db = Db::new();
        let project_id = db.add_project(Project::new("Parallel"));
        let mut first = Task::new("First");
        let mut second = Task::new("Second");
        first.project = Some(project_id.clone());
        second.project = Some(project_id.clone());
        let first_id = db.add_task_to_inbox(first);
        let second_id = db.add_task_to_inbox(second);
        db.tasks.get_mut(&first_id).unwrap().project = Some(project_id.clone());
        db.tasks.get_mut(&second_id).unwrap().project = Some(project_id.clone());
        db.projects.get_mut(&project_id).unwrap().task_order =
            vec![first_id.clone(), second_id.clone()];

        let at = Utc::now();
        let first_rule = FilterNode::condition(Condition::Availability {
            value: AvailabilityFilter::FirstAvailable,
        });
        assert!(matches(&db, &db.tasks[&first_id], &first_rule, at));
        assert!(!matches(&db, &db.tasks[&second_id], &first_rule, at));

        let available = FilterNode::condition(Condition::Availability {
            value: AvailabilityFilter::Available,
        });
        assert!(matches(&db, &db.tasks[&second_id], &available, at));
    }

    #[test]
    fn project_dropped_date_is_queryable() {
        let mut db = Db::new();
        let mut project = Project::new("Old");
        project.status = ProjectStatus::Dropped;
        project.dropped_at = Some(Utc::now());
        let project_id = db.add_project(project);
        let today = local_date(Utc::now());
        let filter = FilterNode::condition(Condition::DateRange {
            field: DateField::Dropped,
            range: DateRangeFilter::Today,
        });
        assert!(matches_project(
            &db,
            db.projects.get(&project_id).unwrap(),
            &filter,
            Utc::now()
        ));
        assert_eq!(today, local_date(Utc::now()));
    }

    #[test]
    fn estimate_greater_than_and_ranges_match() {
        let mut task = Task::new("long");
        task.estimate_min = Some(45);
        let (db, task) = db_with_task(task);
        let greater = FilterNode::condition(Condition::EstimateGreaterThan { minutes: 30 });
        assert!(matches(&db, &task, &greater, Utc::now()));
        let range = FilterNode::condition(Condition::Estimate {
            min_minutes: Some(30),
            max_minutes: Some(60),
        });
        assert!(matches(&db, &task, &range, Utc::now()));
        let too_long = FilterNode::condition(Condition::EstimateGreaterThan { minutes: 60 });
        assert!(!matches(&db, &task, &too_long, Utc::now()));
    }

    #[test]
    fn open_ended_between_ranges_handle_future_and_past_cutoffs() {
        use chrono::NaiveDate;
        let cutoff = NaiveDate::from_ymd_opt(2026, 12, 30).unwrap();
        let future = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::Between {
                from: Some(cutoff + Duration::days(1)),
                to: None,
            },
        });
        let past = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::Between {
                from: None,
                to: Some(cutoff),
            },
        });

        let mut after = Task::new("after");
        after.due = Some(Stamp::date_only(cutoff + Duration::days(1)));
        let (db, after) = db_with_task(after);
        assert!(matches(&db, &after, &future, Utc::now()));
        assert!(!matches(&db, &after, &past, Utc::now()));

        let mut on_cutoff = Task::new("cutoff");
        on_cutoff.due = Some(Stamp::date_only(cutoff));
        let (db, on_cutoff) = db_with_task(on_cutoff);
        assert!(!matches(&db, &on_cutoff, &future, Utc::now()));
        assert!(matches(&db, &on_cutoff, &past, Utc::now()));

        let (db, undated) = db_with_task(Task::new("undated"));
        assert!(!matches(&db, &undated, &future, Utc::now()));
        assert!(!matches(&db, &undated, &past, Utc::now()));
    }

    #[test]
    fn remaining_and_due_soon_exclude_closed_projects() {
        let mut db = Db::new();
        let mut project = Project::new("Done project");
        project.status = ProjectStatus::Done;
        let project_id = db.add_project(project);
        let mut task = Task::new("left behind");
        task.project = Some(project_id.clone());
        let task_id = db.add_task_to_inbox(task);
        db.tasks.get_mut(&task_id).unwrap().project = Some(project_id);
        let task = db.tasks[&task_id].clone();
        let remaining = FilterNode::condition(Condition::Availability {
            value: AvailabilityFilter::Remaining,
        });
        assert!(!matches(&db, &task, &remaining, Utc::now()));
        let soon = FilterNode::condition(Condition::StatusKind {
            value: FilterStatus::DueSoon,
        });
        db.tasks.get_mut(&task_id).unwrap().due =
            Some(Stamp::date_only(local_date(Utc::now())));
        let task = db.tasks[&task_id].clone();
        assert!(!matches(&db, &task, &soon, Utc::now()));
    }

    #[test]
    fn due_soon_window_edges_are_calendar_days() {
        let today = local_date(Utc::now());
        let rule = FilterNode::condition(Condition::StatusKind {
            value: FilterStatus::DueSoon,
        });
        for (offset, expected) in [(-3, true), (0, true), (2, true), (3, false)] {
            let mut task = Task::new("edge");
            task.due = Some(Stamp::date_only(today + Duration::days(offset)));
            let (db, task) = db_with_task(task);
            assert_eq!(matches(&db, &task, &rule, Utc::now()), expected, "offset {offset}");
        }
    }

    #[test]
    fn tag_grouping_uses_parent_and_project_tags() {
        let mut db = Db::new();
        let mut work = Tag::new("Work");
        work.id = crate::ids::TagId::new();
        let work_id = work.id.clone();
        db.tags.insert(work_id.clone(), work);
        db.tag_roots.push(work_id.clone());
        let project_id = db.add_project(Project::new("Project"));
        db.projects.get_mut(&project_id).unwrap().tags.insert(work_id.clone());
        let mut parent = Task::new("Parent");
        parent.tags.insert(work_id.clone());
        parent.project = Some(project_id.clone());
        let parent_id = db.add_task_to_inbox(parent);
        db.tasks.get_mut(&parent_id).unwrap().project = Some(project_id.clone());
        db.projects
            .get_mut(&project_id)
            .unwrap()
            .task_order
            .push(parent_id.clone());
        let mut child = Task::new("Child");
        child.parent = Some(parent_id.clone());
        let child_id = db.add_task_to_inbox(child);
        db.tasks.get_mut(&child_id).unwrap().parent = Some(parent_id.clone());
        db.tasks
            .get_mut(&parent_id)
            .unwrap()
            .child_order
            .push(child_id.clone());
        let filter = FilterNode::condition(Condition::ItemKind {
            value: ItemKindFilter::NotProjectOrGroup,
        });
        let groups = crate::query::run(
            &db,
            &filter,
            crate::model::SortBy::Manual,
            crate::model::GroupBy::Tag,
            false,
            Utc::now(),
        );
        let titles: Vec<&str> = groups.iter().map(|group| group.title.as_str()).collect();
        assert!(titles.contains(&"Work"), "{titles:?}");
    }

    #[test]
    fn action_groups_remain_groups_after_children_complete() {
        let mut db = Db::new();
        let mut parent = Task::new("Group");
        parent.sequential = true;
        let parent_id = db.add_task_to_inbox(parent);
        let mut child = Task::new("Done child");
        child.parent = Some(parent_id.clone());
        child.completed = true;
        let child_id = db.add_task_to_inbox(child);
        db.tasks.get_mut(&child_id).unwrap().parent = Some(parent_id.clone());
        db.tasks.get_mut(&child_id).unwrap().completed = true;
        let parent = db.tasks[&parent_id].clone();
        let rule = FilterNode::condition(Condition::ItemKind {
            value: ItemKindFilter::Group,
        });
        assert!(matches(&db, &parent, &rule, Utc::now()));
    }

    #[test]
    fn hour_ranges_use_timestamps_not_calendar_days() {
        let at = Utc::now();
        let mut task = Task::new("two hours ago");
        task.due = Some(Stamp {
            at: at - Duration::hours(2),
            has_time: true,
        });
        let mut db = Db::new();
        let id = db.add_task_to_inbox(task);
        let task = db.tasks[&id].clone();
        let one_hour = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::InPast {
                amount: 1,
                unit: DateUnit::Hours,
            },
        });
        let three_hours = FilterNode::condition(Condition::DateRange {
            field: DateField::Due,
            range: DateRangeFilter::InPast {
                amount: 3,
                unit: DateUnit::Hours,
            },
        });
        assert!(!matches(&db, &task, &one_hour, at));
        assert!(matches(&db, &task, &three_hours, at));
    }

    #[test]
    fn tasks_in_dropped_projects_inherit_the_dropped_date() {
        let mut db = Db::new();
        let mut project = Project::new("Dropped");
        project.status = ProjectStatus::Dropped;
        project.dropped_at = Some(Utc::now());
        let project_id = db.add_project(project);
        let mut task = Task::new("inside");
        task.project = Some(project_id.clone());
        let task_id = db.add_task_to_inbox(task);
        db.tasks.get_mut(&task_id).unwrap().project = Some(project_id);
        let task = db.tasks[&task_id].clone();
        let rule = FilterNode::condition(Condition::DateRange {
            field: DateField::Dropped,
            range: DateRangeFilter::Today,
        });
        assert!(matches(&db, &task, &rule, Utc::now()));
        let groups = crate::query::run(
            &db,
            &FilterNode::condition(Condition::ItemKind {
                value: ItemKindFilter::NotProjectOrGroup,
            }),
            crate::model::SortBy::Manual,
            crate::model::GroupBy::DroppedDate,
            true,
            Utc::now(),
        );
        assert!(
            groups.iter().any(|group| group.title != "Not Dropped"),
            "{:?}",
            groups.iter().map(|group| group.title.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn due_soon_uses_the_library_horizon() {
        let today = local_date(Utc::now());
        let mut task = Task::new("soon");
        task.due = Some(Stamp::date_only(today + Duration::days(2)));
        let (db, task) = db_with_task(task);
        let soon = FilterNode::condition(Condition::StatusKind {
            value: FilterStatus::DueSoon,
        });
        assert!(matches(&db, &task, &soon, Utc::now()));
    }
}
