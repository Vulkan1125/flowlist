//! Repeat rules and calendar-aware next-occurrence calculation.

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::time::{IntervalUnit, Stamp, add_interval_to_stamp, local_date, local_naive};

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

/// A calendar pattern for monthly repeats. Patterns take precedence over the
/// weekday filter and always count months, so the same rule can say "the third
/// Thursday of every month" or "every two months on the last weekend day".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepeatPattern {
    /// The nth weekday of the month, with `nth` from 1 through 5. A month
    /// without a fifth occurrence is skipped.
    NthWeekday { nth: u8, weekday: u8 },
    /// The latest Saturday or Sunday in the month.
    LastWeekendDay,
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
    /// Optional weekday filter, with 0 = Sunday through 6 = Saturday. When set,
    /// the rule repeats on those weekdays instead of every N days.
    #[serde(default)]
    pub weekdays: Option<Vec<u8>>,
    /// Optional monthly calendar pattern, such as the third Thursday or the
    /// last weekend day. When set it replaces the ordinary interval cadence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<RepeatPattern>,
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
            weekdays: None,
            pattern: None,
        }
    }

    /// Sub-day units advance the clock, so they need a timed anchor.
    pub fn uses_sub_day(&self) -> bool {
        self.pattern.is_none() && matches!(self.unit, IntervalUnit::Minutes | IntervalUnit::Hours)
    }

    pub fn selected_weekdays(&self) -> BTreeSet<u8> {
        self.weekdays
            .as_ref()
            .map(|days| days.iter().copied().filter(|day| *day < 7).collect())
            .unwrap_or_default()
    }

    pub fn set_selected_weekdays(&mut self, days: BTreeSet<u8>) {
        if days.is_empty() || days.len() >= 7 {
            self.weekdays = None;
        } else {
            self.weekdays = Some(days.into_iter().collect());
        }
    }

    pub fn weekday_summary(&self) -> Option<String> {
        let days = self.selected_weekdays();
        if days.is_empty() {
            return None;
        }
        let names: Vec<&str> = days
            .iter()
            .map(|day| match day {
                0 => "Sunday",
                1 => "Monday",
                2 => "Tuesday",
                3 => "Wednesday",
                4 => "Thursday",
                5 => "Friday",
                _ => "Saturday",
            })
            .collect();
        let joined = match names.as_slice() {
            [] => String::new(),
            [one] => one.to_string(),
            [first, second] => format!("{first} and {second}"),
            _ => format!(
                "{}, and {}",
                names[..names.len() - 1].join(", "),
                names[names.len() - 1]
            ),
        };
        Some(format!("every {joined}"))
    }

    /// The monthly calendar pattern, phrased for the inspector and the row
    /// metadata: "the third Thursday of every month" or "the last weekend day
    /// of every two months".
    pub fn pattern_summary(&self) -> Option<String> {
        let pattern = self.pattern?;
        let every = self.every.max(1);
        let months = if every == 1 {
            "every month".to_string()
        } else {
            format!("every {every} months")
        };
        Some(match pattern {
            RepeatPattern::NthWeekday { nth, weekday } => {
                let day = weekday_name(weekday % 7)?;
                format!("the {} {day} of {months}", ordinal(nth.clamp(1, 5)))
            }
            RepeatPattern::LastWeekendDay => {
                format!("the last weekend day of {months}")
            }
        })
    }

    /// A plain-language sentence for the inspector. It is derived only from
    /// the controls above it, so the UI never has to ask the user to read the
    /// compact rule syntax.
    pub fn summary(&self) -> String {
        let cadence = self
            .pattern_summary()
            .or_else(|| self.weekday_summary())
            .unwrap_or_else(|| self.unit.label(self.every.max(1)));
        let every = format!("{}{}", cadence[..1].to_uppercase(), &cadence[1..]);
        let origin = match self.schedule {
            RepeatSchedule::Fixed => "from the assigned dates",
            RepeatSchedule::AfterCompletion => "from completion",
        };
        let next = match self.mode {
            RepeatMode::DueAgain => "the next due date",
            RepeatMode::DeferAnother => "the next defer date",
        };
        let end = match self.end {
            RepeatEnd::Never => String::new(),
            RepeatEnd::OnDate(date) => format!(" and ends on {}", date.format("%b %-d, %Y")),
            RepeatEnd::AfterCount(count) => {
                format!(" and ends after {count} occurrence{}", if count == 1 { "" } else { "s" })
            }
        };
        format!("{every}, {origin}, scheduling {next}{end}.")
    }

    /// The date of the last occurrence this rule will produce, when the
    /// schedule makes it predictable. After-completion rules depend on when
    /// the user completes each copy, so they return None.
    pub fn projected_last_date(&self, due: Option<Stamp>, defer: Option<Stamp>) -> Option<NaiveDate> {
        match self.end {
            RepeatEnd::OnDate(date) => Some(date),
            RepeatEnd::Never => None,
            RepeatEnd::AfterCount(limit) => {
                if self.completed_count >= limit {
                    return None;
                }
                if !matches!(self.schedule, RepeatSchedule::Fixed) {
                    return None;
                }
                let remaining = limit - self.completed_count;
                let anchor = match self.mode {
                    RepeatMode::DueAgain => due.or(defer),
                    RepeatMode::DeferAnother => defer.or(due),
                }?;
                let selected = self.selected_weekdays();
                if let Some(pattern) = self.pattern {
                    let mut date = anchor.local_date();
                    for _ in 1..remaining {
                        date = next_pattern_after(date, self.every.max(1), pattern);
                    }
                    return Some(date);
                }
                if selected.is_empty() && self.uses_sub_day() {
                    let mut stamp = anchor;
                    for _ in 1..remaining {
                        stamp = add_interval_to_stamp(stamp, self.every.max(1), self.unit);
                    }
                    return Some(stamp.local_date());
                }
                let mut date = anchor.local_date();
                for _ in 1..remaining {
                    date = if selected.is_empty() {
                        add_interval(date, self.every.max(1), self.unit)
                    } else {
                        next_weekday_after(date, &selected)
                    };
                }
                Some(date)
            }
        }
    }

    /// The plain-language line for the inspector, with the ending folded in:
    /// how many occurrences are done and, where possible, the last date.
    pub fn summary_with_dates(&self, due: Option<Stamp>, defer: Option<Stamp>) -> String {
        let cadence = self
            .pattern_summary()
            .or_else(|| self.weekday_summary())
            .unwrap_or_else(|| self.unit.label(self.every.max(1)));
        let every = format!("{}{}", cadence[..1].to_uppercase(), &cadence[1..]);
        let origin = match self.schedule {
            RepeatSchedule::Fixed => "from the assigned dates",
            RepeatSchedule::AfterCompletion => "from completion",
        };
        let next = match self.mode {
            RepeatMode::DueAgain => "the next due date",
            RepeatMode::DeferAnother => "the next defer date",
        };
        let end = match self.end {
            RepeatEnd::Never => String::new(),
            RepeatEnd::OnDate(date) => format!(" and ends on {}", date.format("%b %-d, %Y")),
            RepeatEnd::AfterCount(limit) => {
                let done = self.completed_count.min(limit);
                let mut text = format!(
                    " and ends after {limit} occurrence{}",
                    if limit == 1 { "" } else { "s" }
                );
                if done > 0 {
                    text.push_str(&format!(" ({done} done)"));
                }
                if let Some(date) = self.projected_last_date(due, defer) {
                    text.push_str(&format!(", last on {}", date.format("%b %-d, %Y")));
                } else if matches!(self.schedule, RepeatSchedule::AfterCompletion) {
                    text.push_str(", dates follow completion");
                }
                text
            }
        };
        format!("{every}, {origin}, scheduling {next}{end}.")
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
        let cadence = self
            .pattern_summary()
            .or_else(|| self.weekday_summary())
            .unwrap_or_else(|| self.unit.label(self.every.max(1)));
        format!("{cadence}{schedule}{mode}{end}")
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

        let selected_weekdays = self.selected_weekdays();
        let calendar_dates = if self.pattern.is_none() && selected_weekdays.is_empty() {
            None
        } else if due.is_none() && defer.is_none() {
            None
        } else {
            let anchor = match self.schedule {
                RepeatSchedule::AfterCompletion => Some(completed_local),
                RepeatSchedule::Fixed => match self.mode {
                    RepeatMode::DueAgain => due.or(defer).map(|d| d.local_date()),
                    RepeatMode::DeferAnother => defer.or(due).map(|d| d.local_date()),
                },
            };
            anchor.map(|anchor| {
                let next_date = match self.pattern {
                    Some(pattern) => next_pattern_after(anchor, every, pattern),
                    None => next_weekday_after(anchor, &selected_weekdays),
                };
                (next_date, anchor)
            })
        };
        let sub_day_after_completion =
            self.uses_sub_day() && matches!(self.schedule, RepeatSchedule::AfterCompletion);
        let (new_due, new_defer) = if let Some((next_date, _anchor_date)) = calendar_dates {
            match self.mode {
                RepeatMode::DueAgain => {
                    let new_due = due.map(|value| stamp_on(value, next_date));
                    let new_defer = match (due, defer) {
                        (Some(old_due), Some(defer)) => {
                            let shift = (next_date - old_due.local_date()).num_days();
                            Some(with_time_from(shift_by_days(defer, shift), defer))
                        }
                        (None, Some(defer)) => Some(with_time_from(
                            Stamp::date_only(next_date),
                            defer,
                        )),
                        _ => None,
                    };
                    (new_due, new_defer)
                }
                RepeatMode::DeferAnother => {
                    let base = defer.or(due);
                    let new_defer = base.map(|value| stamp_on(value, next_date));
                    let new_due = match (new_defer, due) {
                        (Some(nd), Some(od)) => {
                            Some(with_time_from(shift_by_days(nd, lead_days), od))
                        }
                        _ => None,
                    };
                    (new_due, new_defer)
                }
            }
        } else if sub_day_after_completion {
            // Sub-day rules move from the moment of completion, not from the
            // calendar day, so an hourly task completed at 09:40 is next due
            // at 10:40 rather than staying on the same date.
            let completed_stamp = Stamp::from_local(local_naive(completed_at));
            match self.mode {
                RepeatMode::DueAgain => {
                    let new_due =
                        due.map(|_| add_interval_to_stamp(completed_stamp, every, self.unit));
                    let new_defer = match (new_due, defer) {
                        (Some(nd), Some(defer)) => {
                            Some(with_time_from(shift_by_days(nd, -lead_days), defer))
                        }
                        _ => None,
                    };
                    (new_due, new_defer)
                }
                RepeatMode::DeferAnother => {
                    let base = defer.or(due);
                    let new_defer =
                        base.map(|_| add_interval_to_stamp(completed_stamp, every, self.unit));
                    let new_due = match (new_defer, due) {
                        (Some(nd), Some(od)) => {
                            Some(with_time_from(shift_by_days(nd, lead_days), od))
                        }
                        _ => None,
                    };
                    (new_due, new_defer)
                }
            }
        } else {
            match (self.schedule, self.mode) {
                (RepeatSchedule::Fixed, RepeatMode::DueAgain) => {
                    let new_due = due.map(|value| add_interval_to_stamp(value, every, self.unit));
                    let new_defer =
                        defer.map(|value| add_interval_to_stamp(value, every, self.unit));
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
            }
        };

        // A repeating task without any dates still recurs: completing it simply
        // produces a fresh copy. Only a rule whose dateless copy cannot be
        // scheduled (because the original had dates) means the rule has ended.
        if new_due.is_none() && new_defer.is_none() && (due.is_some() || defer.is_some()) {
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

fn weekday_name(index: u8) -> Option<&'static str> {
    Some(match index {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => return None,
    })
}

fn ordinal(nth: u8) -> &'static str {
    match nth {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        _ => "fifth",
    }
}

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let next = first.checked_add_months(Months::new(1))?;
    Some(next - Duration::days(1))
}

fn pattern_date_in_month(year: i32, month: u32, pattern: RepeatPattern) -> Option<NaiveDate> {
    match pattern {
        RepeatPattern::NthWeekday { nth, weekday } => {
            let first = NaiveDate::from_ymd_opt(year, month, 1)?;
            let first_weekday = first.weekday().num_days_from_sunday() as u8;
            let weekday = weekday % 7;
            let day = 1
                + usize::from((weekday + 7 - first_weekday) % 7)
                + usize::from(nth.clamp(1, 5) - 1) * 7;
            let date = NaiveDate::from_ymd_opt(year, month, day as u32)?;
            (date.month() == month).then_some(date)
        }
        RepeatPattern::LastWeekendDay => {
            let mut date = last_day_of_month(year, month)?;
            while !matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun) {
                date -= Duration::days(1);
            }
            Some(date)
        }
    }
}

/// The first matching date strictly after `anchor`, keeping the rule's month
/// phase so "every two months on the third Thursday" stays on odd months.
fn next_pattern_after(anchor: NaiveDate, every: u32, pattern: RepeatPattern) -> NaiveDate {
    let every = every.max(1);
    let start = NaiveDate::from_ymd_opt(anchor.year(), anchor.month(), 1).unwrap_or(anchor);
    // Visit only the months this rule can occupy, so a large interval such as
    // "every 500 months" still finds its next pattern instead of falling back
    // to the day after the anchor.
    for step in 0..=240u32 {
        let offset = step.saturating_mul(every);
        let Some(cursor) = start.checked_add_months(Months::new(offset)) else {
            break;
        };
        if let Some(date) = pattern_date_in_month(cursor.year(), cursor.month(), pattern)
            && date > anchor
        {
            return date;
        }
    }
    anchor + Duration::days(1)
}

fn next_weekday_after(anchor: NaiveDate, selected: &BTreeSet<u8>) -> NaiveDate {
    let mut candidate = anchor + Duration::days(1);
    for _ in 0..14 {
        if selected.contains(&(candidate.weekday().num_days_from_sunday() as u8)) {
            return candidate;
        }
        candidate += Duration::days(1);
    }
    candidate
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

/// The title for a task's next occurrence. The last number in the title is
/// incremented, so “Week 3 review” becomes “Week 4 review” and “Invoice 007”
/// becomes “Invoice 008”; a title without a number is copied unchanged.
pub fn increment_repeat_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut end = None;
    let mut start = None;
    for (index, ch) in chars.iter().enumerate().rev() {
        if ch.is_ascii_digit() {
            if end.is_none() {
                end = Some(index + 1);
            }
            start = Some(index);
        } else if end.is_some() {
            break;
        }
    }
    let (Some(start), Some(end)) = (start, end) else {
        return title.to_string();
    };
    let digits: String = chars[start..end].iter().collect();
    let Ok(value) = digits.parse::<u64>() else {
        return title.to_string();
    };
    let next = value.saturating_add(1);
    // Keep zero padding so “Sprint 007” becomes “Sprint 008”, not “Sprint 8”.
    let replacement = if digits.len() > 1 && digits.starts_with('0') {
        format!("{next:0width$}", width = digits.len())
    } else {
        next.to_string()
    };
    let mut out: String = chars[..start].iter().collect();
    out.push_str(&replacement);
    out.extend(chars[end..].iter());
    out
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
            weekdays: None,
            pattern: None,
        };
        let next = rule.next_occurrence(Some(due), None, Utc::now()).unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 11, 19));
        assert_eq!(next.completed_count, 1);
    }

    #[test]
    fn defer_another_preserves_lead() {
        let due = Stamp::on(
            date(2026, 11, 20),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        );
        let defer = Stamp::date_only(date(2026, 11, 15));
        let rule = RepeatRule {
            mode: RepeatMode::DeferAnother,
            schedule: RepeatSchedule::Fixed,
            every: 2,
            unit: IntervalUnit::Weeks,
            end: RepeatEnd::Never,
            completed_count: 0,
            weekdays: None,
            pattern: None,
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
        assert!(rule.next_occurrence(Some(due), None, Utc::now()).is_none());

        rule.end = RepeatEnd::OnDate(date(2026, 11, 30));
        rule.completed_count = 0;
        assert!(
            rule.next_occurrence(Some(Stamp::date_only(date(2026, 12, 1))), None, Utc::now())
                .is_none()
        );
        let _ = Utc.timestamp_opt(0, 0);
    }

    #[test]
    fn weekday_filters_advance_to_the_next_selected_day() {
        let due = Stamp::date_only(date(2026, 11, 23)); // Monday
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Days);
        rule.set_selected_weekdays(BTreeSet::from([1, 3])); // Monday, Wednesday
        let next = rule.next_occurrence(Some(due), None, Utc::now()).unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 11, 25));
        assert_eq!(
            rule.summary(),
            "Every Monday and Wednesday, from the assigned dates, scheduling the next due date."
        );
    }

    #[test]
    fn nth_weekday_patterns_advance_by_month() {
        let due = Stamp::date_only(date(2026, 11, 19)); // third Thursday
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Months);
        rule.pattern = Some(RepeatPattern::NthWeekday { nth: 3, weekday: 4 });
        let next = rule.next_occurrence(Some(due), None, Utc::now()).unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 12, 17));
        assert_eq!(
            rule.summary(),
            "The third Thursday of every month, from the assigned dates, scheduling the next due date."
        );

        rule.every = 2;
        let next = rule.next_occurrence(Some(due), None, Utc::now()).unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2027, 1, 21));
    }

    #[test]
    fn large_pattern_intervals_keep_their_month_phase() {
        let due_date = date(2026, 11, 19); // third Thursday
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 500, IntervalUnit::Months);
        rule.pattern = Some(RepeatPattern::NthWeekday {
            nth: 3,
            weekday: 4,
        });
        let next = rule
            .next_occurrence(Some(Stamp::date_only(due_date)), None, Utc::now())
            .and_then(|outcome| outcome.due)
            .expect("next occurrence")
            .local_date();
        let expected_month = due_date
            .checked_add_months(Months::new(500))
            .expect("month arithmetic");
        assert_eq!(next.year(), expected_month.year());
        assert_eq!(next.month(), expected_month.month());
    }

    #[test]
    fn last_weekend_day_lands_on_the_last_saturday_or_sunday() {
        let november = Stamp::date_only(date(2026, 11, 1));
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Months);
        rule.pattern = Some(RepeatPattern::LastWeekendDay);
        let next = rule
            .next_occurrence(Some(november), None, Utc::now())
            .unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 11, 29));
        let next = rule.next_occurrence(next.due, None, Utc::now()).unwrap();
        assert_eq!(next.due.unwrap().local_date(), date(2026, 12, 27));
        assert_eq!(rule.description(), "the last weekend day of every month");
    }

    #[test]
    fn sub_day_after_completion_moves_from_the_completion_clock() {
        let date = date(2026, 11, 28);
        let due = Stamp::on(date, NaiveTime::from_hms_opt(9, 40, 0).unwrap());
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Hours);
        rule.schedule = RepeatSchedule::AfterCompletion;
        let next = rule.next_occurrence(Some(due), None, due.at).unwrap();
        let next = next.due.unwrap();
        assert_eq!(next.local_date(), date);
        assert_eq!(
            next.local_time(),
            NaiveTime::from_hms_opt(10, 40, 0).unwrap()
        );
    }

    #[test]
    fn summary_reads_like_the_inspector_copy() {
        let mut rule = RepeatRule::new(RepeatMode::DeferAnother, 2, IntervalUnit::Weeks);
        rule.schedule = RepeatSchedule::AfterCompletion;
        rule.end = RepeatEnd::AfterCount(5);
        assert_eq!(
            rule.summary(),
            "Every 2 weeks, from completion, scheduling the next defer date and ends after 5 occurrences."
        );
        let rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Months);
        assert_eq!(
            rule.summary(),
            "Every month, from the assigned dates, scheduling the next due date."
        );
    }

    #[test]
    fn repeat_titles_increment_their_last_number() {
        assert_eq!(increment_repeat_title("Week 3 review"), "Week 4 review");
        assert_eq!(increment_repeat_title("Invoice 007"), "Invoice 008");
        assert_eq!(increment_repeat_title("Day 9"), "Day 10");
        assert_eq!(
            increment_repeat_title("Nothing numbered"),
            "Nothing numbered"
        );
        assert_eq!(increment_repeat_title("Version 1.4"), "Version 1.5");
    }
}

#[cfg(test)]
mod projected_tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn after_count_projects_the_last_fixed_date() {
        let due = Stamp::date_only(NaiveDate::from_ymd_opt(2026, 1, 5).unwrap());
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 2, IntervalUnit::Weeks);
        rule.end = RepeatEnd::AfterCount(3);
        assert_eq!(
            rule.projected_last_date(Some(due), None),
            NaiveDate::from_ymd_opt(2026, 2, 2)
        );
        rule.completed_count = 1;
        assert_eq!(
            rule.projected_last_date(Some(due), None),
            NaiveDate::from_ymd_opt(2026, 1, 19)
        );
        rule.schedule = RepeatSchedule::AfterCompletion;
        assert_eq!(rule.projected_last_date(Some(due), None), None);
    }

    #[test]
    fn after_count_projects_sub_day_intervals_across_midnight() {
        let due = Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 28).unwrap(),
            NaiveTime::from_hms_opt(23, 30, 0).unwrap(),
        );
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 2, IntervalUnit::Hours);
        rule.end = RepeatEnd::AfterCount(3);
        assert_eq!(
            rule.projected_last_date(Some(due), None),
            Some(NaiveDate::from_ymd_opt(2026, 11, 29).unwrap())
        );
    }

    #[test]
    fn after_count_projects_monthly_patterns() {
        let due = Stamp::date_only(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 1, IntervalUnit::Months);
        rule.pattern = Some(RepeatPattern::NthWeekday { nth: 3, weekday: 4 });
        rule.end = RepeatEnd::AfterCount(3);
        assert_eq!(
            rule.projected_last_date(Some(due), None),
            Some(NaiveDate::from_ymd_opt(2026, 3, 19).unwrap())
        );
    }
}

#[cfg(test)]
mod merged_summary_tests {
    use super::*;

    #[test]
    fn the_ending_is_part_of_one_line() {
        let due = Stamp::date_only(NaiveDate::from_ymd_opt(2026, 1, 5).unwrap());
        let mut rule = RepeatRule::new(RepeatMode::DueAgain, 2, IntervalUnit::Weeks);
        rule.end = RepeatEnd::AfterCount(3);
        rule.completed_count = 1;
        assert_eq!(
            rule.summary_with_dates(Some(due), None),
            "Every 2 weeks, from the assigned dates, scheduling the next due date and ends after 3 occurrences (1 done), last on Jan 19, 2026."
        );
    }
}
