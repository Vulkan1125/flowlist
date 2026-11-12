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
}

fn default_review_days() -> u32 {
    7
}

fn default_completed_days() -> u32 {
    30
}

fn default_due_time() -> Option<NaiveTime> {
    NaiveTime::from_hms_opt(17, 0, 0)
}

fn default_nearby_radius() -> f64 {
    10.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_review_days: default_review_days(),
            show_completed_days: default_completed_days(),
            week_starts_on: 0,
            default_due_time: default_due_time(),
            default_defer_time: None,
            current_location: None,
            nearby_radius_km: default_nearby_radius(),
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
