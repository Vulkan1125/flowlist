//! Which list the window is showing, and the small amount of state that
//! survives a quit (view, selection, theme, inspector, window size).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use flowlist_core::db::Db;
use flowlist_core::ids::{FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use flowlist_core::model::{BuiltinPerspective, Priority};

use super::theme::{Appearance, ThemeKind, parse_hex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Builtin(BuiltinPerspective),
    Project(ProjectId),
    Folder(FolderId),
    Tag(TagId),
    Custom(PerspectiveId),
    Priority(Priority),
}

impl View {
    pub fn key(&self) -> String {
        match self {
            View::Builtin(perspective) => perspective.slug().to_string(),
            View::Project(id) => format!("project:{id}"),
            View::Folder(id) => format!("folder:{id}"),
            View::Tag(id) => format!("tag:{id}"),
            View::Custom(id) => format!("custom:{id}"),
            View::Priority(priority) => format!("priority:{}", priority.slug()),
        }
    }

    pub fn from_key(db: &Db, key: &str) -> Option<Self> {
        if let Some(perspective) = BuiltinPerspective::from_slug(key) {
            return Some(View::Builtin(perspective));
        }
        let (kind, id) = key.split_once(':')?;
        match kind {
            "project" => db.resolve_project(id).ok().map(View::Project),
            "folder" => db.resolve_folder(id).ok().map(View::Folder),
            "tag" => db.resolve_tag(id).ok().map(View::Tag),
            "custom" => db.resolve_perspective(id).ok().map(View::Custom),
            "priority" => Priority::from_slug(id).map(View::Priority),
            _ => None,
        }
    }

    pub fn title(&self, db: &Db) -> String {
        match self {
            View::Builtin(perspective) => perspective.label().to_string(),
            View::Project(id) => db
                .projects
                .get(id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "Project".to_string()),
            View::Folder(id) => db
                .folders
                .get(id)
                .map(|folder| db.folder_path(&folder.id))
                .unwrap_or_else(|| "Folder".to_string()),
            View::Tag(id) => db
                .tags
                .get(id)
                .map(|tag| db.tag_path(&tag.id))
                .unwrap_or_else(|| "Tag".to_string()),
            View::Custom(id) => db
                .perspectives
                .get(id)
                .map(|perspective| perspective.name.clone())
                .unwrap_or_else(|| "Perspective".to_string()),
            View::Priority(priority) => format!("{} priority", priority.label()),
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            View::Builtin(BuiltinPerspective::Inbox) => "Capture now, organise later",
            View::Builtin(BuiltinPerspective::Today) => "Due today, plus what is overdue",
            View::Builtin(BuiltinPerspective::Projects) => "Every project at a glance",
            View::Builtin(BuiltinPerspective::Forecast) => "What is coming up",
            View::Builtin(BuiltinPerspective::Flagged) => "Your personal priority list",
            View::Builtin(BuiltinPerspective::Review) => "Keep projects honest",
            View::Builtin(BuiltinPerspective::Tags) => "Group work by context",
            View::Builtin(BuiltinPerspective::Completed) => "Recently finished work",
            View::Builtin(BuiltinPerspective::Archive) => "Finished and dropped projects",
            View::Builtin(BuiltinPerspective::Changed) => "Recently edited work",
            View::Builtin(BuiltinPerspective::Attachments) => "Files attached to tasks",
            View::Builtin(BuiltinPerspective::Nearby) => "Contexts with a location",
            View::Builtin(BuiltinPerspective::Trash) => "Deleted tasks, restorable for a while",
            View::Project(_) => "Project outline",
            View::Folder(_) => "Projects in this folder",
            View::Tag(_) => "Everything with this context",
            View::Custom(_) => "Saved perspective",
            View::Priority(_) => "Tasks with this priority",
        }
    }

    /// Views whose body is a list of tasks. Search filters that list; the
    /// Archive, Review and Nearby bodies are built on projects and context, so
    /// they do not take the task scope switch.
    pub fn searches_tasks(&self) -> bool {
        !matches!(
            self,
            View::Builtin(
                BuiltinPerspective::Archive
                    | BuiltinPerspective::Review
                    | BuiltinPerspective::Nearby
            )
        )
    }

    pub fn shows_capture(&self) -> bool {
        match self {
            View::Builtin(BuiltinPerspective::Inbox) => true,
            View::Project(_) => true,
            View::Folder(_) | View::Tag(_) | View::Custom(_) | View::Priority(_) => true,
            _ => false,
        }
    }
}

/// Sorting, grouping and filters remembered per perspective. The choice lives
/// in the session so built-in and saved views behave alike.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewOptions {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub show_completed: Option<bool>,
    #[serde(default)]
    pub only_available: Option<bool>,
    /// The header priority filter, kept per view so a Priority row in the
    /// sidebar does not leave its filter on the next perspective.
    #[serde(default)]
    pub priority: Option<String>,
    /// Today's header toggle: leave overdue work out of the list.
    #[serde(default)]
    pub hide_overdue: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub selected: Vec<String>,
    #[serde(default)]
    pub inspector: Option<bool>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub appearance: Option<String>,
    /// Plain's custom accent, saved separately per appearance so the chosen
    /// tone stays readable in both light and dark. Absent means the family ink.
    #[serde(default)]
    pub plain_accent_light: Option<String>,
    #[serde(default)]
    pub plain_accent_dark: Option<String>,
    #[serde(default)]
    pub window: Option<(f32, f32)>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub show_completed: Option<bool>,
    #[serde(default)]
    pub vim_mode: Option<bool>,
    /// Global priority filter, `high`, `medium`, `low`, or absent for all.
    #[serde(default)]
    pub priority_filter: Option<String>,
    /// Show only the next available task instead of everything that is blocked.
    #[serde(default)]
    pub only_available: Option<bool>,
    /// Keep the theme family but take light/dark from the OS.
    #[serde(default)]
    pub follow_system: Option<bool>,
    /// Show the macOS close, minimise and zoom buttons over the header.
    /// Absent means visible, including for sessions written before the
    /// setting existed.
    #[serde(default)]
    pub traffic_lights: Option<bool>,
    /// Whether the Forecast month grid is expanded.
    #[serde(default)]
    pub forecast_calendar_open: Option<bool>,
    /// The height the user dragged the Forecast calendar to, in pixels. Absent
    /// means the calendar sizes itself to the month's week count.
    #[serde(default)]
    pub forecast_calendar_height: Option<f32>,
    /// Per-perspective sort, grouping and filters.
    #[serde(default)]
    pub view_options: BTreeMap<String, ViewOptions>,
}

impl Session {
    pub fn view(&self, db: &Db) -> View {
        self.view
            .as_deref()
            .and_then(|key| View::from_key(db, key))
            .unwrap_or(View::Builtin(BuiltinPerspective::Inbox))
    }

    pub fn theme_kind(&self) -> Option<ThemeKind> {
        self.theme.as_deref().and_then(ThemeKind::from_slug)
    }

    /// The saved theme family and light/dark mode.
    ///
    /// Sessions from before the family split stored only `"light"` or `"dark"`,
    /// so those two values still map to Paper light and Graphite dark. All
    /// other families fall back to their natural mode when no appearance was
    /// written. A library with no saved theme opens in Plain light, the
    /// first-run default.
    pub fn theme_choice(&self) -> (ThemeKind, Appearance) {
        let kind = self.theme_kind().unwrap_or(ThemeKind::Plain);
        let legacy = match self.theme.as_deref() {
            Some("light") => Some(Appearance::Light),
            Some("dark") => Some(Appearance::Dark),
            _ => None,
        };
        let appearance = legacy
            .or_else(|| self.appearance.as_deref().and_then(Appearance::from_slug))
            .unwrap_or_else(|| kind.default_appearance());
        (kind, appearance)
    }

    pub fn plain_accent(&self, appearance: Appearance) -> Option<u32> {
        let value = match appearance {
            Appearance::Light => &self.plain_accent_light,
            Appearance::Dark => &self.plain_accent_dark,
        };
        value.as_deref().and_then(parse_hex)
    }

    pub fn priority_filter(&self) -> Option<Priority> {
        self.priority_filter
            .as_deref()
            .and_then(Priority::from_slug)
    }

    pub fn selected_tasks(&self, db: &Db) -> Vec<TaskId> {
        self.selected
            .iter()
            .filter_map(|id| db.resolve_task(id).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(theme: Option<&str>, appearance: Option<&str>) -> Session {
        Session {
            theme: theme.map(str::to_string),
            appearance: appearance.map(str::to_string),
            ..Session::default()
        }
    }

    #[test]
    fn legacy_light_and_dark_migrate_to_families() {
        assert_eq!(
            session(Some("light"), None).theme_choice(),
            (ThemeKind::Paper, Appearance::Light)
        );
        assert_eq!(
            session(Some("dark"), None).theme_choice(),
            (ThemeKind::Graphite, Appearance::Dark)
        );
    }

    #[test]
    fn family_and_appearance_both_survive() {
        assert_eq!(
            session(Some("catppuccin"), Some("light")).theme_choice(),
            (ThemeKind::Catppuccin, Appearance::Light)
        );
        assert_eq!(
            session(Some("tokyo-night"), Some("dark")).theme_choice(),
            (ThemeKind::TokyoNight, Appearance::Dark)
        );
    }

    #[test]
    fn priority_view_and_filter_round_trip() {
        let db = flowlist_core::db::Db::default();
        let view = View::Priority(Priority::High);
        assert_eq!(view.key(), "priority:high");
        assert_eq!(View::from_key(&db, "priority:high"), Some(view));
        assert_eq!(View::Priority(Priority::Low).key(), "priority:low");
        assert_eq!(View::from_key(&db, "priority:none"), None);

        let session = Session {
            priority_filter: Some("medium".to_string()),
            ..Session::default()
        };
        assert_eq!(session.priority_filter(), Some(Priority::Medium));
        assert_eq!(Session::default().priority_filter(), None);
    }

    #[test]
    fn changed_view_and_per_view_options_round_trip() {
        let db = flowlist_core::db::Db::default();
        let view = View::Builtin(BuiltinPerspective::Changed);
        assert_eq!(view.key(), "changed");
        assert_eq!(View::from_key(&db, "changed"), Some(view));

        let session = Session {
            only_available: Some(true),
            follow_system: Some(true),
            view_options: BTreeMap::from([(
                "flagged".to_string(),
                ViewOptions {
                    sort: Some("due".to_string()),
                    group: Some("project".to_string()),
                    show_completed: Some(false),
                    only_available: Some(true),
                    priority: Some("high".to_string()),
                    hide_overdue: Some(true),
                },
            )]),
            ..Session::default()
        };
        let text = serde_json::to_string(&session).unwrap();
        let restored: Session = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.only_available, Some(true));
        assert_eq!(restored.follow_system, Some(true));
        assert_eq!(
            restored.view_options["flagged"].sort.as_deref(),
            Some("due")
        );
        assert_eq!(restored.view_options["flagged"].only_available, Some(true));
        assert_eq!(restored.view_options["flagged"].hide_overdue, Some(true));
        assert_eq!(
            restored.view_options["flagged"].priority.as_deref(),
            Some("high")
        );
    }

    #[test]
    fn missing_appearance_uses_the_family_default() {
        assert_eq!(
            session(Some("vesper"), None).theme_choice(),
            (ThemeKind::Vesper, Appearance::Dark)
        );
        assert_eq!(
            session(Some("mist"), None).theme_choice(),
            (ThemeKind::Mist, Appearance::Light)
        );
        assert_eq!(
            session(None, None).theme_choice(),
            (ThemeKind::Plain, Appearance::Light)
        );
        assert_eq!(
            session(None, Some("dark")).theme_choice(),
            (ThemeKind::Plain, Appearance::Dark)
        );
    }

    #[test]
    fn plain_accents_are_per_appearance_and_validated() {
        let session = Session {
            plain_accent_light: Some("#1E4EB8".to_string()),
            plain_accent_dark: Some("8FB6FF".to_string()),
            ..Session::default()
        };
        assert_eq!(session.plain_accent(Appearance::Light), Some(0x1E4EB8));
        assert_eq!(session.plain_accent(Appearance::Dark), Some(0x8FB6FF));

        let broken = Session {
            plain_accent_light: Some("#nothex".to_string()),
            ..Session::default()
        };
        assert_eq!(broken.plain_accent(Appearance::Light), None);
        assert_eq!(Session::default().plain_accent(Appearance::Dark), None);
    }
}
