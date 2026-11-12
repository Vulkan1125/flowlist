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
}

#[derive(Clone, Debug, Default)]
pub struct Forecast {
    pub overdue: Vec<TaskId>,
    pub today: Vec<TaskId>,
    pub tomorrow: Vec<TaskId>,
    pub days: BTreeMap<NaiveDate, ForecastDay>,
    pub later: Vec<TaskId>,
    pub deferred_today: Vec<TaskId>,
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
        }
        out.extend(self.later.iter().cloned());
        out
    }
}

/// Build the Forecast view. Completed tasks and tasks in done/dropped/on-hold
/// projects are excluded; deferred items appear on their defer date.
pub fn forecast(db: &Db, at: DateTime<Utc>) -> Forecast {
    let today = local_date(at);
    let mut result = Forecast::default();

    for task in db.tasks.values() {
        if task.completed {
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
            } else if date == today + chrono::Duration::days(1) {
                result.tomorrow.push(task.id.clone());
            } else if date <= today + chrono::Duration::days(7) {
                let day = result
                    .days
                    .entry(date)
                    .or_insert_with(|| ForecastDay {
                        date,
                        ..ForecastDay::default()
                    });
                day.due.push(task.id.clone());
            } else {
                result.later.push(task.id.clone());
            }
        } else if let Some(defer) = task.defer {
            let date = defer.local_date();
            if date == today {
                result.deferred_today.push(task.id.clone());
            } else if (today..=today + chrono::Duration::days(7)).contains(&date) {
                let day = result
                    .days
                    .entry(date)
                    .or_insert_with(|| ForecastDay {
                        date,
                        ..ForecastDay::default()
                    });
                day.deferred.push(task.id.clone());
            }
        }
    }

    let by_due = |db: &Db, id: &TaskId| db.tasks[id].due.map(|stamp| stamp.at);
    result.overdue.sort_by_key(|id| by_due(db, id));
    result.today.sort_by_key(|id| by_due(db, id));
    result.tomorrow.sort_by_key(|id| by_due(db, id));
    result.later.sort_by_key(|id| by_due(db, id));
    for day in result.days.values_mut() {
        day.due.sort_by_key(|id| by_due(db, id));
        day.deferred.sort_by_key(|id| by_due(db, id));
    }
    result.deferred_today.sort_by_key(|id| {
        db.tasks[id].defer.map(|stamp| stamp.at)
    });
    result
}

pub fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    let delta = (date - today).num_days();
    match delta {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        _ => format!("{} {}", weekday_short(date.weekday()), date.format("%b %-d")),
    }
}
