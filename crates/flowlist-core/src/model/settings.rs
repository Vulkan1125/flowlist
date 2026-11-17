//! Library-wide preferences. Kept inside the data file so CLI, MCP and GUI
//! share defaults.

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
    /// How many months the Forecast calendar shows: 0 follows the pane width,
    /// 1 and 2 pin it.
    #[serde(default)]
    pub forecast_months: u8,
    /// When true the Forecast calendar fills the content pane; otherwise it
    /// keeps calendar-sized cells and centres itself on wide windows.
    #[serde(default)]
    pub forecast_full_width: bool,
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
/// the rail shows. Sections default to visible; counts default quiet except
/// for the project counts inside the library, where they carry the most
/// useful information.
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
            forecast_months: 0,
            forecast_full_width: false,
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
