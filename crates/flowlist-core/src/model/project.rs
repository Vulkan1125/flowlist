//! Project entity.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::ids::{FolderId, TagId, TaskId};
use crate::time::{Stamp, local_date, now};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    #[default]
    Active,
    OnHold,
    Done,
    Dropped,
}

impl ProjectStatus {
    pub const ALL: [ProjectStatus; 4] = [
        ProjectStatus::Active,
        ProjectStatus::OnHold,
        ProjectStatus::Done,
        ProjectStatus::Dropped,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::OnHold => "on_hold",
            ProjectStatus::Done => "done",
            ProjectStatus::Dropped => "dropped",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProjectStatus::Active => "Active",
            ProjectStatus::OnHold => "On Hold",
            ProjectStatus::Done => "Done",
            ProjectStatus::Dropped => "Dropped",
        }
    }

    pub fn is_open(self) -> bool {
        matches!(self, ProjectStatus::Active | ProjectStatus::OnHold)
    }

    pub fn is_closed(self) -> bool {
        matches!(self, ProjectStatus::Done | ProjectStatus::Dropped)
    }
}

/// Project types. Parallel projects release every task at once, sequential
/// projects release one at a time, and single-action lists are buckets of
/// independent one-off tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    #[default]
    Parallel,
    Sequential,
    SingleActions,
}

impl ProjectKind {
    pub const ALL: [ProjectKind; 3] = [
        ProjectKind::Parallel,
        ProjectKind::Sequential,
        ProjectKind::SingleActions,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "Parallel",
            ProjectKind::Sequential => "Sequential",
            ProjectKind::SingleActions => "Single actions",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "parallel",
            ProjectKind::Sequential => "sequential",
            ProjectKind::SingleActions => "single_actions",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value.replace('-', "_").as_str() {
            "parallel" => Some(ProjectKind::Parallel),
            "sequential" => Some(ProjectKind::Sequential),
            "single_actions" | "single" | "single_action" => Some(ProjectKind::SingleActions),
            _ => None,
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "Every task is available at once.",
            ProjectKind::Sequential => "Tasks unlock one after another, top to bottom.",
            ProjectKind::SingleActions => "A bucket for one-off actions with no order.",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ProjectKind::Parallel => ProjectKind::Sequential,
            ProjectKind::Sequential => ProjectKind::SingleActions,
            ProjectKind::SingleActions => ProjectKind::Parallel,
        }
    }
}

/// A small, fixed icon set for projects. Keeping the list fixed means the
/// sidebar and JSON stay in sync and no asset paths leak into the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectIcon {
    #[default]
    Project,
    Folder,
    Star,
    Briefcase,
    Home,
    Heart,
    Book,
    Code,
    Rocket,
    Coffee,
    Hash,
    Plane,
    Apple,
    Basketball,
    BeachBall,
    Cake,
    BookOpen,
    Garden,
    Record,
    Square,
    Growth,
    Checklist,
    Terminal,
    Analytics,
    Document,
    Dog,
    Money,
    Mail,
    Smile,
    Flag,
    Gift,
    Support,
    Flask,
    Laptop,
    Phone,
    Lightbulb,
    Podcast,
    Moon,
    Paintbrush,
    Palette,
    Person,
    Running,
    Team,
    Pill,
    Pin,
    Pizza,
    Puzzle,
    Study,
    Sofa,
    Sparkles,
    Stethoscope,
    Sun,
    Telescope,
    Video,
    Clock,
}

impl ProjectIcon {
    pub const ALL: [ProjectIcon; 55] = [
        ProjectIcon::Project,
        ProjectIcon::Folder,
        ProjectIcon::Star,
        ProjectIcon::Briefcase,
        ProjectIcon::Home,
        ProjectIcon::Heart,
        ProjectIcon::Book,
        ProjectIcon::Code,
        ProjectIcon::Rocket,
        ProjectIcon::Coffee,
        ProjectIcon::Hash,
        ProjectIcon::Plane,
        ProjectIcon::Apple,
        ProjectIcon::Basketball,
        ProjectIcon::BeachBall,
        ProjectIcon::Cake,
        ProjectIcon::BookOpen,
        ProjectIcon::Garden,
        ProjectIcon::Record,
        ProjectIcon::Square,
        ProjectIcon::Growth,
        ProjectIcon::Checklist,
        ProjectIcon::Terminal,
        ProjectIcon::Analytics,
        ProjectIcon::Document,
        ProjectIcon::Dog,
        ProjectIcon::Money,
        ProjectIcon::Mail,
        ProjectIcon::Smile,
        ProjectIcon::Flag,
        ProjectIcon::Gift,
        ProjectIcon::Support,
        ProjectIcon::Flask,
        ProjectIcon::Laptop,
        ProjectIcon::Phone,
        ProjectIcon::Lightbulb,
        ProjectIcon::Podcast,
        ProjectIcon::Moon,
        ProjectIcon::Paintbrush,
        ProjectIcon::Palette,
        ProjectIcon::Person,
        ProjectIcon::Running,
        ProjectIcon::Team,
        ProjectIcon::Pill,
        ProjectIcon::Pin,
        ProjectIcon::Pizza,
        ProjectIcon::Puzzle,
        ProjectIcon::Study,
        ProjectIcon::Sofa,
        ProjectIcon::Sparkles,
        ProjectIcon::Stethoscope,
        ProjectIcon::Sun,
        ProjectIcon::Telescope,
        ProjectIcon::Video,
        ProjectIcon::Clock,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            ProjectIcon::Project => "project",
            ProjectIcon::Folder => "folder",
            ProjectIcon::Star => "star",
            ProjectIcon::Briefcase => "briefcase",
            ProjectIcon::Home => "home",
            ProjectIcon::Heart => "heart",
            ProjectIcon::Book => "book",
            ProjectIcon::Code => "code",
            ProjectIcon::Rocket => "rocket",
            ProjectIcon::Coffee => "coffee",
            ProjectIcon::Hash => "hash",
            ProjectIcon::Plane => "plane",
            ProjectIcon::Apple => "apple",
            ProjectIcon::Basketball => "basketball",
            ProjectIcon::BeachBall => "beach_ball",
            ProjectIcon::Cake => "cake",
            ProjectIcon::BookOpen => "book_open",
            ProjectIcon::Garden => "garden",
            ProjectIcon::Record => "record",
            ProjectIcon::Square => "square",
            ProjectIcon::Growth => "growth",
            ProjectIcon::Checklist => "checklist",
            ProjectIcon::Terminal => "terminal",
            ProjectIcon::Analytics => "analytics",
            ProjectIcon::Document => "document",
            ProjectIcon::Dog => "dog",
            ProjectIcon::Money => "money",
            ProjectIcon::Mail => "mail",
            ProjectIcon::Smile => "smile",
            ProjectIcon::Flag => "flag",
            ProjectIcon::Gift => "gift",
            ProjectIcon::Support => "support",
            ProjectIcon::Flask => "flask",
            ProjectIcon::Laptop => "laptop",
            ProjectIcon::Phone => "phone",
            ProjectIcon::Lightbulb => "lightbulb",
            ProjectIcon::Podcast => "podcast",
            ProjectIcon::Moon => "moon",
            ProjectIcon::Paintbrush => "paintbrush",
            ProjectIcon::Palette => "palette",
            ProjectIcon::Person => "person",
            ProjectIcon::Running => "running",
            ProjectIcon::Team => "team",
            ProjectIcon::Pill => "pill",
            ProjectIcon::Pin => "pin",
            ProjectIcon::Pizza => "pizza",
            ProjectIcon::Puzzle => "puzzle",
            ProjectIcon::Study => "study",
            ProjectIcon::Sofa => "sofa",
            ProjectIcon::Sparkles => "sparkles",
            ProjectIcon::Stethoscope => "stethoscope",
            ProjectIcon::Sun => "sun",
            ProjectIcon::Telescope => "telescope",
            ProjectIcon::Video => "video",
            ProjectIcon::Clock => "clock",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProjectIcon::Project => "Project",
            ProjectIcon::Folder => "Folder",
            ProjectIcon::Star => "Star",
            ProjectIcon::Briefcase => "Briefcase",
            ProjectIcon::Home => "Home",
            ProjectIcon::Heart => "Heart",
            ProjectIcon::Book => "Book",
            ProjectIcon::Code => "Code",
            ProjectIcon::Rocket => "Rocket",
            ProjectIcon::Coffee => "Coffee",
            ProjectIcon::Hash => "Hash",
            ProjectIcon::Plane => "Plane",
            ProjectIcon::Apple => "Apple",
            ProjectIcon::Basketball => "Basketball",
            ProjectIcon::BeachBall => "Beach ball",
            ProjectIcon::Cake => "Cake",
            ProjectIcon::BookOpen => "Open book",
            ProjectIcon::Garden => "Garden",
            ProjectIcon::Record => "Record",
            ProjectIcon::Square => "Square",
            ProjectIcon::Growth => "Growth",
            ProjectIcon::Checklist => "Checklist",
            ProjectIcon::Terminal => "Terminal",
            ProjectIcon::Analytics => "Analytics",
            ProjectIcon::Document => "Document",
            ProjectIcon::Dog => "Dog",
            ProjectIcon::Money => "Money",
            ProjectIcon::Mail => "Mail",
            ProjectIcon::Smile => "Smile",
            ProjectIcon::Flag => "Flag",
            ProjectIcon::Gift => "Gift",
            ProjectIcon::Support => "Support",
            ProjectIcon::Flask => "Science",
            ProjectIcon::Laptop => "Laptop",
            ProjectIcon::Phone => "Phone",
            ProjectIcon::Lightbulb => "Idea",
            ProjectIcon::Podcast => "Podcast",
            ProjectIcon::Moon => "Moon",
            ProjectIcon::Paintbrush => "Design",
            ProjectIcon::Palette => "Palette",
            ProjectIcon::Person => "Person",
            ProjectIcon::Running => "Running",
            ProjectIcon::Team => "Team",
            ProjectIcon::Pill => "Health",
            ProjectIcon::Pin => "Pin",
            ProjectIcon::Pizza => "Pizza",
            ProjectIcon::Puzzle => "Puzzle",
            ProjectIcon::Study => "Study",
            ProjectIcon::Sofa => "Home life",
            ProjectIcon::Sparkles => "Sparkles",
            ProjectIcon::Stethoscope => "Doctor",
            ProjectIcon::Sun => "Sun",
            ProjectIcon::Telescope => "Telescope",
            ProjectIcon::Video => "Video",
            ProjectIcon::Clock => "Clock",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|icon| icon.slug() == value.trim().to_lowercase())
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|icon| *icon == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: crate::ids::ProjectId,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub folder: Option<FolderId>,
    #[serde(default)]
    pub status: ProjectStatus,
    /// Opt the project's tasks into full-screen reminders.
    #[serde(default)]
    pub fullscreen_reminder: bool,
    #[serde(default)]
    pub tags: BTreeSet<TagId>,
    #[serde(default)]
    pub due: Option<Stamp>,
    #[serde(default)]
    pub defer: Option<Stamp>,
    #[serde(default = "default_review_days")]
    pub review_interval_days: u32,
    #[serde(default = "today")]
    pub next_review: NaiveDate,
    /// Legacy boolean kept for older files. New writes use `kind`.
    #[serde(default)]
    pub sequential: bool,
    #[serde(default)]
    pub kind: ProjectKind,
    /// Optional decorative icon shown in the sidebar and dashboards.
    #[serde(default)]
    pub icon: ProjectIcon,
    /// Local file paths for briefs, images or PDFs that belong to the
    /// project. Like task attachments these are referenced, never copied.
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default)]
    pub task_order: Vec<TaskId>,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub modified_at: DateTime<Utc>,
}

fn default_review_days() -> u32 {
    7
}

fn today() -> NaiveDate {
    local_date(now())
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let at = now();
        Self {
            id: crate::ids::ProjectId::new(),
            name: name.into(),
            notes: String::new(),
            folder: None,
            status: ProjectStatus::Active,
            fullscreen_reminder: false,
            tags: BTreeSet::new(),
            due: None,
            defer: None,
            review_interval_days: default_review_days(),
            next_review: local_date(at) + chrono::Duration::days(default_review_days() as i64),
            sequential: false,
            kind: ProjectKind::Parallel,
            icon: ProjectIcon::default(),
            attachments: Vec::new(),
            task_order: Vec::new(),
            created_at: at,
            modified_at: at,
        }
    }

    /// The kind to show and apply, including projects written before `kind`
    /// existed with only the `sequential` boolean set.
    pub fn effective_kind(&self) -> ProjectKind {
        if self.sequential && self.kind == ProjectKind::Parallel {
            ProjectKind::Sequential
        } else {
            self.kind
        }
    }

    pub fn is_sequential(&self) -> bool {
        self.effective_kind() == ProjectKind::Sequential
    }

    pub fn is_due_for_review(&self, date: NaiveDate) -> bool {
        self.status.is_open() && self.next_review <= date
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectIcon;

    #[test]
    fn new_projects_use_the_project_icon() {
        assert_eq!(ProjectIcon::default(), ProjectIcon::Project);
    }
}
