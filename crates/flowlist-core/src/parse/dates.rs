//! Natural-language date and time input.
//!
//! Supports the phrases people actually type into a quick-entry field:
//! `today`, `tomorrow`, `next friday`, `in 3 days`, `nov 20`, `11/20`,
//! `2026-11-20`, plus an optional time (`5pm`, `17:30`, `noon`).

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};

use crate::error::{Error, Result};
use super::duration::parse_duration;
use crate::time::{IntervalUnit, Stamp, add_interval, local_date, next_weekday, parse_weekday};

pub fn parse_stamp(input: &str, reference: DateTime<Utc>) -> Result<Stamp> {
    let cleaned = input
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .replace(',', " ");
    if cleaned.is_empty() {
        return Err(Error::Date("empty date".into()));
    }
    let lower = cleaned.to_lowercase();
    if lower == "now" {
        return Ok(Stamp {
            at: reference,
            has_time: true,
        });
    }
    // Compact relative offsets such as `1h30m`, `2d` and `3w` are how the
    // quick-entry `fsr:` and `due:` markers express "from now".
    if lower.chars().any(|c| c.is_ascii_alphabetic())
        && let Ok(minutes) = parse_duration(&lower)
    {
        return Ok(Stamp {
            at: reference + Duration::minutes(minutes as i64),
            has_time: true,
        });
    }

    let today = local_date(reference);
    let mut tokens: Vec<String> = lower.split_whitespace().map(str::to_string).collect();
    let mut time: Option<NaiveTime> = None;
    let mut tonight = false;
    let mut kept: Vec<String> = Vec::new();
    for token in tokens.drain(..) {
        match token.as_str() {
            "tonight" | "evening" => {
                tonight = true;
                continue;
            }
            _ => {}
        }
        if let Some(parsed) = parse_time(&token) {
            time = Some(parsed);
            continue;
        }
        kept.push(token);
    }
    kept.retain(|token| !matches!(token.as_str(), "at" | "on" | "the"));

    let date = if kept.is_empty() {
        today
    } else {
        parse_date_tokens(&kept, today)
            .ok_or_else(|| Error::Date(format!("could not understand date “{input}”")))?
    };

    if let Some(time) = time {
        Ok(Stamp::on(date, time))
    } else if tonight {
        Ok(Stamp::on(
            date,
            NaiveTime::from_hms_opt(20, 0, 0).unwrap_or(NaiveTime::MIN),
        ))
    } else {
        Ok(Stamp::date_only(date))
    }
}

pub fn parse_date(input: &str, reference: DateTime<Utc>) -> Result<NaiveDate> {
    Ok(parse_stamp(input, reference)?.local_date())
}

/// Apply a default time-of-day to a date-only stamp.
pub fn with_default_time(stamp: Stamp, default: Option<NaiveTime>) -> Stamp {
    match (stamp.has_time, default) {
        (false, Some(time)) => Stamp::on(stamp.local_date(), time),
        _ => stamp,
    }
}

pub fn parse_time(input: &str) -> Option<NaiveTime> {
    let value = input.trim().to_lowercase();
    match value.as_str() {
        "noon" => return NaiveTime::from_hms_opt(12, 0, 0),
        "midnight" => return NaiveTime::from_hms_opt(0, 0, 0),
        _ => {}
    }
    if let Some(rest) = value.strip_suffix("am") {
        let (hour, minute) = split_clock(rest)?;
        if hour > 12 {
            return None;
        }
        let hour = if hour == 12 { 0 } else { hour };
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }
    if let Some(rest) = value.strip_suffix("pm") {
        let (hour, minute) = split_clock(rest)?;
        if hour > 12 {
            return None;
        }
        let hour = if hour == 12 { 12 } else { hour + 12 };
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }
    if value.contains(':') && value.chars().all(|c| c.is_ascii_digit() || c == ':') {
        let (hour, minute) = split_clock(&value)?;
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }
    None
}

fn split_clock(input: &str) -> Option<(u32, u32)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    match input.split_once(':') {
        Some((hour, minute)) => Some((hour.parse().ok()?, minute.parse().ok()?)),
        None => Some((input.parse().ok()?, 0)),
    }
}

fn parse_date_tokens(tokens: &[String], today: NaiveDate) -> Option<NaiveDate> {
    let first = tokens[0].as_str();
    match first {
        "today" if tokens.len() == 1 => return Some(today),
        "tomorrow" if tokens.len() == 1 => return Some(today + Duration::days(1)),
        "yesterday" if tokens.len() == 1 => return Some(today - Duration::days(1)),
        "weekend" if tokens.len() == 1 => {
            let until_saturday = (chrono::Weekday::Sat.num_days_from_monday() as i64
                - today.weekday().num_days_from_monday() as i64)
                .rem_euclid(7);
            return Some(today + Duration::days(until_saturday));
        }
        "in" => {
            if tokens.len() != 3 {
                return None;
            }
            let count: i64 = tokens[1].parse().ok()?;
            if count < 0 {
                return None;
            }
            let unit = unit_from_token(&tokens[2])?;
            return Some(match unit {
                IntervalUnit::Minutes | IntervalUnit::Hours => return None,
                IntervalUnit::Days => today + Duration::days(count),
                IntervalUnit::Weeks => today + Duration::days(count * 7),
                IntervalUnit::Months => add_interval(today, count as u32, IntervalUnit::Months),
                IntervalUnit::Years => add_interval(today, count as u32, IntervalUnit::Years),
            });
        }
        "next" => {
            let rest = tokens.get(1)?;
            return match rest.as_str() {
                "week" => Some(today + Duration::days(7)),
                "month" => Some(add_interval(today, 1, IntervalUnit::Months)),
                "year" => Some(add_interval(today, 1, IntervalUnit::Years)),
                weekday => {
                    if tokens.len() != 2 {
                        return None;
                    }
                    Some(next_weekday(today, parse_weekday(weekday)?, true))
                }
            };
        }
        "this" => {
            if tokens.len() != 2 {
                return None;
            }
            return Some(next_weekday(today, parse_weekday(&tokens[1])?, false));
        }
        "last" => {
            if tokens.len() != 2 {
                return None;
            }
            return Some(
                next_weekday(today, parse_weekday(&tokens[1])?, false) - Duration::days(7),
            );
        }
        _ => {}
    }

    if let Some(date) = parse_iso(first) {
        return (tokens.len() == 1).then_some(date);
    }
    if let Some(date) = parse_slash(first, today) {
        return (tokens.len() == 1).then_some(date);
    }

    if let Some(month) = parse_month(first) {
        if tokens.len() < 2 || tokens.len() > 3 {
            return None;
        }
        let day: u32 = tokens[1].parse().ok()?;
        let year: Option<i32> = tokens.get(2).and_then(|token| token.parse().ok());
        return resolve_month_day(month, day, year, today);
    }

    if first.chars().all(|c| c.is_ascii_digit()) {
        if tokens.len() < 2 || tokens.len() > 3 {
            return None;
        }
        let day: u32 = first.parse().ok()?;
        let month = parse_month(&tokens[1])?;
        let year: Option<i32> = tokens.get(2).and_then(|token| token.parse().ok());
        return resolve_month_day(month, day, year, today);
    }

    if let Some(weekday) = parse_weekday(first) {
        return (tokens.len() == 1).then(|| next_weekday(today, weekday, false));
    }

    None
}

fn unit_from_token(token: &str) -> Option<IntervalUnit> {
    match token.trim_end_matches('s') {
        "day" => Some(IntervalUnit::Days),
        "week" | "wk" => Some(IntervalUnit::Weeks),
        "month" | "mo" => Some(IntervalUnit::Months),
        "year" | "yr" => Some(IntervalUnit::Years),
        _ => None,
    }
}

fn parse_iso(token: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(token, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(token, "%Y/%m/%d").ok())
}

fn parse_slash(token: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = token.split('/').collect();
    match parts.len() {
        2 => {
            let a: u32 = parts[0].parse().ok()?;
            let b: u32 = parts[1].parse().ok()?;
            let (month, day) = if a > 12 { (b, a) } else { (a, b) };
            resolve_month_day(month, day, None, today)
        }
        3 => {
            if parts[0].len() == 4 {
                let year: i32 = parts[0].parse().ok()?;
                let month: u32 = parts[1].parse().ok()?;
                let day: u32 = parts[2].parse().ok()?;
                NaiveDate::from_ymd_opt(year, month, day)
            } else {
                let month: u32 = parts[0].parse().ok()?;
                let day: u32 = parts[1].parse().ok()?;
                let mut year: i32 = parts[2].parse().ok()?;
                if year < 100 {
                    year += 2000;
                }
                NaiveDate::from_ymd_opt(year, month, day)
            }
        }
        _ => None,
    }
}

fn parse_month(token: &str) -> Option<u32> {
    let token = token.trim_end_matches('.');
    if !token.is_ascii() || token.len() < 3 {
        return None;
    }
    match &token[..3] {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn resolve_month_day(
    month: u32,
    day: u32,
    year: Option<i32>,
    today: NaiveDate,
) -> Option<NaiveDate> {
    if let Some(year) = year {
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    let candidate = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if candidate < today - Duration::days(1) {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)
    } else {
        Some(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn reference() -> DateTime<Utc> {
        // A Thursday, 12 November 2026, 09:00 local.
        LocalReference::make()
    }

    struct LocalReference;

    impl LocalReference {
        fn make() -> DateTime<Utc> {
            // Build a timestamp from local components so tests are timezone
            // independent.
            let date = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
            crate::time::local_to_utc(date, NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        }
    }

    #[test]
    fn relative_days() {
        let reference = reference();
        assert_eq!(
            parse_stamp("today", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 12).unwrap()
        );
        assert_eq!(
            parse_stamp("tomorrow", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
        );
        assert_eq!(
            parse_stamp("in 3 days", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 15).unwrap()
        );
    }

    #[test]
    fn weekdays_and_next() {
        let reference = reference();
        assert_eq!(
            parse_stamp("friday", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
        );
        assert_eq!(
            parse_stamp("next friday", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
        );
    }

    #[test]
    fn explicit_dates_and_times() {
        let reference = reference();
        let stamp = parse_stamp("2026-12-01 17:30", reference).unwrap();
        assert_eq!(
            stamp.local_date(),
            NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
        );
        assert_eq!(
            stamp.local_time(),
            NaiveTime::from_hms_opt(17, 30, 0).unwrap()
        );
        assert!(stamp.has_time);

        let stamp = parse_stamp("nov 20 5pm", reference).unwrap();
        assert_eq!(
            stamp.local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
        );
        assert_eq!(
            stamp.local_time(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap()
        );
    }

    #[test]
    fn month_day_rolls_into_next_year() {
        let reference = reference();
        assert_eq!(
            parse_stamp("jan 5", reference).unwrap().local_date(),
            NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()
        );
    }

    #[test]
    fn times_parse() {
        assert_eq!(parse_time("5pm"), NaiveTime::from_hms_opt(17, 0, 0));
        assert_eq!(parse_time("9:15am"), NaiveTime::from_hms_opt(9, 15, 0));
        assert_eq!(parse_time("23:05"), NaiveTime::from_hms_opt(23, 5, 0));
        assert_eq!(parse_time("noon"), NaiveTime::from_hms_opt(12, 0, 0));
        assert_eq!(parse_time("banana"), None);
    }

    #[test]
    fn reference_is_utc() {
        let _ = Utc.timestamp_opt(0, 0);
    }
}
