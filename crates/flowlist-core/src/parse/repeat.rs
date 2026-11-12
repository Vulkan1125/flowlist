//! Repeat rule parsing: `every 2 weeks`, `daily`, `every month after
//! completion`, optionally ending `until 2027-01-01` or `for 10 times`.

use chrono::DateTime;
use chrono::Utc;

use crate::error::{Error, Result};
use crate::model::{RepeatEnd, RepeatMode, RepeatRule, RepeatSchedule};
use crate::parse::dates::parse_stamp;
use crate::time::IntervalUnit;

pub fn parse_repeat(input: &str, reference: DateTime<Utc>) -> Result<RepeatRule> {
    let mut text = input.trim().to_lowercase();
    if text.is_empty() {
        return Err(Error::invalid("empty repeat rule"));
    }
    if let Some(rest) = text.strip_prefix("repeat:") {
        text = rest.trim().to_string();
    }

    let (every, unit) = match single_word_rule(&text) {
        Some(pair) => pair,
        None => parse_interval(&text)?,
    };
    let mut rule = RepeatRule::new(RepeatMode::DueAgain, every, unit);

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

fn single_word_rule(text: &str) -> Option<(u32, IntervalUnit)> {
    let (every, unit) = match text.split_whitespace().next()? {
        "daily" => (1, IntervalUnit::Days),
        "weekly" => (1, IntervalUnit::Weeks),
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
