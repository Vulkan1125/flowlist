//! Tags (contexts): hierarchical, with availability and optional coordinates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::TagId;
use crate::time::now;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TagStatus {
    #[default]
    Available,
    /// Blocked by another context; tasks are not actionable.
    Blocked,
    /// Deliberately paused context.
    OnHold,
}

impl TagStatus {
    pub fn label(self) -> &'static str {
        match self {
            TagStatus::Available => "Available",
            TagStatus::Blocked => "Blocked",
            TagStatus::OnHold => "On Hold",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Geo {
    pub lat: f64,
    pub lon: f64,
}

impl Geo {
    pub fn distance_km(self, other: Geo) -> f64 {
        // Haversine distance; adequate for the Nearby perspective.
        let earth_km = 6371.0;
        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();
        let dlat = (other.lat - self.lat).to_radians();
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        2.0 * earth_km * a.sqrt().asin()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    #[serde(default)]
    pub parent: Option<TagId>,
    #[serde(default)]
    pub status: TagStatus,
    #[serde(default)]
    pub location: Option<Geo>,
    #[serde(default)]
    pub child_order: Vec<TagId>,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub modified_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        let at = now();
        Self {
            id: TagId::new(),
            name: name.into(),
            parent: None,
            status: TagStatus::Available,
            location: None,
            child_order: Vec::new(),
            created_at: at,
            modified_at: at,
        }
    }
}
