//! Library-wide preferences. Kept inside the data file so CLI, MCP and GUI
//! share defaults.

use std::collections::BTreeSet;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use super::tag::Geo;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Default review interval applied to new projects, in days.
    #[serde(default = "default_review_days")]
    pub default_review_days: u32,
    /// Completed tasks older than this are hidden from the Completed view.
    #[serde(default = "default_completed_days")]
    pub show_completed_days: u32,
    /// How far back the Changed perspective looks, in days.
    #[serde(default = "default_changed_days")]
    pub changed_days: u32,
    /// 0 = Monday ... 6 = Sunday, used by calendar views.
    #[serde(default)]
    pub week_starts_on: u8,
    /// Time-of-day applied when a due date is entered without a time.
    #[serde(default = "default_due_time")]
    pub default_due_time: Option<NaiveTime>,
    /// Time-of-day applied when a defer date is entered without a time.
    #[serde(default)]
    pub default_defer_time: Option<NaiveTime>,
    /// Last manually supplied location for the Nearby perspective.
    #[serde(default)]
    pub current_location: Option<Geo>,
    #[serde(default = "default_nearby_radius")]
    pub nearby_radius_km: f64,
    /// Days a trashed task is kept before automatic permanent deletion.
    /// `0` keeps trash forever.
    #[serde(default = "default_trash_days")]
    pub trash_retention_days: u32,
    /// Which sidebar sections appear, and where counts are shown.
    #[serde(default)]
    pub sidebar: SidebarOptions,
}

/// Sidebar presentation, kept in the library so every client agrees on what
/// the rail shows. The per-item `hidden` set drives visibility now. The old
/// `show_*` section switches remain only so libraries written by older
/// versions still deserialize; the count switches seed the first per-item
/// count matrix before anyone edits it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidebarOptions {
    #[serde(default = "default_true")]
    pub show_perspectives: bool,
    #[serde(default = "default_true")]
    pub show_priorities: bool,
    #[serde(default = "default_true")]
    pub show_library: bool,
    #[serde(default = "default_true")]
    pub show_saved: bool,
    #[serde(default = "default_true")]
    pub show_tags: bool,
    #[serde(default)]
    pub counts_perspectives: bool,
    #[serde(default)]
    pub counts_priorities: bool,
    #[serde(default = "default_true")]
    pub counts_library: bool,
    #[serde(default)]
    pub counts_tags: bool,
    /// Individual rail rows hidden from the sidebar, keyed as
    /// `builtin:<slug>`, `priority:<slug>`, `folder:<id>`, `project:<id>`,
    /// `custom:<id>` or `tag:<id>`. Inbox, Today and Trash are never hidden.
    #[serde(default)]
    pub hidden: BTreeSet<String>,
    /// Individual rows whose count badge is hidden. The same keys as `hidden`.
    #[serde(default)]
    pub counts_hidden: BTreeSet<String>,
    /// False keeps the old section-level count flags in charge until someone
    /// edits a count toggle; the first edit materialises the current defaults
    /// into `counts_hidden`.
    #[serde(default)]
    pub counts_configured: bool,
}

impl SidebarOptions {
    /// Inbox, Today and Trash anchor the rail and stay reachable.
    pub fn can_hide(key: &str) -> bool {
        !matches!(key, "builtin:inbox" | "builtin:today" | "builtin:trash")
    }

    pub fn is_hidden(&self, key: &str) -> bool {
        self.hidden.contains(key)
    }

    /// Whether a row's count badge is suppressed. Before the first per-item
    /// edit the old section flags decide, which keeps an upgrade quiet.
    pub fn is_count_hidden(&self, key: &str) -> bool {
        if self.counts_configured {
            self.counts_hidden.contains(key)
        } else if key.starts_with("builtin:") {
            !(self.counts_perspectives || key == "builtin:today" || key == "builtin:review")
        } else if key.starts_with("priority:") {
            !self.counts_priorities
        } else if key.starts_with("folder:") || key.starts_with("project:") {
            !self.counts_library
        } else if key.starts_with("tag:") {
            !self.counts_tags
        } else {
            false
        }
    }

    /// Toggle one row. Protected rows are ignored, so a stale key from a
    /// future version cannot remove a required view.
    pub fn toggle_hidden(&mut self, key: &str) {
        if !Self::can_hide(key) {
            return;
        }
        if !self.hidden.remove(key) {
            self.hidden.insert(key.to_string());
        }
    }
}

impl Default for SidebarOptions {
    fn default() -> Self {
        Self {
            show_perspectives: true,
            show_priorities: true,
            show_library: true,
            show_saved: true,
            show_tags: true,
            counts_perspectives: false,
            counts_priorities: false,
            counts_library: true,
            counts_tags: false,
            hidden: BTreeSet::new(),
            counts_hidden: BTreeSet::new(),
            counts_configured: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_review_days() -> u32 {
    7
}

fn default_completed_days() -> u32 {
    30
}

fn default_changed_days() -> u32 {
    7
}

fn default_due_time() -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(17, 0, 0)
}

fn default_nearby_radius() -> f64 {
    10.0
}

fn default_trash_days() -> u32 {
    30
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_review_days: default_review_days(),
            show_completed_days: default_completed_days(),
            changed_days: default_changed_days(),
            week_starts_on: 0,
            default_due_time: default_due_time(),
            default_defer_time: None,
            current_location: None,
            nearby_radius_km: default_nearby_radius(),
            trash_retention_days: default_trash_days(),
            sidebar: SidebarOptions::default(),
        }
    }
}

impl Settings {
    pub fn week_start(&self) -> chrono::Weekday {
        use chrono::Weekday;
        match self.week_starts_on % 7 {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            _ => Weekday::Sun,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SidebarOptions;

    #[test]
    fn inbox_today_and_trash_cannot_be_hidden() {
        let mut sidebar = SidebarOptions::default();
        sidebar.toggle_hidden("builtin:inbox");
        sidebar.toggle_hidden("builtin:today");
        sidebar.toggle_hidden("builtin:trash");
        assert!(sidebar.hidden.is_empty());

        sidebar.toggle_hidden("builtin:flagged");
        assert!(sidebar.is_hidden("builtin:flagged"));
        sidebar.toggle_hidden("builtin:flagged");
        assert!(!sidebar.is_hidden("builtin:flagged"));
    }

    #[test]
    fn legacy_count_flags_seed_the_first_count_matrix() {
        let sidebar = SidebarOptions::default();
        assert!(sidebar.is_count_hidden("builtin:inbox"));
        assert!(!sidebar.is_count_hidden("builtin:today"));
        assert!(!sidebar.is_count_hidden("builtin:review"));
        assert!(!sidebar.is_count_hidden("custom:any"));
        assert!(sidebar.is_count_hidden("priority:high"));
        assert!(!sidebar.is_count_hidden("project:any"));
        assert!(sidebar.is_count_hidden("tag:any"));
    }

    #[test]
    fn explicit_count_choices_ignore_the_old_flags() {
        let mut sidebar = SidebarOptions {
            counts_configured: true,
            ..SidebarOptions::default()
        };
        sidebar.counts_hidden.insert("tag:home".to_string());
        assert!(sidebar.is_count_hidden("tag:home"));
        assert!(!sidebar.is_count_hidden("tag:work"));
        assert!(!sidebar.is_count_hidden("builtin:inbox"));
    }

    #[test]
    fn hidden_rows_round_trip_through_json() {
        let mut sidebar = SidebarOptions::default();
        sidebar.toggle_hidden("project:abc");
        sidebar.toggle_hidden("tag:home");
        let json = serde_json::to_string(&sidebar).expect("serialize");
        let back: SidebarOptions = serde_json::from_str(&json).expect("deserialize");
        assert!(back.is_hidden("project:abc"));
        assert!(back.is_hidden("tag:home"));
        assert!(!back.is_hidden("project:other"));
    }
}
