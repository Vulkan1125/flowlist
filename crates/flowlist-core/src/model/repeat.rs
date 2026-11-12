//! Repeat rules and calendar-aware next-occurrence calculation.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::time::{IntervalUnit, Stamp, add_interval_to_stamp, local_date};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    /// Repeat from the previous due date and keep the due time.
    #[default]
    DueAgain,
    /// Repeat from the previous defer date and keep the defer-to-due lead.
    DeferAnother,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepeatSchedule {
    /// The next date is derived from the previous scheduled date.
    #[default]
    Fixed,
    /// The next date is derived from when the task was completed.
    AfterCompletion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "date", rename_all = "snake_case")]
pub enum RepeatEnd {
    Never,
    OnDate(NaiveDate),
    AfterCount(u32),
}

impl Default for RepeatEnd {
    fn default() -> Self {
        RepeatEnd::Never
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatRule {
    pub mode: RepeatMode,
    pub schedule: RepeatSchedule,
    #[serde(default = "one")]
    pub every: u32,
    pub unit: IntervalUnit,
    #[serde(default)]
    pub end: RepeatEnd,
    #[serde(default)]
    pub completed_count: u32,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatOutcome {
    pub due: Option<Stamp>,
    pub defer: Option<Stamp>,
    pub completed_count: u32,
}

impl RepeatRule {
    pub fn new(mode: RepeatMode, every: u32, unit: IntervalUnit) -> Self {
        Self {
            mode,
            schedule: RepeatSchedule::Fixed,
            every: every.max(1),
            unit,
            end: RepeatEnd::Never,
            completed_count: 0,
        }
    }

    pub fn description(&self) -> String {
        let schedule = match self.schedule {
            RepeatSchedule::Fixed => String::new(),
            RepeatSchedule::AfterCompletion => " after completion".to_string(),
        };
        let mode = match self.mode {
            RepeatMode::DueAgain => "",
            RepeatMode::DeferAnother => " (defer another)",
        };
        let end = match self.end {
            RepeatEnd::Never => String::new(),
            RepeatEnd::OnDate(date) => format!(" until {}", date.format("%Y-%m-%d")),
            RepeatEnd::AfterCount(count) => format!(" for {count} times"),
        };
        format!(
            "{}{schedule}{mode}{end}",
            self.unit.label(self.every.max(1))
        )
    }

    /// The next occurrence after completing the task. `None` means the rule
    /// has ended (date reached or occurrence count exhausted).
    pub fn next_occurrence(
        &self,
        due: Option<Stamp>,
        defer: Option<Stamp>,
        completed_at: DateTime<Utc>,
    ) -> Option<RepeatOutcome> {
        if !matches!(self.end, RepeatEnd::Never) {
            let next_count = self.completed_count.saturating_add(1);
            if let RepeatEnd::AfterCount(limit) = self.end {
                if next_count >= limit {
                    return None;
                }
            }
        }

        let lead_days = match (due, defer) {
            (Some(due), Some(defer)) => (due.local_date() - defer.local_date()).num_days(),
            _ => 0,
        };
        let completed_local = local_date(completed_at);
        let every = self.every.max(1);

        let (new_due, new_defer) = match (self.schedule, self.mode) {
            (RepeatSchedule::Fixed, RepeatMode::DueAgain) => {
                let new_due = due.map(|value| add_interval_to_stamp(value, every, self.unit));
                let new_defer = defer.map(|value| add_interval_to_stamp(value, every, self.unit));
                (new_due, new_defer)
            }
            (RepeatSchedule::Fixed, RepeatMode::DeferAnother) => {
                let base_defer = defer.or(due);
                let new_defer =
                    base_defer.map(|value| add_interval_to_stamp(value, every, self.unit));
                let new_due = match (new_defer, due) {
                    (Some(nd), Some(od)) => {
                        Some(with_time_from(shift_by_days(nd, lead_days), od))
                    }
                    _ => None,
                };
                (new_due, new_defer)
            }
            (RepeatSchedule::AfterCompletion, RepeatMode::DueAgain) => {
                let anchor_date = add_interval(completed_local, every, self.unit);
                let new_due = due.map(|value| stamp_on(value, anchor_date));
                let new_defer = match (new_due, defer) {
                    (Some(nd), Some(defer)) => Some(shift_by_days(nd, -lead_days))
                        .map(|stamp| with_time_from(stamp, defer)),
                    _ => None,
                };
                (new_due, new_defer)
            }
            (RepeatSchedule::AfterCompletion, RepeatMode::DeferAnother) => {
                let anchor_date = add_interval(completed_local, every, self.unit);
                let base = defer.or(due);
                let new_defer = base.map(|value| stamp_on(value, anchor_date));
                let new_due = match (new_defer, due) {
                    (Some(nd), Some(od)) => {
                        Some(with_time_from(shift_by_days(nd, lead_days), od))
                    }
                    _ => None,
                };
                (new_due, new_defer)
            }
        };

        if new_due.is_none() && new_defer.is_none() {
            return None;
        }

        if let RepeatEnd::OnDate(end) = self.end {
            let anchor = new_defer
                .or(new_due)
                .map(|stamp| stamp.local_date())
                .unwrap_or(completed_local);
            if anchor > end {
                return None;
            }
        }

        Some(RepeatOutcome {
            due: new_due,
            defer: new_defer,
            completed_count: self.completed_count.saturating_add(1),
        })
    }
}

fn add_interval(date: NaiveDate, every: u32, unit: IntervalUnit) -> NaiveDate {
    crate::time::add_interval(date, every, unit)
}

fn shift_by_days(stamp: Stamp, days: i64) -> Stamp {
    let date = stamp.local_date() + Duration::days(days);
    if stamp.has_time {
        Stamp::on(date, stamp.local_time())
    } else {
        Stamp::date_only(date)
    }
}

fn stamp_on(sample: Stamp, date: NaiveDate) -> Stamp {
    if sample.has_time {
        Stamp::on(date, sample.local_time())
    } else {
        Stamp::date_only(date)
    }
}

fn with_time_from(stamp: Stamp, sample: Stamp) -> Stamp {
    if sample.has_time {
        Stamp::on(stamp.local_date(), sample.local_time())
    } else {
        Stamp::date_only(stamp.local_date())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveTime, TimeZone};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn fixed_due_again_advances_due() {
        let due = Stamp::date_only(date(2026, 11, 12));
        let rule = RepeatRule {
            mode: RepeatMode::DueAgain,
            schedule: RepeatSchedule::Fixed,
            every: 1,
            unit: IntervalUnit::Weeks,
            end: RepeatEnd::Never,
            completed_count: 0,
        };
        let next = rule
            .next_occurrence(Some(due), None, Utc::now())
            .unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 11, 19));
        assert_eq!(next.completed_count, 1);
    }

    #[test]
    fn defer_another_preserves_lead() {
        let due = Stamp::on(date(2026, 11, 20), NaiveTime::from_hms_opt(17, 0, 0).unwrap());
        let defer = Stamp::date_only(date(2026, 11, 15));
        let rule = RepeatRule {
            mode: RepeatMode::DeferAnother,
            schedule: RepeatSchedule::Fixed,
            every: 2,
            unit: IntervalUnit::Weeks,
            end: RepeatEnd::Never,
            completed_count: 0,
        };
        let next = rule
            .next_occurrence(Some(due), Some(defer), Utc::now())
            .unwrap();
        assert_eq!(next.defer.unwrap().local_date(), date(2026, 11, 29));
        assert_eq!(next.due.unwrap().local_date(), date(2026, 12, 4));
    }

    #[test]
    fn end_conditions_stop_repeats() {
        let due = Stamp::date_only(date(2026, 11, 12));
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Months);
        rule.end = RepeatEnd::AfterCount(2);
        rule.completed_count = 1;
        assert!(
            rule.next_occurrence(Some(due), None, Utc::now())
                .is_none()
        );

        rule.end = RepeatEnd::OnDate(date(2026, 11, 30));
        rule.completed_count = 0;
        assert!(
            rule.next_occurrence(Some(Stamp::date_only(date(2026, 12, 1))), None, Utc::now())
                .is_none()
        );
        let _ = Utc.timestamp_opt(0, 0);
    }
}
