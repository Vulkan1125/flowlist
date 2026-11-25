//! Forecast buckets relative to the local "today".

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use std::collections::BTreeMap;

use crate::db::Db;
use crate::ids::TaskId;
use crate::model::ProjectStatus;
use crate::time::{local_date, weekday_short};

#[derive(Clone, Debug, Default)]
pub struct ForecastDay {
    pub date: NaiveDate,
    pub due: Vec<TaskId>,
    pub deferred: Vec<TaskId>,
    pub planned: Vec<TaskId>,
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

        if let Some(due) = task.due {
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
        if let Some(defer) = task.defer {
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

    let by_due = |db: &Db, id: &TaskId| db.tasks[id].due.map(|stamp| stamp.at);
    let by_planned = |db: &Db, id: &TaskId| db.effective_planned(id).map(|stamp| stamp.at);
    result.overdue.sort_by_key(|id| by_due(db, id));
    result.today.sort_by_key(|id| by_due(db, id));
    result.tomorrow.sort_by_key(|id| by_due(db, id));
    result.later.sort_by_key(|id| {
        db.tasks[id]
            .due
            .or_else(|| db.effective_planned(id))
            .map(|stamp| stamp.at)
    });
    result.later.dedup();
    result.next_month.sort_by_key(|id| {
        db.tasks[id]
            .due
            .or(db.tasks[id].planned)
            .map(|stamp| stamp.at)
    });
    result.next_month.dedup();
    for day in result.days.values_mut() {
        day.due.sort_by_key(|id| by_due(db, id));
        day.deferred
            .sort_by_key(|id| db.tasks[id].defer.map(|stamp| stamp.at));
        day.planned.sort_by_key(|id| by_planned(db, id));
    }
    result
        .deferred_today
        .sort_by_key(|id| db.tasks[id].defer.map(|stamp| stamp.at));
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
