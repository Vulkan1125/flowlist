//! Duration wording used by estimates: `30m`, `1h`, `1h30m`, `2 hours`.

use crate::error::{Error, Result};

pub fn parse_duration(input: &str) -> Result<u32> {
    let value = input.trim().to_lowercase();
    if value.is_empty() {
        return Err(Error::invalid("empty duration"));
    }
    if value.chars().all(|c| c.is_ascii_digit()) {
        let minutes: u32 = value
            .parse()
            .map_err(|_| Error::invalid(format!("invalid duration “{input}”")))?;
        if minutes == 0 {
            return Err(Error::invalid("duration must be greater than zero"));
        }
        return Ok(minutes);
    }

    let mut total = 0f64;
    let mut number = String::new();
    let mut unit = String::new();
    let mut matched = false;

    let flush = |total: &mut f64,
                 number: &mut String,
                 unit: &mut String,
                 matched: &mut bool|
     -> std::result::Result<(), Error> {
        if number.is_empty() && unit.is_empty() {
            return Ok(());
        }
        if number.is_empty() {
            return Err(Error::invalid("duration is missing a number"));
        }
        let amount: f64 = number
            .parse()
            .map_err(|_| Error::invalid("invalid duration number"))?;
        let minutes = match unit.as_str() {
            "" | "m" | "min" | "mins" | "minute" | "minutes" => 1.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 60.0,
            "d" | "day" | "days" => 1440.0,
            "w" | "wk" | "week" | "weeks" => 10080.0,
            other => {
                return Err(Error::invalid(format!(
                    "unknown duration unit “{other}”"
                )))
            }
        };
        *total += amount * minutes;
        number.clear();
        unit.clear();
        *matched = true;
        Ok(())
    };

    for ch in value.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            if !unit.is_empty() {
                flush(&mut total, &mut number, &mut unit, &mut matched)?;
            }
            number.push(ch);
        } else if ch.is_whitespace() {
            if !number.is_empty() && !unit.is_empty() {
                flush(&mut total, &mut number, &mut unit, &mut matched)?;
            }
        } else if ch.is_ascii_alphabetic() {
            unit.push(ch);
        } else {
            return Err(Error::invalid(format!("invalid duration “{input}”")));
        }
    }
    flush(&mut total, &mut number, &mut unit, &mut matched)?;

    if !matched || total <= 0.0 {
        return Err(Error::invalid(format!("invalid duration “{input}”")));
    }
    Ok(total.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        assert_eq!(parse_duration("30").unwrap(), 30);
        assert_eq!(parse_duration("30m").unwrap(), 30);
        assert_eq!(parse_duration("1h").unwrap(), 60);
        assert_eq!(parse_duration("1h30m").unwrap(), 90);
        assert_eq!(parse_duration("1 h 30 m").unwrap(), 90);
        assert_eq!(parse_duration("2 hours").unwrap(), 120);
        assert_eq!(parse_duration("1.5h").unwrap(), 90);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("soon").is_err());
        assert!(parse_duration("3 bananas").is_err());
    }
}
