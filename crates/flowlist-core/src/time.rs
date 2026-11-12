//! Time primitives.
//!
//! Times are stored as UTC instants plus a `has_time` flag. Date-only values
//! are anchored at local midnight and displayed as dates; timestamped values
//! keep their local wall-clock time. All bucketing and human wording happens
//! in the user's local timezone.

use chrono::{
    DateTime, Datelike, Duration, Local, LocalResult, Months, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone, Utc, Weekday,
};
use serde::{Deserialize, Serialize};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn local_now() -> DateTime<Local> {
    Local::now()
}

pub fn local_date(at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&Local).date_naive()
}

pub fn local_time(at: DateTime<Utc>) -> NaiveTime {
    at.with_timezone(&Local).time()
}

pub fn local_naive(at: DateTime<Utc>) -> NaiveDateTime {
    at.with_timezone(&Local).naive_local()
}

/// Convert a local wall-clock date and time to a UTC instant, handling DST
/// gaps by moving forward one hour and DST overlaps by taking the first
/// occurrence.
pub fn local_to_utc(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let naive = date.and_time(time);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt.with_timezone(&Utc),
        LocalResult::Ambiguous(first, _second) => first.with_timezone(&Utc),
        LocalResult::None => {
            let shifted = naive + Duration::hours(1);
            match Local.from_local_datetime(&shifted) {
                LocalResult::Single(dt) => dt.with_timezone(&Utc),
                LocalResult::Ambiguous(first, _) => first.with_timezone(&Utc),
                LocalResult::None => Utc.from_utc_datetime(&naive),
            }
        }
    }
}

pub fn start_of_day(date: NaiveDate) -> DateTime<Utc> {
    local_to_utc(date, NaiveTime::MIN)
}

/// A user-facing date: either date-only or a date with a local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub has_time: bool,
}

impl Stamp {
    pub fn date_only(date: NaiveDate) -> Self {
        Self {
            at: start_of_day(date),
            has_time: false,
        }
    }

    pub fn on(date: NaiveDate, time: NaiveTime) -> Self {
        Self {
            at: local_to_utc(date, time),
            has_time: true,
        }
    }

    pub fn from_local(naive: NaiveDateTime) -> Self {
        Self {
            at: local_to_utc(naive.date(), naive.time()),
            has_time: true,
        }
    }

    pub fn local_date(&self) -> NaiveDate {
        local_date(self.at)
    }

    pub fn local_time(&self) -> NaiveTime {
        local_time(self.at)
    }

    pub fn local_naive(&self) -> NaiveDateTime {
        local_naive(self.at)
    }

    pub fn to_local(&self) -> DateTime<Local> {
        self.at.with_timezone(&Local)
    }

    /// A compact phrase used by the CLI, inspector and list rows.
    pub fn label(&self, reference: DateTime<Utc>) -> String {
        human_date(self, reference)
    }

    /// Round-trips as `YYYY-MM-DD` or `YYYY-MM-DD HH:MM` in local time.
    pub fn to_input_string(&self) -> String {
        let naive = self.local_naive();
        if self.has_time {
            naive.format("%Y-%m-%d %H:%M").to_string()
        } else {
            naive.format("%Y-%m-%d").to_string()
        }
    }
}

pub fn human_date(stamp: &Stamp, reference: DateTime<Utc>) -> String {
    let today = local_date(reference);
    let date = stamp.local_date();
    let relative = (date - today).num_days();
    let day = match relative {
        -1 => "Yesterday".to_string(),
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        2..=6 => weekday_name(date.weekday()).to_string(),
        n if n < -1 => format!("{} ({}d ago)", date.format("%b %-d"), -n),
        n if n > 6 && n <= 13 => {
            format!("{} {}", weekday_name(date.weekday()), date.format("%b %-d"))
        }
        _ => {
            if date.year() == today.year() {
                date.format("%b %-d").to_string()
            } else {
                date.format("%b %-d, %Y").to_string()
            }
        }
    };

    if stamp.has_time {
        format!("{day}, {}", stamp.local_time().format("%-I:%M %p"))
    } else {
        day
    }
}

pub fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

pub fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

pub fn parse_weekday(input: &str) -> Option<Weekday> {
    let lower = input.to_ascii_lowercase();
    let lower = lower.trim_end_matches('.');
    match lower {
        "mon" | "monday" | "mo" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" | "tu" => Some(Weekday::Tue),
        "wed" | "weds" | "wednesday" | "we" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" | "th" => Some(Weekday::Thu),
        "fri" | "friday" | "fr" => Some(Weekday::Fri),
        "sat" | "saturday" | "sa" => Some(Weekday::Sat),
        "sun" | "sunday" | "su" => Some(Weekday::Sun),
        _ => None,
    }
}

/// The next occurrence of `target`, at least `min_days` away.
pub fn next_weekday(from: NaiveDate, target: Weekday, strictly_next: bool) -> NaiveDate {
    let current = from.weekday().num_days_from_monday() as i64;
    let wanted = target.num_days_from_monday() as i64;
    let mut delta = (wanted - current).rem_euclid(7);
    if strictly_next && delta == 0 {
        delta = 7;
    }
    if strictly_next && delta > 0 && delta < 7 {
        // "next friday" from a Monday is the Friday of next week, not this one.
        delta += 7;
    }
    from + Duration::days(delta)
}

pub fn start_of_week(date: NaiveDate, week_starts_on: Weekday) -> NaiveDate {
    let current = date.weekday().num_days_from_monday() as i64;
    let start = week_starts_on.num_days_from_monday() as i64;
    let delta = (current - start).rem_euclid(7);
    date - Duration::days(delta)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntervalUnit {
    Days,
    Weeks,
    Months,
    Years,
}

impl IntervalUnit {
    pub fn label(self, every: u32) -> String {
        let unit = match self {
            IntervalUnit::Days => "day",
            IntervalUnit::Weeks => "week",
            IntervalUnit::Months => "month",
            IntervalUnit::Years => "year",
        };
        if every == 1 {
            format!("every {unit}")
        } else {
            format!("every {every} {unit}s")
        }
    }
}

/// Calendar-aware date addition. Months clamp to the end of the target month.
pub fn add_interval(date: NaiveDate, every: u32, unit: IntervalUnit) -> NaiveDate {
    let every = every.max(1);
    match unit {
        IntervalUnit::Days => date + Duration::days(every as i64),
        IntervalUnit::Weeks => date + Duration::days(every as i64 * 7),
        IntervalUnit::Months => date
            .checked_add_months(Months::new(every))
            .unwrap_or(date),
        IntervalUnit::Years => date
            .checked_add_months(Months::new(every.saturating_mul(12)))
            .unwrap_or(date),
    }
}

/// Apply a date interval while preserving wall-clock time-of-day.
pub fn add_interval_to_stamp(stamp: Stamp, every: u32, unit: IntervalUnit) -> Stamp {
    let date = add_interval(stamp.local_date(), every, unit);
    if stamp.has_time {
        Stamp::on(date, stamp.local_time())
    } else {
        Stamp::date_only(date)
    }
}

pub fn format_duration(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    match (hours, mins) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

pub fn format_clock(time: NaiveTime) -> String {
    time.format("%-I:%M %p").to_string()
}

/// Days from `reference`'s local date, negative for the past.
pub fn days_from_today(date: NaiveDate, reference: DateTime<Utc>) -> i64 {
    (date - local_date(reference)).num_days()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_arithmetic_clamps() {
        let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(
            add_interval(jan31, 1, IntervalUnit::Months),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(
            add_interval(leap, 1, IntervalUnit::Years),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn stamps_roundtrip_local_wall_clock() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
        let time = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        let stamp = Stamp::on(date, time);
        assert_eq!(stamp.local_date(), date);
        assert_eq!(stamp.local_time(), time);
        assert!(stamp.has_time);
        assert_eq!(stamp.to_input_string(), "2026-11-12 17:30");
    }

    #[test]
    fn next_weekday_is_strict() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        assert_eq!(
            next_weekday(monday, Weekday::Fri, false),
            NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
        );
        assert_eq!(
            next_weekday(monday, Weekday::Fri, true),
            NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
        );
        assert_eq!(
            next_weekday(monday, Weekday::Mon, true),
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
    }

    #[test]
    fn human_labels_are_relative() {
        let now = Utc::now();
        let today = local_date(now);
        assert_eq!(
            human_date(&Stamp::date_only(today), now),
            "Today".to_string()
        );
        assert_eq!(
            human_date(&Stamp::date_only(today + Duration::days(1)), now),
            "Tomorrow".to_string()
        );
    }
}
