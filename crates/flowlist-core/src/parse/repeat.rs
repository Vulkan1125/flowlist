//! Repeat rule parsing: `every 2 weeks`, `daily`, `every month after
//! completion`, optionally ending `until 2027-01-01` or `for 10 times`.

use chrono::DateTime;
use chrono::Utc;

use crate::error::{Error, Result};
use crate::model::{RepeatEnd, RepeatMode, RepeatPattern, RepeatRule, RepeatSchedule};
use crate::parse::dates::parse_stamp;
use crate::time::{IntervalUnit, parse_weekday};

pub fn parse_repeat(input: &str, reference: DateTime<Utc>) -> Result<RepeatRule> {
    let mut text = input.trim().to_lowercase();
    if text.is_empty() {
        return Err(Error::invalid("empty repeat rule"));
    }
    if let Some(rest) = text.strip_prefix("repeat:") {
        text = rest.trim().to_string();
    }

    let pattern = detect_pattern(&text);
    let (every, unit) = if pattern.is_some() {
        parse_interval_anywhere(&text).unwrap_or((1, IntervalUnit::Months))
    } else {
        match single_word_rule(&text) {
            Some(pair) => pair,
            None => parse_interval(&text)?,
        }
    };
    let mut rule = RepeatRule::new(RepeatMode::DueAgain, every, unit);
    if let Some(pattern) = pattern {
        rule.pattern = Some(pattern);
        rule.unit = IntervalUnit::Months;
        rule.weekdays = None;
    }

    if text.contains("after completion") || text.contains("from completion") {
        rule.schedule = RepeatSchedule::AfterCompletion;
    }
    if text.contains("defer another") || text.contains("defer again") {
        rule.mode = RepeatMode::DeferAnother;
    }

    if let Some(index) = text.find(" until ") {
        let rest = &text[index + " until ".len()..];
        let date_text = rest.split(" for ").next().unwrap_or(rest).trim();
        let stamp = parse_stamp(date_text, reference)?;
        rule.end = RepeatEnd::OnDate(stamp.local_date());
    } else if let Some(index) = text.find(" for ") {
        let rest = &text[index + " for ".len()..];
        let count_text = rest
            .trim()
            .trim_end_matches(" times")
            .trim_end_matches(" time");
        let count: u32 = count_text
            .split_whitespace()
            .next()
            .and_then(|token| token.parse().ok())
            .ok_or_else(|| Error::invalid("repeat count must be a number"))?;
        rule.end = RepeatEnd::AfterCount(count.max(1));
    }

    Ok(rule)
}

fn clean_token(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_ascii_alphanumeric())
}

fn ordinal_value(token: &str) -> Option<u8> {
    Some(match clean_token(token) {
        "first" | "1st" => 1,
        "second" | "2nd" => 2,
        "third" | "3rd" => 3,
        "fourth" | "4th" => 4,
        "fifth" | "5th" => 5,
        _ => return None,
    })
}

fn detect_pattern(text: &str) -> Option<RepeatPattern> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        if clean_token(token) == "last"
            && tokens
                .get(index + 1)
                .is_some_and(|next| clean_token(next).starts_with("weekend"))
        {
            return Some(RepeatPattern::LastWeekendDay);
        }
        if let Some(nth) = ordinal_value(token) {
            if let Some(weekday) = tokens
                .get(index + 1)
                .and_then(|next| parse_weekday(clean_token(next)))
            {
                return Some(RepeatPattern::NthWeekday {
                    nth,
                    weekday: weekday.num_days_from_sunday() as u8,
                });
            }
        }
    }
    None
}

/// Find a cadence anywhere in the phrase so pattern wording may come before or
/// after it: "monthly on the third Thursday" and "the last weekend day of every
/// two months" both work.
fn parse_interval_anywhere(text: &str) -> Option<(u32, IntervalUnit)> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let Some(pair) = single_word_rule(clean_token(token)) {
            return Some(pair);
        }
        if clean_token(token) == "every" {
            let rest = tokens[index..].join(" ");
            if let Ok(pair) = parse_interval(&rest) {
                return Some(pair);
            }
        }
    }
    None
}

fn single_word_rule(text: &str) -> Option<(u32, IntervalUnit)> {
    let (every, unit) = match text.split_whitespace().next()? {
        "daily" => (1, IntervalUnit::Days),
        "weekly" => (1, IntervalUnit::Weeks),
        "hourly" => (1, IntervalUnit::Hours),
        "monthly" => (1, IntervalUnit::Months),
        "yearly" | "annually" => (1, IntervalUnit::Years),
        _ => return None,
    };
    Some((every, unit))
}

fn parse_interval(text: &str) -> Result<(u32, IntervalUnit)> {
    let mut tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.first().is_some_and(|token| *token == "every") {
        tokens.remove(0);
    }
    if tokens.is_empty() {
        return Err(Error::invalid("repeat rule is missing an interval"));
    }
    let every = if let Ok(value) = tokens[0].parse::<u32>() {
        tokens.remove(0);
        value.max(1)
    } else {
        1
    };
    let unit_token = tokens
        .first()
        .copied()
        .ok_or_else(|| Error::invalid("repeat rule is missing a unit"))?;
    let unit = match unit_token.trim_end_matches('s') {
        "minute" => IntervalUnit::Minutes,
        "hour" => IntervalUnit::Hours,
        "day" => IntervalUnit::Days,
        "week" => IntervalUnit::Weeks,
        "month" => IntervalUnit::Months,
        "year" => IntervalUnit::Years,
        other => return Err(Error::invalid(format!("unknown repeat unit “{other}”"))),
    };
    Ok((every, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_intervals() {
        let now = Utc::now();
        let rule = parse_repeat("every 2 weeks", now).unwrap();
        assert_eq!(rule.every, 2);
        assert_eq!(rule.unit, IntervalUnit::Weeks);
        assert_eq!(rule.schedule, RepeatSchedule::Fixed);

        let rule = parse_repeat("daily", now).unwrap();
        assert_eq!(rule.unit, IntervalUnit::Days);
        assert_eq!(rule.every, 1);
    }

    #[test]
    fn parses_sub_day_intervals() {
        let now = Utc::now();
        let rule = parse_repeat("every 30 minutes", now).unwrap();
        assert_eq!(rule.every, 30);
        assert_eq!(rule.unit, IntervalUnit::Minutes);

        let rule = parse_repeat("hourly", now).unwrap();
        assert_eq!(rule.every, 1);
        assert_eq!(rule.unit, IntervalUnit::Hours);

        let rule = parse_repeat("every 2 hours", now).unwrap();
        assert_eq!(rule.every, 2);
        assert_eq!(rule.unit, IntervalUnit::Hours);
    }

    #[test]
    fn parses_monthly_calendar_patterns() {
        let now = Utc::now();
        let rule = parse_repeat("every month on the third thursday", now).unwrap();
        assert_eq!(rule.unit, IntervalUnit::Months);
        assert_eq!(rule.every, 1);
        assert_eq!(
            rule.pattern,
            Some(RepeatPattern::NthWeekday { nth: 3, weekday: 4 })
        );

        let rule = parse_repeat("every 2 months on the last weekend day", now).unwrap();
        assert_eq!(rule.every, 2);
        assert_eq!(rule.pattern, Some(RepeatPattern::LastWeekendDay));

        let rule = parse_repeat("second tuesday of every month", now).unwrap();
        assert_eq!(rule.every, 1);
        assert_eq!(
            rule.pattern,
            Some(RepeatPattern::NthWeekday { nth: 2, weekday: 2 })
        );

        let rule = parse_repeat("last weekend day", now).unwrap();
        assert_eq!(rule.unit, IntervalUnit::Months);
        assert_eq!(rule.pattern, Some(RepeatPattern::LastWeekendDay));
    }

    #[test]
    fn parses_modifiers_and_endings() {
        let now = Utc::now();
        let rule = parse_repeat("every month after completion defer another", now).unwrap();
        assert_eq!(rule.schedule, RepeatSchedule::AfterCompletion);
        assert_eq!(rule.mode, RepeatMode::DeferAnother);

        let rule = parse_repeat("every 3 days for 10 times", now).unwrap();
        assert_eq!(rule.end, RepeatEnd::AfterCount(10));

        let rule = parse_repeat("every week until 2027-01-01", now).unwrap();
        assert_eq!(
            rule.end,
            RepeatEnd::OnDate(chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap())
        );
    }

    #[test]
    fn rejects_nonsense() {
        let now = Utc::now();
        assert!(parse_repeat("every banana", now).is_err());
        assert!(parse_repeat("", now).is_err());
    }
}
