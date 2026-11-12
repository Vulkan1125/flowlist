//! Which list the window is showing, and the small amount of state that
//! survives a quit (view, selection, theme, inspector, window size).

use serde::{Deserialize, Serialize};

use flowlist_core::db::Db;
use flowlist_core::ids::{FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use flowlist_core::model::BuiltinPerspective;

use super::theme::Mode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Builtin(BuiltinPerspective),
    Project(ProjectId),
    Folder(FolderId),
    Tag(TagId),
    Custom(PerspectiveId),
}

impl View {
    pub fn key(&self) -> String {
        match self {
            View::Builtin(perspective) => perspective.slug().to_string(),
            View::Project(id) => format!("project:{id}"),
            View::Folder(id) => format!("folder:{id}"),
            View::Tag(id) => format!("tag:{id}"),
            View::Custom(id) => format!("custom:{id}"),
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
            _ => None,
        }
    }

    pub fn title(&self, db: &Db) -> String {
        match self {
            View::Builtin(perspective) => match perspective {
                BuiltinPerspective::Inbox => "Inbox",
                BuiltinPerspective::Projects => "Projects",
                BuiltinPerspective::Forecast => "Forecast",
                BuiltinPerspective::Flagged => "Flagged",
                BuiltinPerspective::Review => "Review",
                BuiltinPerspective::Tags => "Tags",
                BuiltinPerspective::Completed => "Completed",
                BuiltinPerspective::Nearby => "Nearby",
            }
            .to_string(),
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
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            View::Builtin(BuiltinPerspective::Inbox) => "Capture now, organise later",
            View::Builtin(BuiltinPerspective::Projects) => "Every project at a glance",
            View::Builtin(BuiltinPerspective::Forecast) => "What is coming up",
            View::Builtin(BuiltinPerspective::Flagged) => "Your personal priority list",
            View::Builtin(BuiltinPerspective::Review) => "Keep projects honest",
            View::Builtin(BuiltinPerspective::Tags) => "Group work by context",
            View::Builtin(BuiltinPerspective::Completed) => "Recently finished work",
            View::Builtin(BuiltinPerspective::Nearby) => "Contexts with a location",
            View::Project(_) => "Project outline",
            View::Folder(_) => "Projects in this folder",
            View::Tag(_) => "Everything with this context",
            View::Custom(_) => "Saved perspective",
        }
    }

    pub fn is_inbox(&self) -> bool {
        matches!(self, View::Builtin(BuiltinPerspective::Inbox))
    }

    pub fn shows_capture(&self) -> bool {
        match self {
            View::Builtin(BuiltinPerspective::Inbox) => true,
            View::Project(_) => true,
            View::Folder(_) | View::Tag(_) | View::Custom(_) => true,
            _ => false,
        }
    }
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
    pub window: Option<(f32, f32)>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub show_completed: Option<bool>,
    #[serde(default)]
    pub vim_mode: Option<bool>,
}

impl Session {
    pub fn view(&self, db: &Db) -> View {
        self.view
            .as_deref()
            .and_then(|key| View::from_key(db, key))
            .unwrap_or(View::Builtin(BuiltinPerspective::Inbox))
    }

    pub fn theme_mode(&self) -> Option<Mode> {
        self.theme.as_deref().and_then(Mode::from_slug)
    }

    pub fn selected_tasks(&self, db: &Db) -> Vec<TaskId> {
        self.selected
            .iter()
            .filter_map(|id| db.resolve_task(id).ok())
            .collect()
    }
}
