//! Evaluating saved filters against tasks.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Months, NaiveDate, Utc};

use crate::db::Db;
use crate::ids::TagId;
use crate::model::{
    AvailabilityFilter, Condition, DateField, DateRangeFilter, DateUnit, DueWindow, FilterNode,
    FilterStatus, ItemKindFilter, ProjectStatus, ProjectStatusFilter, TagStatus, TagStatusFilter,
    Task, TaskStatus,
};
use crate::query::availability::{self, Availability};
use crate::time::{Stamp, local_date};

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
                if task.completed || project_dropped(db, task) {
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
fn effective_tags(db: &Db, task: &Task) -> BTreeSet<TagId> {
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

fn availability_matches(
    db: &Db,
    task: &Task,
    value: AvailabilityFilter,
    at: DateTime<Utc>,
) -> bool {
    let availability = availability::availability(db, &task.id, at);
    match value {
        AvailabilityFilter::FirstAvailable | AvailabilityFilter::Available => {
            availability.is_available()
        }
        AvailabilityFilter::Remaining => !task.completed && !project_dropped(db, task),
        AvailabilityFilter::Completed => task.completed,
        AvailabilityFilter::Dropped => project_dropped(db, task),
    }
}

fn item_kind_matches(db: &Db, task: &Task, value: ItemKindFilter) -> bool {
    let task_has_children = task
        .child_order
        .iter()
        .any(|id| db.tasks.get(id).is_some_and(|child| !child.completed))
        || db
            .tasks
            .values()
            .any(|other| other.parent.as_ref() == Some(&task.id) && !other.completed);
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
        DateField::Dropped => None,
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
            let edge = local_date(shift(at, *unit, -(*amount as i64)));
            date_only <= today && date_only >= edge
        }
        DateRangeFilter::InNext { amount, unit } => {
            let edge = local_date(shift(at, *unit, *amount as i64));
            date_only >= today && date_only <= edge
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
    use crate::model::{Project, ProjectKind, Tag, Task};
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
