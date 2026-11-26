//! Forecast buckets relative to the local "today".

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use std::collections::BTreeMap;

use crate::db::Db;
use crate::ids::{ProjectId, TaskId};
use crate::model::ProjectStatus;
use crate::time::{local_date, weekday_short};

#[derive(Clone, Debug, Default)]
pub struct ForecastDay {
    pub date: NaiveDate,
    pub due: Vec<TaskId>,
    pub deferred: Vec<TaskId>,
    pub planned: Vec<TaskId>,
}

/// Project dates for one local day. A project can carry due, defer and planned
/// dates on the same day; Forecast still draws it once in that day's Projects
/// section.
#[derive(Clone, Debug, Default)]
pub struct ForecastProjectDay {
    pub due: Vec<ProjectId>,
    pub deferred: Vec<ProjectId>,
    pub planned: Vec<ProjectId>,
}

#[derive(Clone, Debug, Default)]
pub struct Forecast {
    pub overdue: Vec<TaskId>,
    pub today: Vec<TaskId>,
    pub tomorrow: Vec<TaskId>,
    pub days: BTreeMap<NaiveDate, ForecastDay>,
    pub later: Vec<TaskId>,
    /// Due or planned more than 30 days out. Kept separate so Forecast can
    /// fold the far horizon away under Next month and later.
    pub next_month: Vec<TaskId>,
    pub deferred_today: Vec<TaskId>,
    pub planned_today: Vec<TaskId>,
    /// Planned dates that have slipped: still open, still informative, and
    /// deliberately not rolled forward. Forecast keeps them in Past.
    pub past_planned: Vec<TaskId>,
    /// Undated actions carrying the Forecast tag.
    pub tagged: Vec<TaskId>,
    /// Undated flagged actions when their Forecast option is on.
    pub flagged: Vec<TaskId>,
    /// Project due/defer/planned dates, keyed by local day. Completed and
    /// dropped projects are excluded; on-hold projects keep their dates.
    pub projects: BTreeMap<NaiveDate, ForecastProjectDay>,
}

impl Forecast {
    pub fn all_ids(&self) -> Vec<TaskId> {
        let mut out = Vec::new();
        out.extend(self.overdue.iter().cloned());
        out.extend(self.today.iter().cloned());
        out.extend(self.tomorrow.iter().cloned());
        for day in self.days.values() {
            out.extend(day.due.iter().cloned());
            out.extend(day.deferred.iter().cloned());
            out.extend(day.planned.iter().cloned());
        }
        out.extend(self.planned_today.iter().cloned());
        out.extend(self.past_planned.iter().cloned());
        out.extend(self.later.iter().cloned());
        out.extend(self.next_month.iter().cloned());
        out.extend(self.tagged.iter().cloned());
        out.extend(self.flagged.iter().cloned());
        out
    }
}

/// Build the Forecast view. Completed tasks and tasks in done/dropped/on-hold
/// projects are excluded. Each date type is independent: a task planned for
/// today and due next week appears in both places, and a planned date that has
/// slipped stays in Past rather than moving forward.
pub fn forecast(db: &Db, at: DateTime<Utc>) -> Forecast {
    let today = local_date(at);
    let tomorrow = today + chrono::Duration::days(1);
    let horizon = today + chrono::Duration::days(7);
    let month_horizon = today + chrono::Duration::days(30);
    let mut result = Forecast::default();

    for task in db.tasks.values() {
        if task.completed || task.is_trashed() {
            continue;
        }
        if let Some(project_id) = &task.project {
            match db.projects.get(project_id).map(|project| project.status) {
                Some(ProjectStatus::Active) | None => {}
                _ => continue,
            }
        }

        if let Some(due) = db.effective_due(&task.id) {
            let date = due.local_date();
            if date < today {
                result.overdue.push(task.id.clone());
            } else if date == today {
                result.today.push(task.id.clone());
            } else if date == tomorrow {
                result.tomorrow.push(task.id.clone());
            } else if date <= horizon {
                result
                    .days
                    .entry(date)
                    .or_insert_with(|| ForecastDay {
                        date,
                        ..ForecastDay::default()
                    })
                    .due
                    .push(task.id.clone());
            } else if date <= month_horizon {
                result.later.push(task.id.clone());
            } else {
                result.next_month.push(task.id.clone());
            }
        }

        // Defer dates place work on the day it becomes available; a defer date
        // in the past is simply available, not a Forecast entry.
        if let Some(defer) = db.effective_defer(&task.id) {
            let date = defer.local_date();
            if date == today {
                result.deferred_today.push(task.id.clone());
            } else if (today..=horizon).contains(&date) {
                result
                    .days
                    .entry(date)
                    .or_insert_with(|| ForecastDay {
                        date,
                        ..ForecastDay::default()
                    })
                    .deferred
                    .push(task.id.clone());
            }
        }

        if let Some(planned) = db.effective_planned(&task.id) {
            let date = planned.local_date();
            if date < today {
                result.past_planned.push(task.id.clone());
            } else if date == today {
                result.planned_today.push(task.id.clone());
            } else if date <= horizon {
                result
                    .days
                    .entry(date)
                    .or_insert_with(|| ForecastDay {
                        date,
                        ..ForecastDay::default()
                    })
                    .planned
                    .push(task.id.clone());
            } else if date <= month_horizon {
                result.later.push(task.id.clone());
            } else {
                result.next_month.push(task.id.clone());
            }
        }
    }

    for project in db.projects.values() {
        if !project.status.is_open() {
            continue;
        }
        for (date, kind) in [
            (project.due, 0),
            (project.defer, 1),
            (project.planned, 2),
        ] {
            let Some(stamp) = date else {
                continue;
            };
            let day = result
                .projects
                .entry(stamp.local_date())
                .or_insert_with(ForecastProjectDay::default);
            match kind {
                0 => day.due.push(project.id.clone()),
                1 => day.deferred.push(project.id.clone()),
                _ => day.planned.push(project.id.clone()),
            }
        }
    }
    for task in db.tasks.values() {
        if task.completed || task.is_trashed() {
            continue;
        }
        if db
            .effective_project(&task.id)
            .and_then(|project| db.projects.get(&project))
            .is_some_and(|project| project.status != ProjectStatus::Active)
        {
            continue;
        }
        if db.effective_due(&task.id).is_some()
            || db.effective_defer(&task.id).is_some()
            || db.effective_planned(&task.id).is_some()
        {
            continue;
        }
        if db
            .settings
            .forecast_tag
            .as_ref()
            .is_some_and(|tag| task.tags.contains(tag))
        {
            result.tagged.push(task.id.clone());
        } else if db.settings.forecast_flagged && task.flagged {
            result.flagged.push(task.id.clone());
        }
    }
    let by_title = |db: &Db, id: &TaskId| {
        db.tasks
            .get(id)
            .map(|task| task.title.to_lowercase())
            .unwrap_or_default()
    };
    result.tagged.sort_by_key(|id| by_title(db, id));
    result.flagged.sort_by_key(|id| by_title(db, id));

    let project_key = |db: &Db, id: &ProjectId| {
        db.projects.get(id).map(|project| {
            (
                project
                    .due
                    .or(project.planned)
                    .or(project.defer)
                    .map(|stamp| stamp.at),
                project.name.to_lowercase(),
            )
        })
    };
    for day in result.projects.values_mut() {
        day.due.sort_by_key(|id| project_key(db, id));
        day.deferred.sort_by_key(|id| project_key(db, id));
        day.planned.sort_by_key(|id| project_key(db, id));
    }

    let by_due = |db: &Db, id: &TaskId| db.effective_due(id).map(|stamp| stamp.at);
    let by_planned = |db: &Db, id: &TaskId| db.effective_planned(id).map(|stamp| stamp.at);
    result.overdue.sort_by_key(|id| by_due(db, id));
    result.today.sort_by_key(|id| by_due(db, id));
    result.tomorrow.sort_by_key(|id| by_due(db, id));
    result.later.sort_by_key(|id| {
        db.effective_due(id)
            .or_else(|| db.effective_planned(id))
            .map(|stamp| stamp.at)
    });
    result.later.dedup();
    result.next_month.sort_by_key(|id| {
        db.effective_due(id)
            .or_else(|| db.effective_planned(id))
            .map(|stamp| stamp.at)
    });
    result.next_month.dedup();
    for day in result.days.values_mut() {
        day.due.sort_by_key(|id| by_due(db, id));
        day.deferred
            .sort_by_key(|id| db.effective_defer(id).map(|stamp| stamp.at));
        day.planned.sort_by_key(|id| by_planned(db, id));
    }
    result
        .deferred_today
        .sort_by_key(|id| db.effective_defer(id).map(|stamp| stamp.at));
    result
        .planned_today
        .sort_by_key(|id| by_planned(db, id));
    result
        .past_planned
        .sort_by_key(|id| by_planned(db, id));
    result
}

pub fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    let delta = (date - today).num_days();
    match delta {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        _ => format!(
            "{} {}",
            weekday_short(date.weekday()),
            date.format("%b %-d")
        ),
    }
}
