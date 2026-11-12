//! Quick-entry parser shared by the GUI capture bar and `flowlist add`.
//!
//! ```text
//! Buy milk #Errands/Store ::Home/Weekly ! due:tomorrow est:15m repeat:every week
//! ```
//!
//! * `#tag` or `#Parent/Child` — attach a tag, creating it when needed
//! * `::Project` or `::Folder/Project` — file the task, creating it when needed
//! * `!` — flag the task
//! * `due:`, `defer:` — natural-language dates (`due:(next friday 5pm)`)
//! * `est:` / `estimate:` — duration (`est:1h30m`)
//! * `repeat:` — repeat rule (`repeat:(every 2 weeks after completion)`)
//! * `//` — everything after is a note

use chrono::{DateTime, Utc};

use crate::command::NewTask;
use crate::error::{Error, Result};
use crate::model::RepeatRule;
use crate::time::Stamp;
use crate::parse::dates::parse_stamp;
use crate::parse::duration::parse_duration;
use crate::parse::repeat::parse_repeat;

#[derive(Clone, Debug, Default)]
pub struct QuickEntry {
    pub title: String,
    pub project_path: Option<String>,
    pub tag_paths: Vec<String>,
    pub due: Option<Stamp>,
    pub defer: Option<Stamp>,
    pub estimate_min: Option<u32>,
    pub flagged: bool,
    pub repeat: Option<RepeatRule>,
    pub notes: String,
}

impl QuickEntry {
    pub fn into_new_task(&self) -> NewTask {
        let mut task = NewTask::new(self.title.clone());
        task.project_path = self.project_path.clone();
        task.tag_paths = self.tag_paths.clone();
        task.due = self.due;
        task.defer = self.defer;
        task.estimate_min = self.estimate_min;
        task.flagged = self.flagged;
        task.repeat = self.repeat.clone();
        task.notes = self.notes.clone();
        task
    }
}

pub fn parse(input: &str, reference: DateTime<Utc>) -> Result<QuickEntry> {
    let (main, notes) = match input.split_once("//") {
        Some((main, notes)) => (main, notes.trim().to_string()),
        None => (input, String::new()),
    };

    let mut entry = QuickEntry {
        notes,
        ..QuickEntry::default()
    };
    let mut title: Vec<String> = Vec::new();

    for token in tokenize(main) {
        if token == "!" {
            entry.flagged = true;
            continue;
        }
        if let Some(tag) = token.strip_prefix('#') {
            if !tag.trim().is_empty() {
                entry.tag_paths.push(tag.trim().to_string());
                continue;
            }
        }
        if let Some(project) = token.strip_prefix("::") {
            if !project.trim().is_empty() {
                entry.project_path = Some(project.trim().to_string());
                continue;
            }
        }
        if let Some((key, value)) = split_attribute(&token) {
            match key.as_str() {
                "due" => entry.due = Some(parse_stamp(&value, reference)?),
                "defer" => entry.defer = Some(parse_stamp(&value, reference)?),
                "est" | "estimate" => entry.estimate_min = Some(parse_duration(&value)?),
                "repeat" => entry.repeat = Some(parse_repeat(&value, reference)?),
                "note" => {
                    if !entry.notes.is_empty() {
                        entry.notes.push('\n');
                    }
                    entry.notes.push_str(&value);
                }
                "project" => entry.project_path = Some(value),
                _ => title.push(token),
            }
            continue;
        }
        title.push(token);
    }

    entry.title = title.join(" ").trim().to_string();
    if entry.title.is_empty() {
        return Err(Error::invalid("task needs a title"));
    }
    Ok(entry)
}

fn split_attribute(token: &str) -> Option<(String, String)> {
    let (key, value) = token.split_once(':')?;
    let key = key.trim().to_lowercase();
    let value = value.trim().trim_matches(|c| c == '(' || c == ')').trim();
    if !matches!(
        key.as_str(),
        "due" | "defer" | "est" | "estimate" | "repeat" | "note" | "project"
    ) {
        return None;
    }
    if value.is_empty() {
        return None;
    }
    Some((key, value.to_string()))
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    for ch in input.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::local_to_utc;
    use chrono::{NaiveDate, NaiveTime};

    fn reference() -> DateTime<Utc> {
        local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 12).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        )
    }

    #[test]
    fn parses_a_full_capture() {
        let entry = parse(
            "Buy milk #Errands/Store ::Home/Weekly ! due:tomorrow est:15m // from the corner shop",
            reference(),
        )
        .unwrap();
        assert_eq!(entry.title, "Buy milk");
        assert_eq!(entry.project_path.as_deref(), Some("Home/Weekly"));
        assert_eq!(entry.tag_paths, vec!["Errands/Store".to_string()]);
        assert!(entry.flagged);
        assert_eq!(entry.estimate_min, Some(15));
        assert_eq!(entry.notes, "from the corner shop");
        assert_eq!(
            entry.due.unwrap().local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
        );
    }

    #[test]
    fn parses_grouped_dates() {
        let entry = parse("Call mom due:(next friday 5pm)", reference()).unwrap();
        let due = entry.due.unwrap();
        assert_eq!(
            due.local_date(),
            NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
        );
        assert_eq!(due.local_time(), NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }

    #[test]
    fn flags_only_standalone_bang() {
        let entry = parse("Ship it!", reference()).unwrap();
        assert!(!entry.flagged);
        assert_eq!(entry.title, "Ship it!");
    }

    #[test]
    fn rejects_empty_titles() {
        assert!(parse("due:tomorrow", reference()).is_err());
    }
}
