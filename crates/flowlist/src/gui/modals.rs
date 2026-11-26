//! Modal sheets: new project/folder/tag/perspective, import, settings,
//! locations and the keyboard help card.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Bounds, Context, CursorStyle, IntoElement, ObjectFit,
    Window, div, img, point, prelude::*, px, rgb, rgba, size,
};

use flowlist_core::command::Command;
use flowlist_core::db::DateConflict;
use flowlist_core::ids::{ContainerId, FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use flowlist_core::model::{
    AlertEvent, AvailabilityFilter, BuiltinPerspective, Condition, DateField, DateRangeFilter,
    DateUnit, DueWindow, FilterStatus, GroupBy, IconSpec, ItemKindFilter, Priority, ProjectIcon,
    ProjectKind, ProjectStatusFilter, SidebarOptions, SortBy, TagStatusFilter, TaskStatus,
};
use flowlist_core::query;
use flowlist_core::time::now;

use super::drag::{
    DragPerspectiveResize, DragRule, DragSettingsResize, DragSettingsScroll, rule_ghost,
    settings_resize_ghost, settings_scroll_ghost,
};
use super::icons::Icon;
use super::perspective::{
    DeferChoice, DueChoice, GroupOperator, IconTab, LayoutField, PerspectiveForm, PerspectiveTab,
    RuleKind, RuleNode, RuleTemplate, ViewChoice, ViewField, rule_summary,
};
use super::screen_picker;
use super::DateField as PickerDateField;
use super::text::TextBuffer;
use super::theme::{self, Appearance, PLAIN_ACCENTS, Theme, ThemeKind};
use super::widgets::TipExt;
use super::{FlowlistApp, icons, widgets};
use crate::app_icon::{self, AppIcon};

/// The smallest thumb Zed's regular scrollbar uses.
const SETTINGS_SCROLL_MIN_THUMB: f32 = 25.0;

/// What a rename sheet is editing. Keeping the typed id means a rename can
/// never accidentally target the wrong entity kind.
#[derive(Clone, Debug)]
pub enum EntityRef {
    Project(flowlist_core::ids::ProjectId),
    Folder(flowlist_core::ids::FolderId),
    Tag(flowlist_core::ids::TagId),
    Perspective(flowlist_core::ids::PerspectiveId),
}

/// What an attachment link sheet is attached to. Keeping the typed id means a
/// link can never land on the wrong task or project.
#[derive(Clone, Debug)]
pub enum AttachmentTarget {
    Task(TaskId),
    Project(ProjectId),
}

/// One accordion group in the Sidebar settings matrix. Exactly one group is
/// open at a time, so the sheet never turns into a long list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarGroup {
    Perspectives,
    Priorities,
    Library,
    Saved,
    Tags,
}

impl SidebarGroup {
    pub(super) fn slug(self) -> &'static str {
        match self {
            SidebarGroup::Perspectives => "perspectives",
            SidebarGroup::Priorities => "priorities",
            SidebarGroup::Library => "library",
            SidebarGroup::Saved => "saved",
            SidebarGroup::Tags => "tags",
        }
    }
}

pub enum Modal {
    Rename {
        entity: EntityRef,
        name: TextBuffer,
    },
    /// Change how often one project asks to be reviewed after it exists.
    /// The New Project sheet offers the same number at creation time.
    ReviewInterval {
        id: ProjectId,
        name: String,
        days: TextBuffer,
        next_review: TextBuffer,
    },
    NewProject {
        /// None creates a project; Some edits that project in place.
        editing: Option<ProjectId>,
        name: TextBuffer,
        notes: TextBuffer,
        /// Review cadence for this project, in days. Starts at the library
        /// default and can be changed per project.
        review_days: TextBuffer,
        defer: TextBuffer,
        planned: TextBuffer,
        due: TextBuffer,
        kind: ProjectKind,
        icon: IconSpec,
        /// The icon card stays folded away until the current icon is clicked.
        icon_open: bool,
        icon_tab: IconTab,
        icon_query: TextBuffer,
        fullscreen_reminder: bool,
    },
    /// A move that would change stored task dates. Nothing has moved yet.
    AdjustDates {
        command: Command,
        conflicts: Vec<DateConflict>,
    },
    /// A small sheet for choosing one project glyph.
    IconPicker {
        project: ProjectId,
        query: TextBuffer,
        tab: IconTab,
    },
    /// Typing a project path that does not exist yet asks before creating it,
    /// so a typo cannot silently fill the sidebar with projects.
    CreateProject {
        task: TaskId,
        path: String,
    },
    /// Deleting a project trashes everything inside it. The sheet names the
    /// project and asks first; Undo still restores it in one step.
    DeleteProject {
        id: ProjectId,
        name: String,
    },
    /// Deleting a folder moves its projects up to its parent (or the library
    /// root). The sheet names the folder and asks first; Undo restores it.
    DeleteFolder {
        id: FolderId,
        name: String,
    },
    /// Add one absolute custom notification to a task or project.
    CustomNotification {
        target: super::NotificationTarget,
        at: TextBuffer,
    },
    /// Add one relative notification anchored to Defer, Planned or Due.
    RelativeNotification {
        target: super::NotificationTarget,
        event: AlertEvent,
        offset: TextBuffer,
    },
    /// Delete several selected projects and folders in one undo step.
    DeleteContainers {
        projects: Vec<ProjectId>,
        folders: Vec<FolderId>,
    },
    /// Deleting a tag takes it off every task; the sheet names it first.
    DeleteTag {
        id: TagId,
        name: String,
    },
    /// Deleting a repeating task asks whether just this occurrence or the
    /// whole series goes to Trash. The sheet defaults to the safe choice.
    DeleteTask {
        ids: Vec<TaskId>,
        title: String,
        occurrences: usize,
        whole: bool,
    },
    /// Switching a task that has child tasks to one-off steps first warns
    /// that the children will be turned into checklist items.
    HideSubtasks {
        id: TaskId,
        title: String,
        count: usize,
    },
    /// Changing the priority of a repeating task asks whether the new value
    /// belongs to this occurrence alone or to every occurrence in the series.
    PriorityScope {
        id: TaskId,
        priority: Option<Priority>,
        occurrences: usize,
        all: bool,
    },
    /// Removing an attachment asks first. The file or link is only unlinked;
    /// nothing on disk is touched, and Undo puts the row back.
    RemoveAttachment {
        target: AttachmentTarget,
        path: String,
        name: String,
    },
    /// Remove the app and its command. The sheet chooses whether the library
    /// stays; confirming schedules a cleanup helper and quits.
    Uninstall {
        purge: bool,
    },
    /// Adding an http(s) link to a task or project. Links share the attachment
    /// list with file paths, so the row badge and the Attachments view already
    /// include them.
    AttachLink {
        target: AttachmentTarget,
        url: TextBuffer,
        name: TextBuffer,
        /// When set, this is the original attachment string being renamed.
        replace: Option<String>,
    },
    NewFolder {
        /// None creates a folder; Some edits that folder in place.
        editing: Option<FolderId>,
        name: TextBuffer,
        fullscreen_reminder: bool,
    },
    NewTag {
        name: TextBuffer,
    },
    Perspective {
        form: PerspectiveForm,
    },
    Import {
        text: TextBuffer,
        merge: bool,
    },
    Settings {
        review_days: TextBuffer,
        completed_days: TextBuffer,
        changed_days: TextBuffer,
        due_soon_days: TextBuffer,
        radius: TextBuffer,
        trash_days: TextBuffer,
        vim_mode: bool,
        /// Due-date reminders, per device.
        notifications: bool,
        /// Show macOS Calendar events in Forecast, per device.
        calendar_events: bool,
        /// Calendar identifiers hidden from Forecast.
        calendar_hidden: BTreeSet<String>,
        /// Show only due dates in Forecast, per device.
        forecast_due_only: bool,
        /// Take over the screen when a task comes due, per device.
        fullscreen_reminders: bool,
        /// Pause system media while a full-screen reminder is on screen.
        pause_media_on_reminder: bool,
        reminder_background: super::background::AbstractStyle,
        reminder_match_theme: bool,
        reminder_wallpaper_light: Option<std::path::PathBuf>,
        reminder_wallpaper_dark: Option<std::path::PathBuf>,
        reminder_wallpaper_separate: bool,
        reminder_use_wallpaper: bool,
        reminder_wallpaper_opacity: f32,
        fullscreen_snooze_minutes: u32,
        wallpaper_light: Option<std::path::PathBuf>,
        wallpaper_dark: Option<std::path::PathBuf>,
        wallpaper_separate: bool,
        use_wallpaper: bool,
        wallpaper_opacity: f32,
        wallpaper_note: Option<String>,
        reminder_wallpaper_note: Option<String>,
        /// Keep finished subtasks visible under their parent while the view
        /// hides completed work, per device.
        keep_completed_subtasks: bool,
        /// Count only due items in Forecast badges.
        forecast_due_counts: bool,
        /// Tag whose undated actions enter Forecast.
        forecast_tag: TextBuffer,
        /// Show flagged undated actions in Forecast.
        forecast_flagged: bool,
        default_defer_notification: bool,
        default_planned_notification: bool,
        default_latest_start_notification: bool,
        default_due_notification: bool,
        /// Time used by date-only absolute custom notifications.
        custom_notification_time: TextBuffer,
        /// Keep the Focus banner below the title bar.
        focus_banner: bool,
        /// Show button titles under the header icons.
        toolbar_labels: bool,
        /// Drop saved views from the rail while their filter matches nothing.
        hide_empty_views: bool,
        follow_system: bool,
        theme: ThemeKind,
        appearance: Appearance,
        sidebar: SidebarOptions,
        /// Show the macOS close/minimise/zoom buttons over the header.
        traffic_lights: bool,
        accent_light: Option<u32>,
        accent_dark: Option<u32>,
        /// A quiet line under the accent controls, used when the system
        /// eyedropper is unavailable or times out.
        accent_note: Option<String>,
        section: SettingsSection,
        sidebar_group: Option<SidebarGroup>,
    },
    Location {
        lat: TextBuffer,
        lon: TextBuffer,
    },
    TagLocation {
        tag: flowlist_core::ids::TagId,
        lat: TextBuffer,
        lon: TextBuffer,
    },
    Help,
}

/// The left rail of Settings. Sections keep the sheet navigable as more
/// library-wide preferences arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsSection {
    Appearance,
    Sidebar,
    Workflow,
    Reminders,
    Calendar,
    Nearby,
    Trash,
    Mobile,
    About,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 9] = [
        SettingsSection::Appearance,
        SettingsSection::Sidebar,
        SettingsSection::Workflow,
        SettingsSection::Reminders,
        SettingsSection::Calendar,
        SettingsSection::Nearby,
        SettingsSection::Trash,
        SettingsSection::Mobile,
        SettingsSection::About,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            SettingsSection::Appearance => "appearance",
            SettingsSection::Sidebar => "sidebar",
            SettingsSection::Workflow => "workflow",
            SettingsSection::Reminders => "reminders",
            SettingsSection::Calendar => "calendar",
            SettingsSection::Nearby => "nearby",
            SettingsSection::Trash => "trash",
            SettingsSection::Mobile => "mobile",
            SettingsSection::About => "about",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Sidebar => "Sidebar",
            SettingsSection::Workflow => "Workflow",
            SettingsSection::Reminders => "Reminders",
            SettingsSection::Calendar => "Calendar",
            SettingsSection::Nearby => "Nearby",
            SettingsSection::Trash => "Trash",
            SettingsSection::Mobile => "Mobile",
            SettingsSection::About => "About",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            SettingsSection::Appearance => {
                "Theme, accent, window controls and how light and dark are chosen."
            }
            SettingsSection::Sidebar => "Which sections, items and counts the rail shows.",
            SettingsSection::Workflow => "Keyboard, review cadence and history windows.",
            SettingsSection::Reminders => {
                "Due notifications and full-screen reminder timing and backdrops."
            }
            SettingsSection::Calendar => {
                "Read-only macOS Calendar events and which calendars Forecast shows."
            }
            SettingsSection::Nearby => "How distance ranks tasks with tagged locations.",
            SettingsSection::Trash => "How long deleted tasks stay restorable.",
            SettingsSection::Mobile => "Pair your Android phone and keep it in sync with this Mac.",
            SettingsSection::About => "Version, library location and update checks.",
        }
    }

    /// Which numeric fields live in this section. Tab and the text buffers are
    /// scoped to it, so typing in Appearance can never edit a hidden default.
    pub fn fields(self) -> &'static [usize] {
        match self {
            SettingsSection::Workflow => &[0, 1, 2, 5, 7],
            SettingsSection::Reminders => &[6],
            SettingsSection::Nearby => &[3],
            SettingsSection::Trash => &[4],
            SettingsSection::Appearance
            | SettingsSection::Sidebar
            | SettingsSection::Reminders
            | SettingsSection::Calendar
            | SettingsSection::Mobile
            | SettingsSection::About => &[],
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            SettingsSection::Appearance => Icon::Palette,
            SettingsSection::Sidebar => Icon::PanelRight,
            SettingsSection::Workflow => Icon::Review,
            SettingsSection::Reminders => Icon::Clock,
            SettingsSection::Calendar => Icon::Calendar,
            SettingsSection::Nearby => Icon::Crosshair,
            SettingsSection::Trash => Icon::Trash,
            SettingsSection::Mobile => Icon::Smartphone,
            SettingsSection::About => Icon::Info,
        }
    }
}

/// Turn the old section-level count flags into explicit per-item states the
/// first time the sidebar matrix is opened. Saving Settings then preserves the
/// rail the user already has instead of showing every count at once.
fn materialize_sidebar_counts(sidebar: &mut SidebarOptions, db: &flowlist_core::db::Db) {
    if sidebar.counts_configured {
        return;
    }
    let mut keys: Vec<String> = BuiltinPerspective::ALL
        .iter()
        .map(|perspective| format!("builtin:{}", perspective.slug()))
        .collect();
    keys.extend(
        Priority::ALL
            .iter()
            .map(|priority| format!("priority:{}", priority.slug())),
    );
    for row in db.flatten_containers() {
        keys.push(match row.container {
            ContainerId::Folder(id) => format!("folder:{id}"),
            ContainerId::Project(id) => format!("project:{id}"),
        });
    }
    keys.extend(
        db.perspectives
            .values()
            .map(|perspective| format!("custom:{}", perspective.id)),
    );
    keys.extend(db.tags.keys().map(|tag| format!("tag:{tag}")));
    sidebar.counts_hidden = keys
        .into_iter()
        .filter(|key| sidebar.is_count_hidden(key))
        .collect();
    sidebar.counts_configured = true;
}

impl Modal {
    pub fn rename(app: &FlowlistApp, entity: EntityRef) -> Self {
        let db = app.store.db();
        let name = match &entity {
            EntityRef::Project(id) => db.projects.get(id).map(|project| project.name.clone()),
            EntityRef::Folder(id) => db.folders.get(id).map(|folder| folder.name.clone()),
            EntityRef::Tag(id) => db.tags.get(id).map(|tag| tag.name.clone()),
            EntityRef::Perspective(id) => db
                .perspectives
                .get(id)
                .map(|perspective| perspective.name.clone()),
        }
        .unwrap_or_default();
        // The name is already selected, so typing replaces it and a single
        // Backspace clears it. Previously the caret sat at the end and the
        // only way to change a name was to erase it one key at a time.
        let mut name = TextBuffer::new(name);
        name.select_all();
        Modal::Rename { entity, name }
    }

    pub fn delete_project(app: &FlowlistApp, id: ProjectId) -> Self {
        let name = app
            .store
            .db()
            .projects
            .get(&id)
            .map(|project| project.name.clone())
            .unwrap_or_default();
        Modal::DeleteProject { id, name }
    }

    pub fn delete_folder(app: &FlowlistApp, id: FolderId) -> Self {
        let name = app
            .store
            .db()
            .folders
            .get(&id)
            .map(|folder| folder.name.clone())
            .unwrap_or_default();
        Modal::DeleteFolder { id, name }
    }

    pub fn new_project(app: &FlowlistApp) -> Self {
        Modal::NewProject {
            editing: None,
            name: TextBuffer::empty(),
            notes: TextBuffer::empty(),
            review_days: TextBuffer::new(app.store.db().settings.default_review_days.to_string()),
            defer: TextBuffer::empty(),
            planned: TextBuffer::empty(),
            due: TextBuffer::empty(),
            kind: ProjectKind::Parallel,
            icon: IconSpec::default(),
            icon_open: false,
            icon_tab: IconTab::Icons,
            icon_query: TextBuffer::empty(),
            fullscreen_reminder: false,
        }
    }

    pub fn edit_project(app: &FlowlistApp, id: &ProjectId) -> Option<Self> {
        let project = app.store.db().projects.get(id)?;
        Some(Modal::NewProject {
            editing: Some(id.clone()),
            name: TextBuffer::new(project.name.clone()),
            notes: TextBuffer::new(project.notes.clone()),
            review_days: TextBuffer::new(project.review_interval_days.to_string()),
            defer: TextBuffer::new(
                project
                    .defer
                    .map(|stamp| stamp.to_input_string())
                    .unwrap_or_default(),
            ),
            planned: TextBuffer::new(
                project
                    .planned
                    .map(|stamp| stamp.to_input_string())
                    .unwrap_or_default(),
            ),
            due: TextBuffer::new(
                project
                    .due
                    .map(|stamp| stamp.to_input_string())
                    .unwrap_or_default(),
            ),
            kind: project.kind,
            icon: project.icon.clone(),
            icon_open: false,
            icon_tab: IconTab::Icons,
            icon_query: TextBuffer::empty(),
            fullscreen_reminder: project.fullscreen_reminder,
        })
    }

    pub fn adjust_dates(command: Command, conflicts: Vec<DateConflict>) -> Self {
        Modal::AdjustDates { command, conflicts }
    }

    pub fn icon_picker(project: ProjectId) -> Self {
        Modal::IconPicker {
            project,
            query: TextBuffer::empty(),
            tab: IconTab::Icons,
        }
    }

    pub fn delete_tag(app: &FlowlistApp, id: &TagId) -> Self {
        let name = app
            .store
            .db()
            .tags
            .get(id)
            .map(|tag| tag.name.clone())
            .unwrap_or_else(|| "tag".to_string());
        Modal::DeleteTag {
            id: id.clone(),
            name,
        }
    }

    pub fn custom_notification(target: super::NotificationTarget) -> Self {
        Modal::CustomNotification {
            target,
            at: TextBuffer::empty(),
        }
    }

    pub fn relative_notification(
        target: super::NotificationTarget,
        event: AlertEvent,
    ) -> Self {
        Modal::RelativeNotification {
            target,
            event,
            offset: TextBuffer::new("-15m"),
        }
    }

    pub fn delete_containers(projects: Vec<ProjectId>, folders: Vec<FolderId>) -> Self {
        Modal::DeleteContainers { projects, folders }
    }

    pub fn delete_task(app: &FlowlistApp, ids: Vec<TaskId>, title: String) -> Self {
        let occurrences = ids
            .iter()
            .map(|id| app.store.db().series_task_ids(id).len())
            .max()
            .unwrap_or(1)
            .max(1);
        Modal::DeleteTask {
            ids,
            title,
            occurrences,
            whole: false,
        }
    }

    pub fn priority_scope(app: &FlowlistApp, id: TaskId, priority: Option<Priority>) -> Self {
        let occurrences = app.store.db().series_task_ids(&id).len().max(1);
        Modal::PriorityScope {
            id,
            priority,
            occurrences,
            all: false,
        }
    }

    pub fn hide_subtasks(app: &FlowlistApp, id: TaskId) -> Self {
        let db = app.store.db();
        let title = db
            .tasks
            .get(&id)
            .map(|task| task.title.clone())
            .unwrap_or_default();
        let count = db
            .task_subtree(&id)
            .into_iter()
            .skip(1)
            .filter(|child| db.tasks.get(child).is_some_and(|task| !task.is_trashed()))
            .count();
        Modal::HideSubtasks { id, title, count }
    }

    pub fn attach_link(target: AttachmentTarget) -> Self {
        Modal::AttachLink {
            target,
            url: TextBuffer::empty(),
            name: TextBuffer::empty(),
            replace: None,
        }
    }

    pub fn rename_link(target: AttachmentTarget, value: &str) -> Self {
        let url = flowlist_core::model::attachment::attachment_url(value);
        let name = value.split_once('\t').map(|(_, name)| name).unwrap_or("");
        Modal::AttachLink {
            target,
            url: TextBuffer::new(url),
            name: TextBuffer::new(name),
            replace: Some(value.to_string()),
        }
    }

    pub fn new_folder(_app: &FlowlistApp) -> Self {
        Modal::NewFolder {
            editing: None,
            name: TextBuffer::empty(),
            fullscreen_reminder: false,
        }
    }

    pub fn edit_folder(app: &FlowlistApp, id: &FolderId) -> Option<Self> {
        let folder = app.store.db().folders.get(id)?;
        Some(Modal::NewFolder {
            editing: Some(id.clone()),
            name: TextBuffer::new(folder.name.clone()),
            fullscreen_reminder: folder.fullscreen_reminder,
        })
    }

    pub fn new_tag(_app: &FlowlistApp) -> Self {
        Modal::NewTag {
            name: TextBuffer::empty(),
        }
    }

    pub fn new_perspective(_app: &FlowlistApp) -> Self {
        Modal::Perspective {
            form: PerspectiveForm::new(),
        }
    }

    /// Open the editor on an existing perspective. The form keeps the id so
    /// saving updates the saved query instead of creating a second one.
    pub fn perspective(app: &FlowlistApp, id: &PerspectiveId) -> Option<Self> {
        let perspective = app.store.db().perspectives.get(id)?;
        Some(Modal::Perspective {
            form: PerspectiveForm::edit(perspective),
        })
    }

    pub fn settings(app: &FlowlistApp) -> Self {
        let db = app.store.db();
        let settings = &db.settings;
        let mut sidebar = settings.sidebar.clone();
        materialize_sidebar_counts(&mut sidebar, db);
        Modal::Settings {
            review_days: TextBuffer::new(settings.default_review_days.to_string()),
            completed_days: TextBuffer::new(settings.show_completed_days.to_string()),
            changed_days: TextBuffer::new(settings.changed_days.to_string()),
            due_soon_days: TextBuffer::new(settings.due_soon_days.to_string()),
            radius: TextBuffer::new(settings.nearby_radius_km.to_string()),
            trash_days: TextBuffer::new(settings.trash_retention_days.to_string()),
            vim_mode: app.vim_mode,
            notifications: app.due_notifications,
            calendar_events: app.calendar_events_enabled,
            calendar_hidden: app.calendar_hidden.clone(),
            forecast_due_only: app.forecast_due_only,
            fullscreen_reminders: app.fullscreen_reminders,
            pause_media_on_reminder: app.pause_media_on_reminder,
            reminder_background: app.reminder_background,
            reminder_match_theme: app.reminder_match_theme,
            reminder_wallpaper_light: app.reminder_wallpaper_light.clone(),
            reminder_wallpaper_dark: app.reminder_wallpaper_dark.clone(),
            reminder_wallpaper_separate: app.reminder_wallpaper_separate,
            reminder_use_wallpaper: app.reminder_use_wallpaper,
            reminder_wallpaper_opacity: app.reminder_wallpaper_opacity,
            fullscreen_snooze_minutes: app.reminder_snooze_minutes,
            wallpaper_light: app.wallpaper_light.clone(),
            wallpaper_dark: app.wallpaper_dark.clone(),
            wallpaper_separate: app.wallpaper_separate,
            use_wallpaper: app.use_wallpaper,
            wallpaper_opacity: app.wallpaper_opacity,
            wallpaper_note: None,
            reminder_wallpaper_note: None,
            keep_completed_subtasks: app.keep_completed_subtasks,
            forecast_due_counts: app.store.db().settings.forecast_due_counts,
            forecast_tag: TextBuffer::new(
                app.store
                    .db()
                    .settings
                    .forecast_tag
                    .as_ref()
                    .map(|tag| app.store.db().tag_path(tag))
                    .unwrap_or_default(),
            ),
            forecast_flagged: app.store.db().settings.forecast_flagged,
            default_defer_notification: app.store.db().settings.default_defer_notification,
            default_planned_notification: app.store.db().settings.default_planned_notification,
            default_latest_start_notification: app
                .store
                .db()
                .settings
                .default_latest_start_notification,
            default_due_notification: app.store.db().settings.default_due_notification,
            custom_notification_time: TextBuffer::new(
                app.store
                    .db()
                    .settings
                    .default_custom_notification_time
                    .map(|time| time.format("%H:%M").to_string())
                    .unwrap_or_default(),
            ),
            focus_banner: app.show_focus_banner,
            toolbar_labels: app.show_toolbar_labels,
            hide_empty_views: app.hide_empty_views,
            follow_system: app.follow_system,
            theme: app.theme.kind(),
            appearance: app.theme.appearance(),
            sidebar,
            traffic_lights: app.traffic_lights_visible,
            accent_light: app.plain_accent_light,
            accent_dark: app.plain_accent_dark,
            accent_note: None,
            section: SettingsSection::Appearance,
            sidebar_group: None,
        }
    }

    pub fn location(app: &FlowlistApp) -> Self {
        let location = app.store.db().settings.current_location;
        Modal::Location {
            lat: TextBuffer::new(location.map(|geo| geo.lat.to_string()).unwrap_or_default()),
            lon: TextBuffer::new(location.map(|geo| geo.lon.to_string()).unwrap_or_default()),
        }
    }

    pub fn tag_location(app: &FlowlistApp, tag: flowlist_core::ids::TagId) -> Self {
        let location = app.store.db().tags.get(&tag).and_then(|tag| tag.location);
        Modal::TagLocation {
            tag,
            lat: TextBuffer::new(location.map(|geo| geo.lat.to_string()).unwrap_or_default()),
            lon: TextBuffer::new(location.map(|geo| geo.lon.to_string()).unwrap_or_default()),
        }
    }

    pub fn remove_attachment(target: AttachmentTarget, path: String) -> Self {
        let name = flowlist_core::model::attachment::attachment_name(&path);
        Modal::RemoveAttachment { target, path, name }
    }

    pub fn uninstall() -> Self {
        Modal::Uninstall { purge: false }
    }

    pub fn review_interval(app: &FlowlistApp, id: ProjectId) -> Self {
        let project = app.store.db().projects.get(&id);
        let name = project
            .map(|project| project.name.clone())
            .unwrap_or_default();
        let days = project
            .map(|project| project.review_interval_days.max(1))
            .unwrap_or(7);
        Modal::ReviewInterval {
            id,
            name,
            days: TextBuffer::new(days.to_string()),
            next_review: TextBuffer::new(
                project
                    .map(|project| project.next_review.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            ),
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Modal::Rename { .. } => "Rename",
            Modal::ReviewInterval { .. } => "Review",
            Modal::NewProject { editing, .. } => {
                if editing.is_some() {
                    "Edit project"
                } else {
                    "New project"
                }
            }
            Modal::IconPicker { .. } => "Project icon",
            Modal::AdjustDates { .. } => "Adjust dates?",
            Modal::CreateProject { .. } => "Create project",
            Modal::DeleteProject { .. } => "Delete project",
            Modal::DeleteFolder { .. } => "Delete folder",
            Modal::CustomNotification { .. } => "Custom notification",
            Modal::RelativeNotification { .. } => "Relative notification",
            Modal::DeleteContainers { .. } => "Delete selected items",
            Modal::DeleteTag { .. } => "Delete tag",
            Modal::DeleteTask { .. } => "Delete repeating task",
            Modal::HideSubtasks { .. } => "Convert to one-off steps?",
            Modal::PriorityScope { .. } => "Repeating task priority",
            Modal::RemoveAttachment { .. } => "Remove attachment",
            Modal::Uninstall { .. } => "Uninstall Flowlist",
            Modal::AttachLink { replace, .. } if replace.is_some() => "Rename link",
            Modal::AttachLink { .. } => "Attach link",
            Modal::NewFolder { editing, .. } => {
                if editing.is_some() {
                    "Edit folder"
                } else {
                    "New folder"
                }
            }
            Modal::NewTag { .. } => "New tag",
            Modal::Perspective { .. } => "Perspective",
            Modal::Import { .. } => "Import library",
            Modal::Settings { .. } => "Settings",
            Modal::Location { .. } => "Set location",
            Modal::TagLocation { .. } => "Tag location",
            Modal::Help => "Keyboard & mouse",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Modal::Rename { .. } => "Names are shared by the sidebar, CLI and MCP server.",
            Modal::ReviewInterval { .. } => {
                "When the next review is due, and how often it comes back."
            }
            Modal::NewProject { editing, .. } => {
                if editing.is_some() {
                    "Change the project's name, notes, review cadence, type or icon."
                } else {
                    "Projects hold an outline of tasks; folders keep projects together."
                }
            }
            Modal::IconPicker { .. } => "Choose a named icon or an emoji for this project.",
            Modal::AdjustDates { command, .. } => match command {
                Command::UpdateTaskAdjusted { .. } => {
                    "If you continue, the task date will be adjusted to fit the project."
                }
                Command::UpdateProjectAdjusted { .. } => {
                    "If you continue, these task dates will be adjusted to fit the project’s new dates."
                }
                _ => {
                    "If you continue, these task dates will be adjusted to fit the project or parent."
                }
            },
            Modal::CreateProject { .. } => {
                "This path is not in the library yet. Create it and file the task there?"
            }
            Modal::DeleteProject { .. } => {
                "Its tasks move to Trash. Undo restores the project and its tasks."
            }
            Modal::DeleteFolder { .. } => {
                "Its projects and their tasks move to Trash with it. Undo restores everything."
            }
            Modal::CustomNotification { .. } => {
                "An absolute date and time for this item. It does not follow the item's dates."
            }
            Modal::RelativeNotification { .. } => {
                "An alert that moves with the item's dates. Negative fires before; positive after."
            }
            Modal::DeleteContainers { .. } => {
                "Their tasks move to Trash. Undo restores everything in one step."
            }
            Modal::DeleteTag { .. } => {
                "Tasks keep their other tags; this tag is removed from them."
            }
            Modal::DeleteTask { .. } => {
                "This task repeats. Trash this occurrence, or the whole series; Undo restores everything."
            }
            Modal::HideSubtasks { .. } => {
                "Each subtask becomes a checklist step. Task-only details such as notes, dates, tags and priority are removed; the title and completion state are kept."
            }
            Modal::PriorityScope { .. } => {
                "This task repeats. Apply the new priority to this occurrence, or to every occurrence in the series."
            }
            Modal::RemoveAttachment { .. } => {
                "Only the link on this item goes away; the file stays on disk."
            }
            Modal::Uninstall { .. } => {
                "Remove the app and the flowlist command. Choose whether your library stays."
            }
            Modal::AttachLink { replace, .. } if replace.is_some() => {
                "Give the link a shorter name. Leave the name empty to show the address."
            }
            Modal::AttachLink { .. } => {
                "Paste an http(s) address; it opens in your browser. Undo removes it."
            }
            Modal::NewFolder { editing, .. } => {
                if editing.is_some() {
                    "Rename the folder. Its projects and nested folders stay put."
                } else {
                    "Folders group projects. They can be nested."
                }
            }
            Modal::NewTag { .. } => "Tags are contexts: places, tools or people.",
            Modal::Perspective { .. } => {
                "Saved filters with their own conditions, grouping and sort order."
            }
            Modal::Import { .. } => {
                "Paste Flowlist JSON or TaskPaper text. Merge keeps existing items."
            }
            // Settings is a hub of sections; the pane describes itself.
            Modal::Settings { .. } => "",
            Modal::Location { .. } => {
                "Nearby uses this point and the radius below to rank tagged tasks."
            }
            Modal::TagLocation { .. } => "Give this context a latitude and longitude.",
            Modal::Help => "",
        }
    }

    /// Project creation sheets carry their own Cancel button in the footer,
    /// so they do not need a second close control in the top corner.
    pub fn shows_close(&self) -> bool {
        !matches!(self, Modal::NewProject { .. } | Modal::CreateProject { .. })
    }

    pub fn confirm_label(&self) -> &'static str {
        match self {
            Modal::CreateProject { .. } => "Create",
            Modal::DeleteProject { .. } | Modal::DeleteTag { .. } => "Delete",
            Modal::DeleteFolder { .. } | Modal::DeleteContainers { .. } => "Delete",
            Modal::CustomNotification { .. } | Modal::RelativeNotification { .. } => "Add",
            Modal::DeleteTask { .. } => "Delete",
            Modal::HideSubtasks { .. } => "Convert",
            Modal::PriorityScope { .. } => "Apply",
            Modal::RemoveAttachment { .. } => "Remove",
            Modal::Uninstall { .. } => "Uninstall",
            Modal::IconPicker { .. } => "Close",
            Modal::AdjustDates { command, .. } => match command {
                Command::MoveTasksAdjusted { .. } => "Adjust and move",
                Command::UpdateTaskAdjusted { .. } => "Adjust and assign",
                Command::UpdateProjectAdjusted { .. } => "Adjust and save",
                _ => "Adjust and save",
            },
            Modal::AttachLink { replace, .. } if replace.is_some() => "Rename",
            Modal::AttachLink { .. } => "Attach",
            _ => "Save",
        }
    }

    pub fn field_count(&self) -> usize {
        match self {
            Modal::Rename { .. } => 1,
            Modal::ReviewInterval { .. } => 2,
            Modal::NewProject { icon_open, .. } => {
                if *icon_open {
                    7
                } else {
                    6
                }
            }
            Modal::IconPicker { .. } => 1,
            Modal::AdjustDates { .. } => 0,
            Modal::CreateProject { .. } => 0,
            Modal::DeleteProject { .. } => 0,
            Modal::CustomNotification { .. } | Modal::RelativeNotification { .. } => 1,
            Modal::DeleteFolder { .. }
            | Modal::DeleteContainers { .. }
            | Modal::DeleteTag { .. } => 0,
            Modal::DeleteTask { .. } => 0,
            Modal::HideSubtasks { .. } => 0,
            Modal::PriorityScope { .. } => 0,
            Modal::RemoveAttachment { .. } => 0,
            Modal::Uninstall { .. } => 0,
            Modal::AttachLink { .. } => 2,
            Modal::NewFolder { .. } | Modal::NewTag { .. } => 1,
            Modal::Perspective { .. } => 10,
            Modal::Import { .. } => 1,
            Modal::Settings { .. } => 8,
            Modal::Location { .. } | Modal::TagLocation { .. } => 2,
            Modal::Help => 0,
        }
    }

    pub fn field(&self, index: usize) -> Option<&TextBuffer> {
        match self {
            Modal::Rename { name, .. } => (index == 0).then_some(name),
            Modal::ReviewInterval { days, next_review, .. } => match index {
                0 => Some(days),
                1 => Some(next_review),
                _ => None,
            },
            Modal::NewProject {
                name,
                notes,
                review_days,
                defer,
                planned,
                due,
                icon_query,
                ..
            } => match index {
                0 => Some(name),
                1 => Some(notes),
                2 => Some(review_days),
                3 => Some(defer),
                4 => Some(planned),
                5 => Some(due),
                6 => Some(icon_query),
                _ => None,
            },
            Modal::IconPicker { query, .. } => (index == 0).then_some(query),
            Modal::AdjustDates { .. } => None,
            Modal::CustomNotification { at, .. } => (index == 0).then_some(at),
            Modal::RelativeNotification { offset, .. } => (index == 0).then_some(offset),
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. }
            | Modal::DeleteContainers { .. }
            | Modal::DeleteTag { .. } => None,
            Modal::DeleteTask { .. } => None,
            Modal::HideSubtasks { .. } => None,
            Modal::PriorityScope { .. } => None,
            Modal::RemoveAttachment { .. } | Modal::Uninstall { .. } => None,
            Modal::AttachLink { url, name, .. } => match index {
                0 => Some(url),
                1 => Some(name),
                _ => None,
            },
            Modal::NewFolder { name, .. } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => match index {
                0 => Some(&form.name),
                1 => Some(&form.description),
                2 if form.tab == PerspectiveTab::Simple => Some(&form.text),
                3 if form.tab == PerspectiveTab::Simple => Some(&form.estimate_min),
                4 if form.tab == PerspectiveTab::Simple => Some(&form.estimate_max),
                5 => Some(&form.icon_query),
                8 if form.tab == PerspectiveTab::Comprehensive => form.editing_buffer(0),
                9 if form.tab == PerspectiveTab::Comprehensive => form.editing_buffer(1),
                _ => None,
            },
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                changed_days,
                due_soon_days,
                radius,
                trash_days,
                forecast_tag,
                custom_notification_time,
                section,
                ..
            } => match index {
                0 if section.fields().contains(&0) => Some(review_days),
                1 if section.fields().contains(&1) => Some(completed_days),
                2 if section.fields().contains(&2) => Some(changed_days),
                3 if section.fields().contains(&3) => Some(radius),
                4 if section.fields().contains(&4) => Some(trash_days),
                5 if section.fields().contains(&5) => Some(forecast_tag),
                6 if section.fields().contains(&6) => Some(custom_notification_time),
                7 if section.fields().contains(&7) => Some(due_soon_days),
                _ => None,
            },
            Modal::Location { lat, lon } | Modal::TagLocation { lat, lon, .. } => match index {
                0 => Some(lat),
                1 => Some(lon),
                _ => None,
            },
            Modal::Help => None,
        }
    }

    pub fn field_mut(&mut self, index: usize) -> Option<&mut TextBuffer> {
        match self {
            Modal::Rename { name, .. } => (index == 0).then_some(name),
            Modal::ReviewInterval { days, next_review, .. } => match index {
                0 => Some(days),
                1 => Some(next_review),
                _ => None,
            },
            Modal::NewProject {
                name,
                notes,
                review_days,
                defer,
                planned,
                due,
                icon_query,
                ..
            } => match index {
                0 => Some(name),
                1 => Some(notes),
                2 => Some(review_days),
                3 => Some(defer),
                4 => Some(planned),
                5 => Some(due),
                6 => Some(icon_query),
                _ => None,
            },
            Modal::IconPicker { query, .. } => (index == 0).then_some(query),
            Modal::AdjustDates { .. } => None,
            Modal::CustomNotification { at, .. } => (index == 0).then_some(at),
            Modal::RelativeNotification { offset, .. } => (index == 0).then_some(offset),
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. }
            | Modal::DeleteContainers { .. }
            | Modal::DeleteTag { .. } => None,
            Modal::DeleteTask { .. } => None,
            Modal::HideSubtasks { .. } => None,
            Modal::PriorityScope { .. } => None,
            Modal::RemoveAttachment { .. } | Modal::Uninstall { .. } => None,
            Modal::AttachLink { url, name, .. } => match index {
                0 => Some(url),
                1 => Some(name),
                _ => None,
            },
            Modal::NewFolder { name, .. } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => match index {
                0 => Some(&mut form.name),
                1 => Some(&mut form.description),
                2 if form.tab == PerspectiveTab::Simple => Some(&mut form.text),
                3 if form.tab == PerspectiveTab::Simple => Some(&mut form.estimate_min),
                4 if form.tab == PerspectiveTab::Simple => Some(&mut form.estimate_max),
                5 => Some(&mut form.icon_query),
                8 if form.tab == PerspectiveTab::Comprehensive => form.editing_buffer_mut(0),
                9 if form.tab == PerspectiveTab::Comprehensive => form.editing_buffer_mut(1),
                _ => None,
            },
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                changed_days,
                due_soon_days,
                radius,
                trash_days,
                forecast_tag,
                custom_notification_time,
                section,
                ..
            } => match index {
                0 if section.fields().contains(&0) => Some(review_days),
                1 if section.fields().contains(&1) => Some(completed_days),
                2 if section.fields().contains(&2) => Some(changed_days),
                3 if section.fields().contains(&3) => Some(radius),
                4 if section.fields().contains(&4) => Some(trash_days),
                5 if section.fields().contains(&5) => Some(forecast_tag),
                6 if section.fields().contains(&6) => Some(custom_notification_time),
                7 if section.fields().contains(&7) => Some(due_soon_days),
                _ => None,
            },
            Modal::Location { lat, lon } | Modal::TagLocation { lat, lon, .. } => match index {
                0 => Some(lat),
                1 => Some(lon),
                _ => None,
            },
            Modal::Help => None,
        }
    }

    pub fn field_label(&self, index: usize) -> &'static str {
        match self {
            Modal::Rename { .. } => "Name",
            Modal::ReviewInterval { .. } => match index {
                0 => "Every (days)",
                1 => "Next review (YYYY-MM-DD)",
                _ => "",
            },
            Modal::NewProject { .. } => match index {
                0 => "Name",
                1 => "Notes",
                2 => "Review every (days)",
                3 => "Defer date",
                4 => "Planned date",
                5 => "Due date",
                6 => "Search icons",
                _ => "",
            },
            Modal::IconPicker { .. } => "Search icons",
            Modal::AdjustDates { .. } => "",
            Modal::CustomNotification { .. } => "Date and time",
            Modal::RelativeNotification { .. } => "Timing (e.g. -15m)",
            Modal::CreateProject { .. } => "",
            Modal::DeleteProject { .. } => "",
            Modal::DeleteFolder { .. }
            | Modal::DeleteContainers { .. }
            | Modal::DeleteTag { .. } => "",
            Modal::DeleteTask { .. } => "",
            Modal::HideSubtasks { .. } => "",
            Modal::PriorityScope { .. } => "",
            Modal::RemoveAttachment { .. } | Modal::Uninstall { .. } => "",
            Modal::AttachLink { .. } => match index {
                0 => "Address",
                1 => "Name (optional)",
                _ => "",
            },
            Modal::NewFolder { .. } | Modal::NewTag { .. } => "Name",
            Modal::Perspective { .. } => match index {
                0 => "Name",
                1 => "Description",
                2 => "Contains text",
                3 => "Minimum estimate (minutes)",
                4 => "Maximum estimate (minutes)",
                5 => "Search icons",
                8 => "Rule value",
                9 => "Rule second value",
                _ => "",
            },
            Modal::Import { .. } => "Library text",
            Modal::Settings { .. } => match index {
                0 => "Default review days",
                1 => "Completed history (days)",
                2 => "Changed history (days)",
                3 => "Nearby radius (km)",
                4 => "Trash retention (days, 0 keeps forever)",
                5 => "Forecast tag (a tag path, or empty for none)",
                6 => "Custom notification time (HH:MM)",
                7 => "Due soon (days)",
                _ => "",
            },
            Modal::Location { .. } => match index {
                0 => "Latitude",
                1 => "Longitude",
                _ => "",
            },
            Modal::TagLocation { .. } => match index {
                0 => "Latitude",
                1 => "Longitude",
                _ => "",
            },
            Modal::Help => "",
        }
    }

    fn multiline(&self, index: usize) -> bool {
        matches!(self, Modal::Import { .. }) && index == 0
    }

    fn width(&self) -> f32 {
        match self {
            Modal::Import { .. } | Modal::Help => 620.0,
            Modal::DeleteTask { .. } => 500.0,
            Modal::HideSubtasks { .. } => 560.0,
            Modal::PriorityScope { .. } => 520.0,
            Modal::Perspective { .. } => 680.0,
            Modal::IconPicker { .. } => 540.0,
            Modal::AdjustDates { .. } => 560.0,
            Modal::DeleteContainers { .. } => 520.0,
            Modal::CustomNotification { .. } | Modal::RelativeNotification { .. } => 520.0,
            Modal::Settings { .. } => 880.0,
            Modal::Uninstall { .. } => 560.0,
            _ => 460.0,
        }
    }
}

fn view_checkbox(theme: super::theme::Theme, selected: bool) -> gpui::Div {
    div()
        .size(px(13.))
        .rounded_sm()
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(if selected { theme.accent } else { theme.clear })
        .flex()
        .items_center()
        .justify_center()
        .when(selected, |this| {
            this.child(icons::icon(Icon::Check, 9.0, theme.bg))
        })
}

/// Where a picker's selection goes.
#[derive(Clone)]
pub(super) enum IconTarget {
    NewProject,
    Perspective,
    Project(ProjectId),
}

impl FlowlistApp {
    pub(super) fn render_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(modal) = self.modal.as_ref() else {
            return div().into_any_element();
        };
        let title = modal.title().to_string();
        let subtitle = modal.subtitle().to_string();
        let width = modal.width();
        let field_count = modal.field_count();
        let confirm_label = modal.confirm_label();
        let is_help = matches!(modal, Modal::Help);
        let is_perspective = matches!(modal, Modal::Perspective { .. });
        let is_icon_picker = matches!(modal, Modal::IconPicker { .. });
        let is_import = matches!(modal, Modal::Import { .. });
        let is_settings = matches!(modal, Modal::Settings { .. });
        let is_new_project = matches!(modal, Modal::NewProject { .. });
        let is_uninstall = matches!(modal, Modal::Uninstall { .. });
        // Project creation already has Cancel and Create at the bottom, so a
        // second close control in the corner is redundant.
        let shows_close = modal.shows_close();
        let is_tag_location = matches!(modal, Modal::TagLocation { .. });
        let mut header = div().flex().items_start().justify_between().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title),
                )
                .when(!subtitle.is_empty(), |this| {
                    this.child(
                        div()
                            .max_w(px(480.))
                            .text_sm()
                            .text_color(theme.text_faint)
                            .child(subtitle),
                    )
                }),
        );
        if shows_close {
            header = header.child(
                div()
                    .id("modal-close")
                    .cursor_pointer()
                    .text_color(theme.text_faint)
                    .hover(|style| style.text_color(theme.text))
                    .flow_tip(self, cx, theme, "Close (Esc)")
                    .child(icons::icon(icons::Icon::Close, 14.0, theme.text_faint))
                    .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
            );
        }

        // Ordinary modals scroll as one column. Settings is different: its
        // rail must stay put while only the chosen section scrolls, so the
        // sheet owns that scroll region (see render_settings_sheet). While a
        // perspective menu is open, the menu owns scrolling so a wheel over it
        // can never move the sheet behind it.
        let mut body = div()
            .id("modal-body")
            .flex()
            .flex_col()
            .gap_3()
            .flex_1()
            .min_h_0();
        if let Some(error) = &self.modal_error {
            body = body.child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.warning)
                    .bg(theme.field)
                    .text_sm()
                    .text_color(theme.warning)
                    .whitespace_normal()
                    .child(error.clone()),
            );
        }
        if !is_settings {
            body = body
                .pr(px(14.))
                .overflow_y_scroll()
                .track_scroll(&self.modal_scroll);
        }

        if is_help {
            body = body.child(help_body(theme));
        } else {
            // Theme and appearance are the first thing people open Settings
            // for; keep them above the numeric fields so they are visible
            // without scrolling on a normal window.
            if is_settings {
                body = body.child(self.render_settings_sheet(cx));
            }
            if is_uninstall {
                let purge = matches!(modal, Modal::Uninstall { purge: true, .. });
                body = body.child(self.render_uninstall_choices(purge, cx));
            }
            if let Modal::CreateProject { path, .. } = modal {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.field)
                        .text_sm()
                        .text_color(theme.text)
                        .child(path.clone()),
                );
            }
            if let Modal::RemoveAttachment { name, .. } = modal {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.field)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(name.clone()),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(
                            "The file or link itself is left alone. Undo puts the attachment back.",
                        )),
                );
            }
            if let Modal::ReviewInterval { id, name, .. } = modal {
                let next = self
                    .store
                    .db()
                    .projects
                    .get(id)
                    .map(|project| project.next_review.format("%b %-d, %Y").to_string())
                    .unwrap_or_else(|| "—".to_string());
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.field)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(format!("Next review {next}")),
                        ),
                );
            }
            if let Modal::DeleteProject { name, .. } = modal {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(name.clone()),
                );
            }
            if let Modal::AdjustDates { conflicts, .. } = modal {
                let mut list = div().flex().flex_col().gap_2();
                for conflict in conflicts {
                    let mut lines = div().flex().flex_col().gap_0p5();
                    if let Some((from, to)) = conflict.due {
                        lines = lines.child(div().text_xs().text_color(theme.text_faint).child(
                            format!("Due: {} → {}", from.to_input_string(), to.to_input_string()),
                        ));
                    }
                    if let Some((from, to)) = conflict.defer {
                        lines = lines.child(div().text_xs().text_color(theme.text_faint).child(
                            format!(
                                "Defer: {} → {}",
                                from.to_input_string(),
                                to.to_input_string()
                            ),
                        ));
                    }
                    list = list.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.field)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text)
                                    .child(conflict.title.clone()),
                            )
                            .child(lines),
                    );
                }
                body = body.child(list);
            }
            if let Modal::DeleteContainers { projects, folders } = modal {
                let db = self.store.db();
                let mut names = Vec::new();
                for id in projects.iter().take(8) {
                    if let Some(project) = db.projects.get(id) {
                        names.push(format!("Project · {}", db.project_path(&project.id)));
                    }
                }
                for id in folders.iter().take(8) {
                    if let Some(folder) = db.folders.get(id) {
                        names.push(format!("Folder · {}", db.folder_path(&folder.id)));
                    }
                }
                let more = projects.len() + folders.len();
                body =
                    body.child(
                        div()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.danger)
                            .bg(theme.field)
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_sm()
                            .text_color(theme.text)
                            .child(format!("{} selected", super::plural(more, "item", "items")))
                            .children(names.into_iter().map(|name| {
                                div().text_xs().text_color(theme.text_faint).child(name)
                            })),
                    );
            }
            if let Modal::DeleteTag { name, .. } = modal {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(format!("#{name}")),
                );
            }
            if let Modal::DeleteFolder { id, name } = modal {
                // Name the folder and say how much goes with it, so the
                // cascade is never a surprise.
                let projects = self.store.db().project_count(id);
                let detail = match projects {
                    0 => "It has no projects inside.".to_string(),
                    1 => "Its 1 project, and every task in it, moves to Trash too.".to_string(),
                    count => {
                        format!("Its {count} projects, and every task in them, move to Trash too.")
                    }
                };
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(name.clone()),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(detail)),
                );
            }
            if let Modal::DeleteTask {
                title,
                occurrences,
                whole,
                ..
            } = modal
            {
                let title = title.clone();
                let whole = *whole;
                let occurrences = *occurrences;
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(title),
                );
                // Two chips behave like a segmented control. Tab and the arrow
                // keys flip them, so the choice never needs the mouse.
                let chip = |label: &'static str, active: bool, key: &'static str| {
                    div().id(key).child(
                        div()
                            .on_children_prepainted(self.view_probe(key.to_string()))
                            .child(widgets::toggle_chip(theme, label, active)),
                    )
                };
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            chip("This occurrence", !whole, "delete-occurrence").on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::DeleteTask { whole, .. }) = &mut this.modal {
                                        *whole = false;
                                    }
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            chip("Whole series", whole, "delete-series").on_click(cx.listener(
                                |this, _, _, cx| {
                                    if let Some(Modal::DeleteTask { whole, .. }) = &mut this.modal {
                                        *whole = true;
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                );
                body = body.child(div().text_xs().text_color(theme.text_faint).child(
                    match occurrences {
                        1 => "This is the only occurrence in the series.".to_string(),
                        count => {
                            format!("The series has {count} occurrences, including this one.")
                        }
                    },
                ));
            }
            if let Modal::HideSubtasks { title, count, .. } = modal {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.warning)
                        .bg(theme.field)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(title.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(if *count == 1 {
                                    "1 subtask becomes a one-off step. Notes, dates, tags and priority are removed; the title and completion state stay.".to_string()
                                } else {
                                    format!(
                                        "{count} subtasks become one-off steps. Notes, dates, tags and priority are removed; titles and completion states stay."
                                    )
                                }),
                        ),
                );
            }
            if let Modal::PriorityScope {
                priority,
                occurrences,
                all,
                ..
            } = modal
            {
                let priority = *priority;
                let all = *all;
                let occurrences = *occurrences;
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.field)
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(match priority {
                            Some(priority) => format!("{} priority", priority.label()),
                            None => "No priority".to_string(),
                        }),
                );
                let chip = |label: &'static str, active: bool, key: &'static str| {
                    div().id(key).child(
                        div()
                            .on_children_prepainted(self.view_probe(key.to_string()))
                            .child(widgets::toggle_chip(theme, label, active)),
                    )
                };
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            chip("This occurrence", !all, "priority-occurrence").on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::PriorityScope { all, .. }) = &mut this.modal
                                    {
                                        *all = false;
                                    }
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            chip("Whole series", all, "priority-series").on_click(cx.listener(
                                |this, _, _, cx| {
                                    if let Some(Modal::PriorityScope { all, .. }) = &mut this.modal
                                    {
                                        *all = true;
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                );
                body = body.child(div().text_xs().text_color(theme.text_faint).child(
                    match occurrences {
                        1 => "This is the only occurrence in the series.".to_string(),
                        count => {
                            format!("The series has {count} occurrences, including this one.")
                        }
                    },
                ));
            }
            if !is_settings {
                if is_new_project {
                    body = body.child(self.render_new_project_icon(cx));
                }
                if is_perspective {
                    body = body.child(self.render_modal_field(0, cx));
                    body = body.child(self.render_modal_field(1, cx));
                    if matches!(
                        &self.modal,
                        Some(Modal::Perspective { form }) if form.tab == PerspectiveTab::Simple
                    ) {
                        body = body.child(self.render_modal_field(2, cx));
                        body = body.child(
                            div()
                                .flex()
                                .items_start()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(self.render_modal_field(3, cx)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(self.render_modal_field(4, cx)),
                                ),
                        );
                    }
                } else if is_new_project {
                    for index in 0..6 {
                        body = body.child(self.render_modal_field(index, cx));
                    }
                } else if is_icon_picker {
                    // The picker card renders its own search field.
                } else {
                    for index in 0..field_count {
                        body = body.child(self.render_modal_field(index, cx));
                    }
                }
            }
            if let Modal::RelativeNotification { event, .. } = modal {
                let current = *event;
                let chip = |label: &'static str, active: bool, key: &'static str| {
                    div()
                        .on_children_prepainted(self.view_probe(key.to_string()))
                        .id(key)
                        .child(widgets::toggle_chip(theme, label, active))
                };
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            chip("Deferred", current == AlertEvent::Defer, "relative-event:defer")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::RelativeNotification { event, .. }) =
                                        &mut this.modal
                                    {
                                        *event = AlertEvent::Defer;
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            chip("Planned", current == AlertEvent::Planned, "relative-event:planned")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::RelativeNotification { event, .. }) =
                                        &mut this.modal
                                    {
                                        *event = AlertEvent::Planned;
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            chip("Due", current == AlertEvent::Due, "relative-event:due").on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::RelativeNotification { event, .. }) =
                                        &mut this.modal
                                    {
                                        *event = AlertEvent::Due;
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                );
            }
            if is_icon_picker {
                if let Modal::IconPicker { project, tab, .. } = modal {
                    let current = self
                        .store
                        .db()
                        .projects
                        .get(project)
                        .map(|project| project.icon.clone());
                    body = body.child(self.render_icon_picker_card(
                        IconTarget::Project(project.clone()),
                        0,
                        current.as_ref(),
                        *tab,
                        cx,
                    ));
                }
            }
            if is_new_project {
                body = body.child(self.render_new_project_controls(cx));
            }
            if matches!(modal, Modal::NewFolder { .. }) {
                body = body.child(self.render_new_folder_controls(cx));
            }
            if is_perspective {
                body = body.child(self.render_perspective_controls(cx));
            }
            if is_import {
                body = body.child(self.render_import_controls(cx));
            }
            if is_tag_location {
                body = body.child(self.render_tag_location_controls(cx));
            }
        }

        let mut body = div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(body);
        if !is_settings {
            body = body.child(self.modal_scrollbar(theme, cx));
        }
        // The settings sheet carries the theme grid, the accent picker and the
        // window chrome row. Size it to the window so a tall display shows
        // everything at once while a short one falls back to scrolling.
        let viewport = window.viewport_size();
        let viewport_width = f32::from(viewport.width);
        let viewport_height = f32::from(viewport.height);
        let card_max_height = if is_settings {
            (viewport_height - 120.0).clamp(480.0, 1040.0)
        } else if is_perspective {
            780.0
        } else {
            640.0
        };
        let settings_max_width = (viewport_width - 40.0).max(640.0);
        let settings_max_height = (viewport_height - 80.0).max(420.0);
        let (settings_width, settings_height) = self
            .settings_size
            .map(|(width, height)| {
                (
                    width.clamp(640.0, settings_max_width),
                    height.clamp(420.0, settings_max_height),
                )
            })
            .unwrap_or((width, card_max_height.min(settings_max_height)));
        let perspective_max_width = (viewport_width - 40.0).max(640.0);
        let perspective_max_height = (viewport_height - 80.0).max(420.0);
        let (perspective_width, perspective_height) = self
            .perspective_size
            .map(|(width, height)| {
                (
                    width.clamp(640.0, perspective_max_width),
                    height.clamp(420.0, perspective_max_height),
                )
            })
            .unwrap_or((760.0_f32.max(width), card_max_height.min(perspective_max_height)));
        let card_width = if is_settings {
            settings_width
        } else if is_perspective {
            perspective_width
        } else {
            width
        };
        let mut card = div()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(card_width))
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_2xl()
            .overflow_hidden()
            .child(header)
            .child(body);
        // Settings keeps one size across sections. Otherwise the sheet grows
        // and shrinks as you switch panes, which is disorienting.
        card = if is_settings {
            card.relative()
                .h(px(settings_height))
                .on_drag_move::<DragSettingsResize>(cx.listener(
                    |this, event: &gpui::DragMoveEvent<DragSettingsResize>, window, cx| {
                        let viewport = window.viewport_size();
                        let center_x = f32::from(viewport.width) / 2.0;
                        let center_y = f32::from(viewport.height) / 2.0;
                        let max_width = (f32::from(viewport.width) - 40.0).max(640.0);
                        let max_height = (f32::from(viewport.height) - 80.0).max(420.0);
                        let width = 2.0 * (f32::from(event.event.position.x) - center_x);
                        let height = 2.0 * (f32::from(event.event.position.y) - center_y);
                        this.settings_size = Some((
                            width.clamp(640.0, max_width),
                            height.clamp(420.0, max_height),
                        ));
                        cx.notify();
                    },
                ))
                .on_drag_move::<DragSettingsScroll>(cx.listener(
                    |this, event: &gpui::DragMoveEvent<DragSettingsScroll>, _, cx| {
                        let Some(drag_offset) = this.scroll_drag_grab.get() else {
                            return;
                        };
                        let bounds = this.modal_scroll.bounds();
                        let viewport = f32::from(bounds.size.height);
                        let max_offset = f32::from(this.modal_scroll.max_offset().height);
                        if viewport <= 0.0 || max_offset <= 0.0 {
                            return;
                        }
                        let content = viewport + max_offset;
                        let thumb_height = (viewport * viewport / content)
                            .clamp(SETTINGS_SCROLL_MIN_THUMB, viewport);
                        let travel = (viewport - thumb_height).max(1.0);
                        let pointer = f32::from(event.event.position.y - bounds.origin.y);
                        let thumb_top = (pointer - drag_offset).clamp(0.0, travel);
                        let scroll = (thumb_top / travel) * max_offset;
                        this.modal_scroll.set_offset(point(px(0.), px(-scroll)));
                        cx.notify();
                    },
                ))
        } else if is_perspective {
            card.relative()
                .h(px(perspective_height))
                .on_drag_move::<DragPerspectiveResize>(cx.listener(
                    |this, event: &gpui::DragMoveEvent<DragPerspectiveResize>, window, cx| {
                        let viewport = window.viewport_size();
                        let center_x = f32::from(viewport.width) / 2.0;
                        let center_y = f32::from(viewport.height) / 2.0;
                        let max_width = (f32::from(viewport.width) - 40.0).max(640.0);
                        let max_height = (f32::from(viewport.height) - 80.0).max(420.0);
                        let width = 2.0 * (f32::from(event.event.position.x) - center_x);
                        let height = 2.0 * (f32::from(event.event.position.y) - center_y);
                        this.perspective_size = Some((
                            width.clamp(640.0, max_width),
                            height.clamp(420.0, max_height),
                        ));
                        cx.notify();
                    },
                ))
        } else {
            card.max_h(px(card_max_height))
        };

        if !is_help {
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .pt_1()
                    .child(
                        div()
                            .on_children_prepainted(self.view_probe("modal-cancel".to_string()))
                            .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("modal-cancel".to_string()),
                            )
                            .id("modal-cancel")
                            .child(widgets::button(theme, "Cancel", false))
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                            ),
                    )
                    .child(
                        div()
                            .on_children_prepainted(self.view_probe("modal-save".to_string()))
                            .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("modal-save".to_string()),
                            )
                            .id("modal-save")
                            .child(widgets::button(theme, confirm_label, true))
                            .on_click(cx.listener(|this, _, _, cx| this.commit_modal(cx))),
                            ),
                    ),
            );
        }

        if is_settings {
            card = card.child(self.settings_resize_grip(theme, cx));
        } else if is_perspective {
            card = card.child(self.perspective_resize_grip(theme, cx));
        }

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // The dimmed backdrop must swallow clicks meant for the window
            // underneath, not select a row or chip through the sheet.
            .occlude()
            .bg(rgba(0x00000055))
            .child(super::fade_in(
                "modal-fade",
                div()
                    .flex()
                    .flex_col()
                    .max_h(px(if is_settings {
                        settings_max_height + 60.0
                    } else if is_perspective {
                        perspective_max_height + 60.0
                    } else {
                        720.0
                    }))
                    .child(card),
            ))
            .into_any_element()
    }

    fn settings_resize_grip(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let dot = |bottom: f32, right: f32| {
            div()
                .absolute()
                .bottom(px(bottom))
                .right(px(right))
                .size(px(2.5))
                .rounded_full()
                .bg(theme.text_faint)
        };
        let boxes = self.settings_toggle_boxes.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        "settings-resize".to_string(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(
                div()
                    .relative()
                    .size(px(10.))
                    .child(dot(0., 0.))
                    .child(dot(0., 5.))
                    .child(dot(5., 0.)),
            )
            .id("settings-resize")
            .absolute()
            .bottom(px(2.))
            .right(px(2.))
            .size(px(16.))
            .rounded_sm()
            .cursor(CursorStyle::ResizeUpLeftDownRight)
            .hover(|style| style.bg(theme.hover))
            .flow_tip(self, cx, theme, "Drag to resize Settings")
            .on_drag(DragSettingsResize, settings_resize_ghost)
            .into_any_element()
    }

    /// A Zed-style thin scrollbar: a six-point rounded thumb with four points
    /// of padding, a quiet track that only tints on hover, and exact tracking
    /// while dragging. It appears only when the body overflows.
    fn perspective_resize_grip(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let dot = |bottom: f32, right: f32| {
            div()
                .absolute()
                .bottom(px(bottom))
                .right(px(right))
                .size(px(2.5))
                .rounded_full()
                .bg(theme.text_faint)
        };
        div()
            .on_children_prepainted(self.view_probe("perspective-resize".to_string()))
            .child(
                div()
                    .relative()
                    .size(px(10.))
                    .child(dot(0., 0.))
                    .child(dot(0., 5.))
                    .child(dot(5., 0.)),
            )
            .id("perspective-resize")
            .absolute()
            .bottom(px(2.))
            .right(px(2.))
            .size(px(16.))
            .rounded_sm()
            .cursor(CursorStyle::ResizeUpLeftDownRight)
            .hover(|style| style.bg(theme.hover))
            .flow_tip(self, cx, theme, "Drag to resize the view editor")
            .on_drag(DragPerspectiveResize, |_, position, window, cx| {
                settings_resize_ghost(&DragSettingsResize, position, window, cx)
            })
            .into_any_element()
    }

    fn modal_scrollbar(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        const THUMB_WIDTH: f32 = 6.0;
        const PADDING: f32 = 4.0;
        const TRACK_WIDTH: f32 = THUMB_WIDTH + 2.0 * PADDING;

        let handle = &self.modal_scroll;
        let bounds = handle.bounds();
        let viewport = f32::from(bounds.size.height);
        let max_offset = f32::from(handle.max_offset().height);
        if viewport <= 0.0 || max_offset <= 1.0 {
            return div().into_any_element();
        }
        let scrolled = (-f32::from(handle.offset().y)).clamp(0.0, max_offset);
        let content = viewport + max_offset;
        let thumb_height =
            (viewport * viewport / content).clamp(SETTINGS_SCROLL_MIN_THUMB, viewport);
        let travel = (viewport - thumb_height).max(1.0);
        let thumb_top = (scrolled / max_offset) * travel;
        let thumb_top_window = f32::from(bounds.origin.y) + thumb_top;

        let boxes = self.settings_toggle_boxes.clone();
        let grab = self.scroll_drag_grab.clone();
        let thumb = div()
            .id("modal-scroll-thumb")
            .absolute()
            .top(px(thumb_top))
            .right(px(PADDING))
            .w(px(THUMB_WIDTH))
            .h(px(thumb_height))
            .rounded_full()
            .bg(theme.border)
            .hover(|style| style.bg(theme.text_faint))
            .on_drag(DragSettingsScroll, move |payload, position, window, cx| {
                let pointer = f32::from(position.y);
                let drag_offset = (pointer - thumb_top_window).clamp(0.0, thumb_height);
                grab.set(Some(drag_offset));
                settings_scroll_ghost(payload, position, window, cx)
            });
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        "settings-scrollbar".to_string(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .id("modal-scroll-track")
            .absolute()
            .top_0()
            .right_0()
            .h_full()
            .w(px(TRACK_WIDTH))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let pointer = f32::from(event.position.y);
                    // A click on the thumb belongs to the drag handler; a
                    // click on the rest of the track jumps the thumb there.
                    if pointer >= thumb_top_window && pointer <= thumb_top_window + thumb_height {
                        return;
                    }
                    let bounds = this.modal_scroll.bounds();
                    let viewport = f32::from(bounds.size.height);
                    let max_offset = f32::from(this.modal_scroll.max_offset().height);
                    if viewport <= 0.0 || max_offset <= 0.0 {
                        return;
                    }
                    let content = viewport + max_offset;
                    let thumb_height =
                        (viewport * viewport / content).clamp(SETTINGS_SCROLL_MIN_THUMB, viewport);
                    let travel = (viewport - thumb_height).max(1.0);
                    let pointer = f32::from(event.position.y - bounds.origin.y);
                    let thumb_top = (pointer - thumb_height / 2.0).clamp(0.0, travel);
                    let scroll = (thumb_top / travel) * max_offset;
                    this.modal_scroll.set_offset(point(px(0.), px(-scroll)));
                    cx.notify();
                }),
            )
            .child(thumb)
            .into_any_element()
    }

    fn render_uninstall_choices(&self, purge: bool, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let library = self.store.paths().root.display().to_string();
        let choice = |key: &'static str,
                      selected: bool,
                      title: &'static str,
                      detail: String,
                      danger: bool,
                      cx: &mut Context<Self>| {
            div()
                .id(key)
                .flex()
                .items_start()
                .gap_2()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(if selected {
                    if danger { theme.danger } else { theme.accent }
                } else {
                    theme.border
                })
                .bg(if selected { theme.field } else { theme.clear })
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(icons::icon(
                    if selected {
                        Icon::Check
                    } else {
                        Icon::CircleDashed
                    },
                    14.0,
                    if danger { theme.danger } else { theme.accent },
                ))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(if danger { theme.danger } else { theme.text })
                                .child(title),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(detail)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(Modal::Uninstall { purge: current }) = &mut this.modal {
                        *current = matches!(key, "uninstall-purge");
                    }
                    cx.notify();
                }))
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.field)
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(
                        "Flowlist will quit, remove the app and the flowlist command, then delete the files below. This cannot be undone.",
                    ),
            )
            .child(choice(
                "uninstall-keep",
                !purge,
                "Keep my library",
                format!("Tasks, history and backups stay at {library}."),
                false,
                cx,
            ))
            .child(choice(
                "uninstall-purge",
                purge,
                "Delete the app and all data",
                format!("Removes the library, history and backups at {library}."),
                true,
                cx,
            ))
            .into_any_element()
    }

    fn render_modal_field(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(modal) = self.modal.as_ref() else {
            return div().into_any_element();
        };
        let Some(buffer) = modal.field(index) else {
            return div().into_any_element();
        };
        let label = modal.field_label(index).to_string();
        let focused = self.modal_focus == index;
        let multiline = modal.multiline(index);
        let field = if multiline {
            widgets::text_area_with_cursor(
                theme,
                buffer,
                "Paste JSON or TaskPaper here",
                focused,
                180.0,
                cx.listener(move |this, hit: &widgets::TextHit, _, cx| {
                    this.set_modal_cursor(index, hit.index, hit.extend, hit.click_count, cx);
                }),
            )
            .into_any_element()
        } else {
            widgets::text_field_with_cursor(
                theme,
                buffer,
                "",
                focused,
                cx.listener(move |this, hit: &widgets::TextHit, _, cx| {
                    this.set_modal_cursor(index, hit.index, hit.extend, hit.click_count, cx);
                }),
            )
            .into_any_element()
        };
        let anchor = self.modal_field_anchor.clone();
        div()
            .on_children_prepainted(self.view_probe(format!("modal-field:{index}")))
            .id(super::sid(format!("modal-field:{index}")))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            anchor.set(Some(*bounds));
                        }
                    })
                    .child(field),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.modal_focus = index;
                this.focus = super::FocusArea::Modal;
                cx.notify();
            }))
            .into_any_element()
    }

    /// A searchable card of named icons and emoji. The search field is a
    /// normal modal field so typing, selection and tab order keep working.
    fn render_icon_picker_card(
        &self,
        target: IconTarget,
        search_index: usize,
        current: Option<&IconSpec>,
        tab: IconTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let raw_query = self
            .modal
            .as_ref()
            .and_then(|modal| modal.field(search_index))
            .map(|buffer| buffer.text().trim().to_string())
            .unwrap_or_default();
        let query = raw_query.to_lowercase();

        let mut grid = div().flex().flex_wrap().items_center().gap_1();
        let mut count = 0;
        match tab {
            IconTab::Icons => {
                for candidate in ProjectIcon::ALL {
                    if !query.is_empty()
                        && !candidate.slug().contains(&query)
                        && !candidate.label().to_lowercase().contains(&query)
                    {
                        continue;
                    }
                    count += 1;
                    let active = current == Some(&IconSpec::builtin(candidate));
                    grid = grid.child(
                        self.icon_choice_cell(
                            format!("picker-icon:{}", candidate.slug()),
                            active,
                            widgets::icon_for_project(candidate, 14.0, theme.text_dim)
                                .into_any_element(),
                            candidate.label(),
                            target.clone(),
                            IconSpec::builtin(candidate),
                            cx,
                        ),
                    );
                }
            }
            IconTab::Emoji => {
                if flowlist_core::model::icon::is_single_emoji(&raw_query)
                    && !super::emoji::ALL
                        .iter()
                        .any(|candidate| candidate.glyph == raw_query)
                {
                    count += 1;
                    let active = current == Some(&IconSpec::Emoji(raw_query.clone()));
                    grid = grid.child(
                        self.icon_choice_cell(
                            format!("picker-emoji:{}", raw_query),
                            active,
                            div()
                                .text_size(px(17.))
                                .child(raw_query.clone())
                                .into_any_element(),
                            "Use this emoji",
                            target.clone(),
                            IconSpec::Emoji(raw_query.clone()),
                            cx,
                        ),
                    );
                }
                for item in super::emoji::ALL {
                    if query.is_empty() && count >= 48 {
                        break;
                    }
                    if !query.is_empty()
                        && item.glyph != raw_query
                        && !item.name.to_lowercase().contains(&query)
                        && !item.keywords.contains(&query)
                    {
                        continue;
                    }
                    count += 1;
                    let spec = IconSpec::Emoji(item.glyph.to_string());
                    let active = current == Some(&spec);
                    grid = grid.child(
                        self.icon_choice_cell(
                            format!("picker-emoji:{}", item.glyph),
                            active,
                            div()
                                .text_size(px(17.))
                                .child(item.glyph)
                                .into_any_element(),
                            item.name,
                            target.clone(),
                            spec,
                            cx,
                        ),
                    );
                }
            }
        }

        let icons_target = target.clone();
        let emoji_target = target.clone();
        let icons_tab = div()
            .on_children_prepainted(self.theme_probe("picker-tab-icons".to_string()))
            .id("picker-tab-icons")
            .child(widgets::toggle_chip(theme, "Icons", tab == IconTab::Icons))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_icon_tab(icons_target.clone(), IconTab::Icons, cx);
            }));
        let emoji_tab = div()
            .on_children_prepainted(self.theme_probe("picker-tab-emoji".to_string()))
            .id("picker-tab-emoji")
            .child(widgets::toggle_chip(theme, "Emoji", tab == IconTab::Emoji))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_icon_tab(emoji_target.clone(), IconTab::Emoji, cx);
            }));

        let mut results = div().flex().flex_col().gap_2();
        if count == 0 {
            results = results.child(div().text_sm().text_color(theme.text_faint).child(
                if query.is_empty() {
                    "Nothing here yet."
                } else {
                    "No matches in this tab."
                },
            ));
        } else {
            results = results.child(grid);
            if tab == IconTab::Emoji && query.is_empty() && super::emoji::ALL.len() > 48 {
                results = results.child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("Type to search all emoji."),
                );
            }
        }

        let default_active = match &target {
            IconTarget::Perspective => current.is_none(),
            _ => current == Some(&IconSpec::default()),
        };
        let default_target = target.clone();
        div()
            .on_children_prepainted(self.theme_probe("icon-picker-card".to_string()))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field)
            .child(self.render_modal_field(search_index, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(icons_tab)
                    .child(emoji_tab),
            )
            .child(
                div()
                    .id("icon-picker-default")
                    .child(widgets::toggle_chip(theme, "Default", default_active))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.choose_default_icon(default_target.clone(), cx);
                    })),
            )
            .child(results)
            .into_any_element()
    }

    fn icon_choice_cell(
        &self,
        id: String,
        active: bool,
        glyph: AnyElement,
        label: &'static str,
        target: IconTarget,
        spec: IconSpec,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .on_children_prepainted(self.theme_probe(id.clone()))
            .id(super::sid(id))
            .size(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(9.))
            .border_1()
            .border_color(if active {
                theme.accent
            } else {
                theme.border_soft
            })
            .bg(if active { theme.selection } else { theme.clear })
            .cursor_pointer()
            .flow_tip(self, cx, theme, label.to_string())
            .hover(|style| style.bg(theme.hover))
            .child(glyph)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.choose_icon(target.clone(), spec.clone(), cx);
            }))
            .into_any_element()
    }

    pub(super) fn choose_icon(
        &mut self,
        target: IconTarget,
        spec: IconSpec,
        cx: &mut Context<Self>,
    ) {
        match target {
            IconTarget::NewProject => {
                if let Some(Modal::NewProject {
                    icon,
                    icon_open,
                    icon_query,
                    ..
                }) = &mut self.modal
                {
                    *icon = spec;
                    *icon_open = false;
                    icon_query.set(String::new());
                }
            }
            IconTarget::Perspective => {
                if let Some(Modal::Perspective { form }) = &mut self.modal {
                    form.icon = Some(spec);
                    form.icon_open = false;
                    form.icon_query.set(String::new());
                }
            }
            IconTarget::Project(id) => {
                self.execute(flowlist_core::command::Command::UpdateProject {
                    id,
                    patch: flowlist_core::command::ProjectPatch {
                        icon: Some(spec),
                        ..Default::default()
                    },
                });
                self.close_modal(cx);
            }
        }
        cx.notify();
    }

    fn set_icon_tab(&mut self, target: IconTarget, tab: IconTab, cx: &mut Context<Self>) {
        match target {
            IconTarget::Perspective => {
                if let Some(Modal::Perspective { form }) = &mut self.modal {
                    form.icon_tab = tab;
                }
            }
            IconTarget::NewProject => {
                if let Some(Modal::NewProject { icon_tab, .. }) = &mut self.modal {
                    *icon_tab = tab;
                }
            }
            IconTarget::Project(_) => {
                if let Some(Modal::IconPicker { tab: current, .. }) = &mut self.modal {
                    *current = tab;
                }
            }
        }
        cx.notify();
    }

    fn choose_default_icon(&mut self, target: IconTarget, cx: &mut Context<Self>) {
        match target {
            IconTarget::Perspective => {
                if let Some(Modal::Perspective { form }) = &mut self.modal {
                    form.icon = None;
                    form.icon_open = false;
                    form.icon_query.set(String::new());
                }
                cx.notify();
            }
            _ => self.choose_icon(target, IconSpec::default(), cx),
        }
    }

    /// If the icon card is open, where a selection goes and which field holds
    /// its search text.
    pub(super) fn icon_picker_target(&self) -> Option<(IconTarget, usize)> {
        match &self.modal {
            Some(Modal::Perspective { form }) if form.icon_open => {
                Some((IconTarget::Perspective, 5))
            }
            Some(Modal::NewProject {
                icon_open: true, ..
            }) => Some((IconTarget::NewProject, 6)),
            Some(Modal::IconPicker { project, .. }) => {
                Some((IconTarget::Project(project.clone()), 0))
            }
            _ => None,
        }
    }

    /// The first search result, used by Return without a pointer.
    pub(super) fn first_icon_match(&self, search_index: usize) -> Option<IconSpec> {
        let raw_query = self
            .modal
            .as_ref()
            .and_then(|modal| modal.field(search_index))
            .map(|buffer| buffer.text().trim().to_string())
            .unwrap_or_default();
        if raw_query.is_empty() {
            return None;
        }
        let query = raw_query.to_lowercase();
        for candidate in ProjectIcon::ALL {
            if candidate.slug().contains(&query)
                || candidate.label().to_lowercase().contains(&query)
            {
                return Some(IconSpec::builtin(candidate));
            }
        }
        if flowlist_core::model::icon::is_single_emoji(&raw_query) {
            return Some(IconSpec::Emoji(raw_query));
        }
        for item in super::emoji::ALL {
            if item.name.to_lowercase().contains(&query) || item.keywords.contains(&query) {
                return Some(IconSpec::Emoji(item.glyph.to_string()));
            }
        }
        None
    }

    pub(super) fn close_icon_picker(&mut self, cx: &mut Context<Self>) {
        match &mut self.modal {
            Some(Modal::Perspective { form }) => form.icon_open = false,
            Some(Modal::NewProject { icon_open, .. }) => *icon_open = false,
            Some(Modal::IconPicker { .. }) => {
                self.close_modal(cx);
                return;
            }
            _ => {}
        }
        cx.notify();
    }

    /// The perspective editor is a small settings sheet rather than a row of
    /// cycling chips: every condition has its own labelled row and opens a menu
    /// of every value, so choosing one is a jump instead of a lap.
    /// Record where a control painted so a GUI probe can click it without
    /// guessing at pixels.
    pub(super) fn view_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.view_boxes.clone();
        move |bounds, _, _| {
            let Some(bounds) = bounds.first() else {
                return;
            };
            boxes.borrow_mut().insert(
                key.clone(),
                [
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ],
            );
        }
    }

    /// One labelled menu button in the perspective editor. The value reads like
    /// a field and the chevron says it opens a list rather than cycling.
    fn perspective_select(
        &self,
        field: ViewField,
        value: String,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let key = field.key().to_string();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(field.label()),
            )
            .child(
                div().on_children_prepainted(self.view_probe(key)).child(
                    div()
                        .id(super::sid(format!("modal-{}", field.key())))
                        .h(px(28.))
                        .px_2()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .rounded_md()
                        .border_1()
                        .border_color(if active { theme.accent } else { theme.border })
                        .bg(if active { theme.selection } else { theme.field })
                        .text_sm()
                        .text_color(if active { theme.accent } else { theme.text })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(value),
                        )
                        .child(icons::icon(
                            Icon::ChevronDown,
                            11.0,
                            if active {
                                theme.accent
                            } else {
                                theme.text_faint
                            },
                        ))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.toggle_view_menu(field, cx)),
                        ),
                ),
            )
    }

    fn presentation_select(
        &self,
        field: ViewField,
        label: &'static str,
        value: String,
        active: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let key = field.key().to_string();
        div()
            .flex()
            .items_center()
            .gap_3()
            .min_w_0()
            .child(
                div()
                    .w(px(150.))
                    .text_sm()
                    .text_color(theme.text)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .on_children_prepainted(self.view_probe(key))
                    .child(
                        div()
                            .id(super::sid(format!("modal-{}", field.key())))
                            .h(px(28.))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(if active { theme.accent } else { theme.border })
                            .bg(if active { theme.selection } else { theme.field })
                            .text_sm()
                            .text_color(if active { theme.accent } else { theme.text })
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(value),
                            )
                            .child(icons::icon(
                                Icon::ChevronDown,
                                11.0,
                                if active { theme.accent } else { theme.text_faint },
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_view_menu(field, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn view_choices(&self, field: ViewField) -> Vec<ViewChoice> {
        let db = self.store.db();
        match field {
            ViewField::Project => {
                let mut projects: Vec<_> = db.projects.values().collect();
                projects.sort_by_cached_key(|project| project.name.to_lowercase());
                let mut choices = vec![ViewChoice::Project(None)];
                choices.extend(
                    projects
                        .into_iter()
                        .map(|project| ViewChoice::Project(Some(project.id.clone()))),
                );
                choices
            }
            ViewField::Tag => {
                let mut tags: Vec<_> = db.tags.values().collect();
                tags.sort_by_cached_key(|tag| tag.name.to_lowercase());
                let mut choices = vec![ViewChoice::Tag(None)];
                choices.extend(
                    tags.into_iter()
                        .map(|tag| ViewChoice::Tag(Some(tag.id.clone()))),
                );
                choices
            }
            ViewField::Status => [
                TaskStatus::Any,
                TaskStatus::Available,
                TaskStatus::Blocked,
                TaskStatus::Deferred,
                TaskStatus::Completed,
            ]
            .into_iter()
            .map(ViewChoice::Status)
            .collect(),
            ViewField::Due => [
                super::perspective::DueChoice::Any,
                super::perspective::DueChoice::Overdue,
                super::perspective::DueChoice::Today,
                super::perspective::DueChoice::Tomorrow,
                super::perspective::DueChoice::Next7Days,
                super::perspective::DueChoice::Next30Days,
                super::perspective::DueChoice::Later,
                super::perspective::DueChoice::NoDate,
            ]
            .into_iter()
            .map(ViewChoice::Due)
            .collect(),
            ViewField::Planned => [
                super::perspective::DueChoice::Any,
                super::perspective::DueChoice::Overdue,
                super::perspective::DueChoice::Today,
                super::perspective::DueChoice::Tomorrow,
                super::perspective::DueChoice::Next7Days,
                super::perspective::DueChoice::Next30Days,
                super::perspective::DueChoice::Later,
                super::perspective::DueChoice::NoDate,
            ]
            .into_iter()
            .map(ViewChoice::Planned)
            .collect(),
            ViewField::Fullscreen => [None, Some(true), Some(false)]
                .into_iter()
                .map(ViewChoice::Fullscreen)
                .collect(),
            ViewField::Defer => [
                super::perspective::DeferChoice::Any,
                super::perspective::DeferChoice::Today,
                super::perspective::DeferChoice::Tomorrow,
                super::perspective::DeferChoice::Next7Days,
                super::perspective::DeferChoice::Next30Days,
                super::perspective::DeferChoice::NoDate,
            ]
            .into_iter()
            .map(ViewChoice::Defer)
            .collect(),
            ViewField::Group => [
                GroupBy::None,
                GroupBy::Project,
                GroupBy::Folder,
                GroupBy::Tag,
                GroupBy::DueDate,
                GroupBy::DeferDate,
                GroupBy::PlannedDate,
                GroupBy::CompletedDate,
                GroupBy::DroppedDate,
                GroupBy::AddedDate,
                GroupBy::ChangedDate,
                GroupBy::NextReview,
                GroupBy::TagsCombined,
                GroupBy::Flagged,
                GroupBy::Estimate,
            ]
            .into_iter()
            .map(ViewChoice::Group)
            .collect(),
            ViewField::Sort => [
                SortBy::Manual,
                SortBy::Due,
                SortBy::Defer,
                SortBy::Planned,
                SortBy::Name,
                SortBy::Priority,
                SortBy::Flagged,
                SortBy::Estimate,
                SortBy::Created,
                SortBy::CompletedAt,
                SortBy::DueFlagged,
                SortBy::Tag,
                SortBy::AddedDate,
                SortBy::ChangedDate,
                SortBy::DroppedDate,
                SortBy::NextReview,
            ]
            .into_iter()
            .map(ViewChoice::Sort)
            .collect(),
        }
    }

    fn view_choice_label(&self, choice: &ViewChoice) -> String {
        match choice {
            ViewChoice::Project(None) => "Any".to_string(),
            ViewChoice::Project(Some(id)) => self.store.db().project_path(id),
            ViewChoice::Tag(None) => "Any".to_string(),
            ViewChoice::Tag(Some(id)) => self
                .store
                .db()
                .tags
                .get(id)
                .map(|tag| tag.name.clone())
                .unwrap_or_else(|| "Unknown tag".to_string()),
            ViewChoice::Status(status) => match status {
                TaskStatus::Any => "Any",
                TaskStatus::Available => "Available",
                TaskStatus::Blocked => "Blocked",
                TaskStatus::Deferred => "Deferred",
                TaskStatus::Completed => "Completed",
            }
            .to_string(),
            ViewChoice::Due(choice) => choice.label().to_string(),
            ViewChoice::Planned(choice) => choice.label().to_string(),
            ViewChoice::Defer(choice) => choice.label().to_string(),
            ViewChoice::Fullscreen(None) => "Any".to_string(),
            ViewChoice::Fullscreen(Some(true)) => "Has full-screen reminder".to_string(),
            ViewChoice::Fullscreen(Some(false)) => "No full-screen reminder".to_string(),
            ViewChoice::Group(group) => super::group_label(Some(*group)).to_string(),
            ViewChoice::Sort(sort) => super::sort_label(Some(*sort)).to_string(),
        }
    }

    fn view_choice_selected(&self, form: &PerspectiveForm, choice: &ViewChoice) -> bool {
        match choice {
            ViewChoice::Project(None) => form.projects.is_empty(),
            ViewChoice::Project(Some(id)) => form.projects.contains(id),
            ViewChoice::Tag(None) => form.tags.is_empty(),
            ViewChoice::Tag(Some(id)) => form.tags.contains(id),
            ViewChoice::Status(status) => form.status == *status,
            ViewChoice::Due(choice) => {
                (*choice == DueChoice::Any && form.dues.is_empty()) || form.dues.contains(choice)
            }
            ViewChoice::Planned(choice) => {
                (*choice == DueChoice::Any && form.planned.is_empty())
                    || form.planned.contains(choice)
            }
            ViewChoice::Defer(choice) => {
                (*choice == DeferChoice::Any && form.defers.is_empty())
                    || form.defers.contains(choice)
            }
            ViewChoice::Fullscreen(value) => form.fullscreen == *value,
            ViewChoice::Group(group) => form.group == *group,
            ViewChoice::Sort(sort) => form.sort == *sort,
        }
    }

    fn toggle_view_menu(&mut self, field: ViewField, cx: &mut Context<Self>) {
        if self.view_menu == Some(field) {
            self.view_menu = None;
        } else {
            let cursor = {
                let Some(Modal::Perspective { form }) = &self.modal else {
                    return;
                };
                self.view_choices(field)
                    .iter()
                    .position(|choice| self.view_choice_selected(form, choice))
                    .unwrap_or(0)
            };
            self.view_menu = Some(field);
            self.view_menu_cursor = cursor;
            self.view_menu_scroll.set_offset(point(px(0.), px(0.)));
        }
        cx.notify();
    }

    pub(super) fn choose_view_choice(&mut self, choice: ViewChoice, cx: &mut Context<Self>) {
        let multi = matches!(
            choice,
            ViewChoice::Project(_)
                | ViewChoice::Tag(_)
                | ViewChoice::Due(_)
                | ViewChoice::Planned(_)
                | ViewChoice::Defer(_)
        );
        if let Some(Modal::Perspective { form }) = &mut self.modal {
            match choice {
                ViewChoice::Project(id) => form.toggle_project(id),
                ViewChoice::Tag(id) => form.toggle_tag(id),
                ViewChoice::Status(status) => form.status = status,
                ViewChoice::Due(choice) => form.toggle_due(choice),
                ViewChoice::Planned(choice) => form.toggle_planned(choice),
                ViewChoice::Defer(choice) => form.toggle_defer(choice),
                ViewChoice::Fullscreen(value) => form.fullscreen = value,
                ViewChoice::Group(group) => form.group = group,
                ViewChoice::Sort(sort) => form.sort = sort,
            }
        }
        if !multi {
            self.view_menu = None;
        }
        cx.notify();
    }

    fn render_view_menu(&self, field: ViewField, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div().into_any_element();
        };
        let choices = self.view_choices(field);
        let mut menu = div()
            .id("view-menu")
            .min_w(px(220.))
            .max_w(px(320.))
            .max_h(px(300.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg()
            .overflow_y_scroll()
            .track_scroll(&self.view_menu_scroll);
        for (index, choice) in choices.iter().enumerate() {
            let selected = self.view_choice_selected(form, choice);
            let cursor = self.view_menu_cursor == index;
            let label = self.view_choice_label(choice);
            let slug = choice.slug();
            let option_key = format!("option:{}:{slug}", field.key());
            let captured = choice.clone();
            menu = menu.child(
                div()
                    .on_children_prepainted(self.view_probe(option_key))
                    .child(
                        div()
                            .id(super::sid(format!("view-option:{}:{slug}", field.key())))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .text_xs()
                            .text_color(if selected {
                                theme.accent
                            } else {
                                theme.text_dim
                            })
                            .bg(if selected {
                                theme.selection
                            } else if cursor {
                                theme.hover
                            } else {
                                theme.clear
                            })
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(label),
                            )
                            .child(if field.is_multi() {
                                view_checkbox(theme, selected).into_any_element()
                            } else if selected {
                                icons::icon(Icon::Check, 11.0, theme.accent).into_any_element()
                            } else {
                                div().w(px(11.)).into_any_element()
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.choose_view_choice(captured.clone(), cx);
                            })),
                    ),
            );
        }
        menu.into_any_element()
    }

    /// The root-layer menu for the open perspective select. It paints after the
    /// sheet so a project or tag list can extend past the card without being
    /// clipped by the scrolling body.
    pub(super) fn render_view_menu_layer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let field = self.view_menu?;
        let (x, y) = {
            let boxes = self.view_boxes.borrow();
            let [x, y, width, height] = *boxes.get(field.key())?;
            let anchor = Bounds {
                origin: point(px(x), px(y)),
                size: size(px(width), px(height)),
            };
            let count = self.view_choices(field).len() as f32;
            let menu_height = (count * 25.0 + 10.0).min(300.0);
            super::menu_position(
                anchor,
                self.window_size.unwrap_or((1254.0, 1404.0)),
                240.0,
                menu_height,
            )
        };
        Some(
            div()
                .id("view-menu-layer")
                .absolute()
                .occlude()
                .left(px(x))
                .top(px(y))
                .child(self.render_view_menu(field, cx))
                .into_any_element(),
        )
    }

    fn render_perspective_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div();
        };
        let form = form.clone();
        let db = self.store.db();
        let icon = form.icon.clone();
        let icon_open = form.icon_open;
        let display_icon = icon.clone().unwrap_or_else(|| {
            IconSpec::builtin(if form.pinned {
                ProjectIcon::Pin
            } else {
                ProjectIcon::Project
            })
        });
        let icon_label = match &icon {
            Some(spec @ IconSpec::Builtin(_)) => spec.label(),
            Some(IconSpec::Emoji(_)) => String::new(),
            None => if form.pinned { "Pin" } else { "Default" }.to_string(),
        };
        let has_icon_label = !icon_label.is_empty();
        let heading = |text: &'static str| {
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(text)
        };

        let icon_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("perspective-icon-row".to_string()))
                    .id("modal-perspective-icon")
                    .relative()
                    .w(px(32.))
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(2.))
                    .border_1()
                    .border_color(if icon_open {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .bg(if icon_open {
                        theme.selection
                    } else {
                        theme.field
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(widgets::icon_spec(&display_icon, 16.0, theme.text_dim))
                    .child(
                        div()
                            .absolute()
                            .bottom(px(1.))
                            .right(px(1.))
                            .child(icons::icon(Icon::ChevronDown, 8.0, theme.text_faint)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.icon_open = !form.icon_open;
                            if form.icon_open {
                                this.modal_focus = 5;
                            }
                        }
                        cx.notify();
                    })),
            )
            .when(has_icon_label, |this| {
                this.child(div().text_xs().text_color(theme.text_faint).child(icon_label))
            });

        let mut tabs = div()
            .id("modal-perspective-tabs")
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded_md()
            .bg(theme.field)
            .border_1()
            .border_color(theme.border_soft);
        for tab in [PerspectiveTab::Simple, PerspectiveTab::Comprehensive] {
            let active = form.tab == tab;
            let slug = match tab {
                PerspectiveTab::Simple => "simple",
                PerspectiveTab::Comprehensive => "comprehensive",
            };
            tabs = tabs.child(
                div()
                    .on_children_prepainted(
                        self.view_probe(format!("perspective-tab:{slug}")),
                    )
                    .id(super::sid(format!("perspective-tab:{slug}")))
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .bg(if active { theme.raised } else { theme.clear })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(tab.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.set_tab(tab);
                            this.view_menu = None;
                            this.date_picker = None;
                            this.date_picker_time = None;
                            this.date_picker_step = super::DatePickerStep::Calendar;
                            this.modal_focus = 0;
                        }
                        cx.notify();
                    })),
            );
        }

        let content = match form.tab {
            PerspectiveTab::Simple => self
                .render_simple_perspective_controls(cx)
                .into_any_element(),
            PerspectiveTab::Comprehensive => self
                .render_comprehensive_perspective_controls(cx)
                .into_any_element(),
        };

        let layout_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Group,
                form.group_label().to_string(),
                form.group != GroupBy::None,
                cx,
            )))
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Sort,
                form.sort_label().to_string(),
                false,
                cx,
            )));

        let preview = query::run(
            db,
            &form.filter(),
            form.sort,
            form.group,
            form.show_completed,
            now(),
        );
        let matches = preview
            .iter()
            .map(|group| group.rows.len() + group.projects.len())
            .sum::<usize>();
        let extra = form.extra_condition_count();
        let summary = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(icons::icon(Icon::Perspective, 13.0, theme.text_faint))
                    .child(div().text_sm().text_color(theme.text).child(format!(
                        "{matches} {} match",
                        if matches == 1 { "item" } else { "items" }
                    )))
                    .when(extra > 0, |this| {
                        this.child(div().text_xs().text_color(theme.text_faint).child(format!(
                            "· {extra} advanced condition{} kept",
                            if extra == 1 { "" } else { "s" }
                        )))
                    }),
            )
            .when(form.has_filters(), |this| {
                this.child(
                    div()
                        .on_children_prepainted(self.view_probe("clear".to_string()))
                        .child(
                            div()
                                .id("modal-clear-filters")
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .child("Clear filters")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.clear_filters();
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(icon_row)
                    .child(tabs),
            )
            .when(icon_open, |this| {
                this.child(self.render_icon_picker_card(
                    IconTarget::Perspective,
                    5,
                    form.icon.as_ref(),
                    form.icon_tab,
                    cx,
                ))
            })
            .child(widgets::divider(theme))
            .child(content)
            .child(widgets::divider(theme))
            .child(heading("PRESENTATION"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(px(150.))
                            .text_sm()
                            .text_color(theme.text)
                            .child("Group and sort:"),
                    )
                    .child(
                        div()
                            .h(px(28.))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.field)
                            .text_sm()
                            .text_color(theme.text)
                            .child("Individual Actions"),
                    ),
            )
            .child(self.presentation_select(
                ViewField::Group,
                "Group actions by:",
                form.group_label().to_string(),
                form.group != GroupBy::None,
                cx,
            ))
            .child(self.presentation_select(
                ViewField::Sort,
                "Sort actions by:",
                form.sort_label().to_string(),
                false,
                cx,
            ))
            .child(widgets::divider(theme))
            .child(summary)
            .when(matches == 0 && form.has_filters(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .whitespace_normal()
                        .text_color(theme.warning)
                        .child(
                            "No items match. A rule and a None group can cancel each other out — for example, Available plus None of Available.",
                        ),
                )
            })
    }

    fn render_simple_perspective_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div();
        };
        let form = form.clone();
        let db = self.store.db();
        let icon = form.icon.clone();
        let icon_open = form.icon_open;
        let display_icon = icon.clone().unwrap_or_else(|| {
            IconSpec::builtin(if form.pinned {
                ProjectIcon::Pin
            } else {
                ProjectIcon::Project
            })
        });
        let icon_label = match &icon {
            Some(spec @ IconSpec::Builtin(_)) => spec.label(),
            Some(IconSpec::Emoji(_)) => String::new(),
            None => if form.pinned { "Pin" } else { "Default" }.to_string(),
        };
        let has_icon_label = !icon_label.is_empty();
        let project_label = match form.projects.len() {
            0 => "Any".to_string(),
            1 => db.project_path(form.projects.first().expect("length checked")),
            count => format!("{count} projects"),
        };
        let tag_label = match form.tags.len() {
            0 => "Any".to_string(),
            1 => form
                .tags
                .first()
                .and_then(|id| db.tags.get(id))
                .map(|tag| format!("#{}", tag.name))
                .unwrap_or_else(|| "Any".to_string()),
            count => format!("{count} tags"),
        };
        let due_label = match form.dues.len() {
            0 => "Any".to_string(),
            1 => form
                .dues
                .first()
                .expect("length checked")
                .label()
                .to_string(),
            count => format!("{count} date filters"),
        };
        let planned_label = match form.planned.len() {
            0 => "Any".to_string(),
            1 => form
                .planned
                .first()
                .expect("length checked")
                .label()
                .to_string(),
            count => format!("{count} planned filters"),
        };
        let defer_label = match form.defers.len() {
            0 => "Any".to_string(),
            1 => form
                .defers
                .first()
                .expect("length checked")
                .label()
                .to_string(),
            count => format!("{count} defer filters"),
        };
        let fullscreen_label = match form.fullscreen {
            None => "Any".to_string(),
            Some(true) => "Has full-screen reminder".to_string(),
            Some(false) => "No full-screen reminder".to_string(),
        };
        let extra = form.extra_condition_count();
        // A live count turns the sheet into a preview: every change shows how
        // many tasks the saved view would list.
        let preview = query::run(
            db,
            &form.filter(),
            form.sort,
            form.group,
            form.show_completed,
            now(),
        );
        let matches = preview.iter().map(|group| group.rows.len()).sum::<usize>();

        let toggle = |label: &'static str, active: bool, id: &'static str| {
            div()
                .on_children_prepainted(self.view_probe(id.to_string()))
                .id(id)
                .child(widgets::toggle_chip(theme, label, active))
        };
        let toggles = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("modal-inbox".to_string()))
                    .id("modal-inbox-wrap")
                    .child(toggle("Inbox only", form.inbox_only, "modal-inbox"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.toggle_inbox();
                        }
                        cx.notify();
                    })),
            )
            .child(
                toggle("Flagged only", form.flagged_only, "modal-flagged").on_click(cx.listener(
                    |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.flagged_only = !form.flagged_only;
                        }
                        cx.notify();
                    },
                )),
            )
            .child(
                toggle("Completed", form.show_completed, "modal-completed")
                    .flow_tip(self, cx, theme, "Include completed tasks")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.show_completed = !form.show_completed;
                        }
                        cx.notify();
                    })),
            );

        let priority_toggles = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Priority"),
            )
            .children(Priority::ALL.iter().map(|priority| {
                let active = form.priorities.contains(priority);
                div()
                    .id(gpui::ElementId::Name(
                        format!("modal-priority-{}", priority.slug()).into(),
                    ))
                    .child(widgets::toggle_chip(theme, priority.label(), active))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.toggle_priority(*priority);
                        }
                        cx.notify();
                    }))
            }));

        let icon_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(theme.text_faint).child("Icon"))
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("perspective-icon-row".to_string()))
                    .id("modal-perspective-icon")
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if icon_open {
                        theme.accent
                    } else {
                        theme.border_soft
                    })
                    .bg(if icon_open {
                        theme.selection
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(widgets::icon_spec(&display_icon, 14.0, theme.text_dim))
                    .when(has_icon_label, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(icon_label),
                        )
                    })
                    .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.icon_open = !form.icon_open;
                            if form.icon_open {
                                this.modal_focus = 5;
                            }
                        }
                        cx.notify();
                    })),
            );
        let project_tag_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Project,
                project_label,
                !form.projects.is_empty(),
                cx,
            )))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.perspective_select(
                        ViewField::Tag,
                        tag_label,
                        !form.tags.is_empty(),
                        cx,
                    ))
                    .when(!form.tags.is_empty(), |this| {
                        this.child(
                            div()
                                .id("modal-tag-children")
                                .flex()
                                .flow_tip(self, cx, theme, "Match tasks tagged with a child tag")
                                .child(widgets::toggle_chip(theme, "Child tags", form.tag_children))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.tag_children = !form.tag_children;
                                    }
                                    cx.notify();
                                })),
                        )
                    }),
            );

        let windows_row = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(self.perspective_select(
                        ViewField::Status,
                        form.status_label().to_string(),
                        form.status != TaskStatus::Any,
                        cx,
                    )))
                    .child(div().flex_1().min_w_0().child(self.perspective_select(
                        ViewField::Due,
                        due_label,
                        !form.dues.is_empty(),
                        cx,
                    ))),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(self.perspective_select(
                        ViewField::Planned,
                        planned_label,
                        !form.planned.is_empty(),
                        cx,
                    )))
                    .child(div().flex_1().min_w_0().child(self.perspective_select(
                        ViewField::Defer,
                        defer_label,
                        !form.defers.is_empty(),
                        cx,
                    ))),
            );

        let fullscreen_row =
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(div().flex_1().min_w_0().child(self.perspective_select(
                    ViewField::Fullscreen,
                    fullscreen_label,
                    form.fullscreen.is_some(),
                    cx,
                )));

        let layout_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Group,
                form.group_label().to_string(),
                form.group != GroupBy::None,
                cx,
            )))
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Sort,
                form.sort_label().to_string(),
                false,
                cx,
            )));

        let heading = |text: &'static str| {
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(text)
        };
        let summary = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(icons::icon(Icon::Perspective, 13.0, theme.text_faint))
                    .child(div().text_sm().text_color(theme.text).child(format!(
                        "{matches} {} match",
                        if matches == 1 { "task" } else { "tasks" }
                    )))
                    .when(extra > 0, |this| {
                        this.child(div().text_xs().text_color(theme.text_faint).child(format!(
                            "· {extra} CLI condition{} kept",
                            if extra == 1 { "" } else { "s" }
                        )))
                    }),
            )
            .when(form.has_filters(), |this| {
                this.child(
                    div()
                        .on_children_prepainted(self.view_probe("clear".to_string()))
                        .child(
                            div()
                                .id("modal-clear-filters")
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .child("Clear filters")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.clear_filters();
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(heading("FILTERS"))
            .child(toggles)
            .child(priority_toggles)
            .child(project_tag_row)
            .child(windows_row)
            .child(fullscreen_row)
    }

    fn render_comprehensive_perspective_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div();
        };
        let form = form.clone();
        let db = self.store.db();
        let icon = form.icon.clone();
        let icon_open = form.icon_open;
        let display_icon = icon.clone().unwrap_or_else(|| {
            IconSpec::builtin(if form.pinned {
                ProjectIcon::Pin
            } else {
                ProjectIcon::Project
            })
        });
        let icon_label = match &icon {
            Some(spec @ IconSpec::Builtin(_)) => spec.label(),
            Some(IconSpec::Emoji(_)) => String::new(),
            None => if form.pinned { "Pin" } else { "Default" }.to_string(),
        };
        let has_icon_label = !icon_label.is_empty();
        let preview = query::run(
            db,
            &form.filter(),
            form.sort,
            form.group,
            form.show_completed,
            now(),
        );
        let matches = preview
            .iter()
            .map(|group| group.rows.len() + group.projects.len())
            .sum::<usize>();

        let icon_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(theme.text_faint).child("Icon"))
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("perspective-icon-row".to_string()))
                    .id("modal-perspective-icon")
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if icon_open {
                        theme.accent
                    } else {
                        theme.border_soft
                    })
                    .bg(if icon_open {
                        theme.selection
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(widgets::icon_spec(&display_icon, 14.0, theme.text_dim))
                    .when(has_icon_label, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(icon_label),
                        )
                    })
                    .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.icon_open = !form.icon_open;
                            if form.icon_open {
                                this.modal_focus = 5;
                            }
                        }
                        cx.notify();
                    })),
            );

        let heading = |text: &'static str| {
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(text)
        };

        let summary = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(icons::icon(Icon::Perspective, 13.0, theme.text_faint))
                    .child(div().text_sm().text_color(theme.text).child(format!(
                        "{matches} {} match",
                        if matches == 1 { "task" } else { "tasks" }
                    )))
                    .when(form.editing.is_some(), |this| {
                        this.child(div().text_xs().text_color(theme.text_faint).child("· editing a rule"))
                    }),
            )
            .when(form.has_filters(), |this| {
                this.child(
                    div()
                        .on_children_prepainted(self.view_probe("clear".to_string()))
                        .child(
                            div()
                                .id("modal-clear-filters")
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .child("Clear filters")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.clear_filters();
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(heading("CONTENTS"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .p_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border_soft)
                    .bg(theme.field)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .children(self.render_rule_rows(&form.root, 0, None, true, cx)),
                    )
                    .child(
                        div()
                            .id("rule-add-bottom")
                            .flex()
                            .items_center()
                            .gap_2()
                            .h(px(30.))
                            .pl(px(12.))
                            .pr_2()
                            .rounded_sm()
                            .text_sm()
                            .text_color(theme.text_faint)
                            .cursor_pointer()
                            .hover(|style| {
                                style
                                    .bg(theme.hover)
                                    .text_color(theme.text)
                            })
                            .child(icons::icon(Icon::Plus, 10.0, theme.text_faint))
                            .child("Add a new rule")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::Perspective { form }) = &mut this.modal {
                                    form.add_menu = Some(form.root.id);
                                    form.add_after = None;
                                    form.editing = None;
                                    form.row_menu = None;
                                    this.rule_menu_scroll
                                        .set_offset(point(px(0.), px(0.)));
                                }
                                cx.notify();
                            })),
                    ),
            )
    }

    fn render_layout_select(
        &self,
        field: LayoutField,
        value: String,
        active: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let open = matches!(
            &self.modal,
            Some(Modal::Perspective { form }) if form.layout_menu == Some(field)
        );
        let key = match field {
            LayoutField::Group => "group",
            LayoutField::Sort => "sort",
        };
        let label = match field {
            LayoutField::Group => "Group by",
            LayoutField::Sort => "Sort by",
        };
        let mut card = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(
                div()
                    .id(super::sid(format!("layout-{key}")))
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if active || open { theme.accent } else { theme.border })
                    .bg(if active || open {
                        theme.selection
                    } else {
                        theme.field
                    })
                    .text_sm()
                    .text_color(if active { theme.accent } else { theme.text })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(value),
                    )
                    .child(icons::icon(
                        Icon::ChevronDown,
                        11.0,
                        if active { theme.accent } else { theme.text_faint },
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.layout_menu = if form.layout_menu == Some(field) {
                                None
                            } else {
                                Some(field)
                            };
                        }
                        cx.notify();
                    })),
            );
        if open {
            let choices: Vec<(String, String)> = match field {
                LayoutField::Group => [
                    (GroupBy::None, "None"),
                    (GroupBy::Project, "Project"),
                    (GroupBy::Folder, "Folder"),
                    (GroupBy::Tag, "Tag"),
                    (GroupBy::DueDate, "Due"),
                    (GroupBy::DeferDate, "Defer"),
                    (GroupBy::PlannedDate, "Planned"),
                    (GroupBy::CompletedDate, "Completed"),
                    (GroupBy::DroppedDate, "Dropped"),
                    (GroupBy::AddedDate, "Added"),
                    (GroupBy::ChangedDate, "Changed"),
                    (GroupBy::NextReview, "Next Review"),
                    (GroupBy::TagsCombined, "Tags (combined)"),
                    (GroupBy::Flagged, "Flagged"),
                    (GroupBy::Estimate, "Estimate"),
                ]
                .into_iter()
                .map(|(group, label)| (group.slug().to_string(), label.to_string()))
                .collect(),
                LayoutField::Sort => [
                    (SortBy::Manual, "Manual"),
                    (SortBy::Due, "Due"),
                    (SortBy::Defer, "Defer"),
                    (SortBy::Planned, "Planned"),
                    (SortBy::Name, "Name"),
                    (SortBy::Priority, "Priority"),
                    (SortBy::Flagged, "Flagged"),
                    (SortBy::Estimate, "Estimate"),
                    (SortBy::Created, "Created"),
                    (SortBy::CompletedAt, "Completed"),
                    (SortBy::DueFlagged, "Due & Flagged"),
                    (SortBy::Tag, "Tag"),
                    (SortBy::AddedDate, "Added"),
                    (SortBy::ChangedDate, "Changed"),
                    (SortBy::DroppedDate, "Dropped"),
                    (SortBy::NextReview, "Next Review"),
                ]
                .into_iter()
                .map(|(sort, label)| (sort.slug().to_string(), label.to_string()))
                .collect(),
            };
            let mut menu = div()
                .id("layout-menu")
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                .flex()
                .flex_col()
                .gap(px(1.))
                .p_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.raised)
                .max_h(px(220.))
                .overflow_y_scroll();
            for (slug, label) in choices {
                let theme = theme;
                let slug_for_id = slug.clone();
                let captured = slug;
                let captured_label = label.clone();
                menu = menu.child(
                    div()
                        .id(super::sid(format!("layout-option-{key}-{slug_for_id}")))
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .text_xs()
                        .text_color(theme.text_dim)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .child(captured_label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(Modal::Perspective { form }) = &mut this.modal {
                                match field {
                                    LayoutField::Group => {
                                        if let Some(group) = GroupBy::from_slug(&captured) {
                                            form.group = group;
                                        }
                                    }
                                    LayoutField::Sort => {
                                        if let Some(sort) = SortBy::from_slug(&captured) {
                                            form.sort = sort;
                                        }
                                    }
                                }
                                form.layout_menu = None;
                            }
                            cx.notify();
                        })),
                );
            }
            card = card.child(menu);
        }
        card.into_any_element()
    }

    fn render_rule_rows(
        &self,
        node: &RuleNode,
        depth: usize,
        parent: Option<u64>,
        is_root: bool,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let theme = self.theme;
        let db = self.store.db();
        let id = node.id;
        let is_group = node.kind.is_group();
        let editing = matches!(&self.modal, Some(Modal::Perspective { form }) if form.editing == Some(id));
        let menu_open = matches!(&self.modal, Some(Modal::Perspective { form }) if form.row_menu == Some(id));
        let disabled = !node.enabled;
        let summary = rule_summary(db, node);
        let indent = (depth as f32) * 18.0;
        let group = format!("perspective-rule-{id}");
        let base = if is_group {
            format!("{summary}:")
        } else {
            summary.clone()
        };
        let label = if disabled {
            format!("{base}  · off")
        } else {
            base
        };

        let mut row = div()
            .on_children_prepainted(self.view_probe(format!("rule-select:{id}")))
            .id(super::sid(format!("rule-select:{id}")))
            .group(group.clone())
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(px(30.))
            .pl(px(indent + 8.0))
            .pr_2()
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .text_color(if disabled {
                theme.text_faint
            } else if is_group {
                theme.text
            } else {
                theme.text_dim
            })
            .bg(if editing { theme.selection } else { theme.clear })
            .hover(|style| style.bg(theme.hover));
        // Code-editor style indent guides. Each ancestor level paints a faint
        // vertical line through the row, so deep rules read as nested.
        for level in 0..depth {
            row = row.child(
                div()
                    .absolute()
                    .left(px(12.0 + level as f32 * 18.0))
                    .top_0()
                    .bottom_0()
                    .w(px(1.))
                    .bg(theme.text_faint)
                    .opacity(0.45),
            );
        }
        row = row
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(is_group, |this| this.font_weight(gpui::FontWeight::MEDIUM))
                    .child(label),
            )
            .child(div().flex_grow())
            .child(
                div()
                    .opacity(0.32)
                    .group_hover(group.clone(), |style| style.opacity(1.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .on_children_prepainted(self.view_probe(format!("rule-plus:{id}")))
                            .id(super::sid(format!("rule-plus:{id}")))
                            .w(px(20.))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_color(theme.text_faint)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover).text_color(theme.text))
                            .child(icons::icon(Icon::Plus, 10.0, theme.text_faint))
                            .flow_tip(self, cx, theme, "Add a rule here")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if let Some(Modal::Perspective { form }) = &mut this.modal {
                                    if is_group {
                                        form.add_menu = Some(id);
                                        form.add_after = None;
                                    } else {
                                        form.add_menu = parent;
                                        form.add_after = Some(id);
                                    }
                                    form.editing = None;
                                    this.rule_menu_scroll
                                        .set_offset(point(px(0.), px(0.)));
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .on_children_prepainted(self.view_probe(format!("rule-menu:{id}")))
                            .id(super::sid(format!("rule-menu:{id}")))
                            .w(px(20.))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_color(if menu_open { theme.text } else { theme.text_faint })
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover).text_color(theme.text))
                            .child("⋯")
                            .flow_tip(self, cx, theme, "Rule actions")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if let Some(Modal::Perspective { form }) = &mut this.modal {
                                    form.row_menu = if form.row_menu == Some(id) {
                                        None
                                    } else {
                                        Some(id)
                                    };
                                    form.editing = None;
                                    form.add_menu = None;
                                    form.add_after = None;
                                }
                                cx.notify();
                            })),
                    ),
            );

        row = row.on_click(cx.listener(move |this, _, _, cx| {
            if let Some(Modal::Perspective { form }) = &mut this.modal {
                form.editing = if form.editing == Some(id) {
                    None
                } else {
                    Some(id)
                };
                form.row_menu = None;
                form.add_menu = None;
                form.add_after = None;
            }
            cx.notify();
        }));
        if !is_root {
            let payload = DragRule::rule(id, summary.clone());
            row = row
                .on_drag(payload, move |payload, position, window, cx| {
                    rule_ghost(payload, position, window, cx, theme)
                })
                .on_drop::<DragRule>(cx.listener(move |this, payload: &DragRule, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        let moved = if is_group {
                            form.move_into(payload.id, id)
                        } else {
                            form.move_before(payload.id, id)
                        };
                        if moved {
                            form.editing = None;
                        }
                    }
                    cx.notify();
                }));
        }

        let mut out = vec![row.into_any_element()];
        if editing {
            out.push(self.render_rule_editor(node, depth + 1, cx));
        }
        if menu_open {
            out.push(
                div()
                    .flex()
                    .justify_end()
                    .pr(px(8.))
                    .child(self.render_rule_menu(node, parent, depth + 1, cx))
                    .into_any_element(),
            );
        }
        if is_group {
            if node.children().is_empty() {
                out.push(
                    div()
                        .pl(px(((depth + 1) as f32) * 18.0 + 12.0))
                        .py_1()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("No rules")
                        .into_any_element(),
                );
            }
            for child in node.children() {
                out.extend(self.render_rule_rows(child, depth + 1, Some(id), false, cx));
            }
            if matches!(&self.modal, Some(Modal::Perspective { form }) if form.add_menu == Some(id) && form.add_after.is_none())
            {
                out.push(self.render_add_rule_menu(id, depth + 1, cx));
            }
        }
        if matches!(&self.modal, Some(Modal::Perspective { form }) if form.add_menu == parent && form.add_after == Some(id))
        {
            out.push(self.render_add_rule_menu(parent.unwrap_or(id), depth, cx));
        }
        out
    }

    fn render_add_rule_row(
        &self,
        parent: u64,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let open = matches!(&self.modal, Some(Modal::Perspective { form }) if form.add_menu == Some(parent));
        div()
            .on_children_prepainted(self.view_probe(format!("rule-add:{parent}")))
            .id(super::sid(format!("rule-add:{parent}")))
            .flex()
            .items_center()
            .gap_1()
            .min_h(px(24.))
            .pl(px((depth as f32) * 14.0 + 20.0))
            .rounded_md()
            .text_xs()
            .text_color(if open { theme.accent } else { theme.text_faint })
            .cursor_pointer()
            .hover(|style| style.text_color(theme.accent))
            .child(icons::icon(Icon::Plus, 10.0, theme.text_faint))
            .child("Add a new rule")
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(Modal::Perspective { form }) = &mut this.modal {
                    form.add_menu = if form.add_menu == Some(parent) {
                        None
                    } else {
                        Some(parent)
                    };
                    form.editing = None;
                    form.row_menu = None;
                }
                cx.notify();
            }))
            .into_any_element()
    }

    fn render_add_rule_menu(
        &self,
        parent: u64,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut menu = div()
            .on_children_prepainted(self.view_probe("rule-add-menu".to_string()))
            .id("rule-add-menu")
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .ml(px((depth as f32) * 24.0 + 40.0))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .max_h(px(320.))
            .overflow_y_scroll()
            .track_scroll(&self.rule_menu_scroll);
        menu = menu.child(
            div()
                .px_2()
                .pt_1()
                .pb_1()
                .text_size(px(10.))
                .text_color(theme.text_faint)
                .child("Choosing a rule adds a new card under this group"),
        );
        let mut category = "";
        for template in RuleTemplate::ALL {
            if template.category() != category {
                category = template.category();
                menu = menu.child(
                    div()
                        .pl_2()
                        .pr_2()
                        .pt_2()
                        .pb_1()
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_faint)
                        .child(category),
                );
            }
            let label = template.label().to_string();
            let slug = label
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect::<String>();
            menu = menu.child(
                div()
                    .on_children_prepainted(
                        self.view_probe(format!("rule-option:{parent}:{slug}")),
                    )
                    .id(super::sid(format!("rule-option:{parent}:{slug}")))
                    .pl_7()
                    .pr_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(theme.text_dim)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover).text_color(theme.text))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            if let Some(after) = form.add_after {
                                form.add_rule_after(after, template);
                            } else {
                                form.add_rule(parent, template);
                            }
                        }
                        cx.notify();
                    })),
            );
        }
        menu.into_any_element()
    }

    fn render_rule_menu(
        &self,
        node: &RuleNode,
        parent: Option<u64>,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let id = node.id;
        let enabled = node.enabled;
        let is_group = node.kind.is_group();
        let is_root = matches!(
            &self.modal,
            Some(Modal::Perspective { form }) if form.root.id == id
        );
        let mut menu = div()
            .w(px(176.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_md();
        let items: Vec<(String, Option<&'static str>)> = if is_group && is_root {
            vec![
                ("Edit Group".to_string(), Some("edit")),
                ("Add Rule Inside".to_string(), Some("add-inside")),
                (
                    if enabled { "Turn Off" } else { "Turn On" }.to_string(),
                    Some("toggle"),
                ),
            ]
        } else if is_group {
            vec![
                ("Edit Group".to_string(), Some("edit")),
                ("Add Rule Inside".to_string(), Some("add-inside")),
                (
                    if enabled { "Turn Off" } else { "Turn On" }.to_string(),
                    Some("toggle"),
                ),
                ("Duplicate".to_string(), Some("duplicate")),
                ("Move Up".to_string(), Some("up")),
                ("Move Down".to_string(), Some("down")),
                ("Delete".to_string(), Some("delete")),
            ]
        } else {
            vec![
                ("Edit Rule".to_string(), Some("edit")),
                ("Add Rule After".to_string(), Some("add-after")),
                (
                    if enabled { "Turn Off" } else { "Turn On" }.to_string(),
                    Some("toggle"),
                ),
                ("Duplicate".to_string(), Some("duplicate")),
                ("Move Up".to_string(), Some("up")),
                ("Move Down".to_string(), Some("down")),
                ("Delete".to_string(), Some("delete")),
            ]
        };
        for (label, action) in items {
            let slug = label.replace(' ', "-").to_lowercase();
            menu = menu.child(
                div()
                    .on_children_prepainted(
                        self.view_probe(format!("rule-menu-item:{id}:{slug}")),
                    )
                    .id(super::sid(format!("rule-menu-item:{id}:{slug}")))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if action == Some("delete") {
                        theme.danger
                    } else {
                        theme.text_dim
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            match action {
                                Some("edit") => {
                                    form.editing = Some(id);
                                }
                                Some("add-inside") => {
                                    form.add_menu = Some(id);
                                    form.add_after = None;
                                }
                                Some("add-after") => {
                                    form.add_menu = parent;
                                    form.add_after = Some(id);
                                }
                                Some("toggle") => {
                                    let now = form.is_rule_enabled(id);
                                    form.set_rule_enabled(id, !now);
                                }
                                Some("duplicate") => {
                                    form.duplicate_rule(id);
                                }
                                Some("up") => {
                                    form.move_rule(id, true);
                                }
                                Some("down") => {
                                    form.move_rule(id, false);
                                }
                                Some("delete") => form.delete_rule(id),
                                _ => {}
                            }
                            form.row_menu = None;
                        }
                        cx.notify();
                    })),
            );
        }
        menu.into_any_element()
    }

    fn render_rule_editor(
        &self,
        node: &RuleNode,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let id = node.id;
        let mut card = div()
            .ml(px((depth as f32) * 18.0 + 12.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field);
        let title = rule_summary(self.store.db(), node);
        card = card.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_faint)
                        .child(title),
                )
                .child(
                    div()
                        .on_children_prepainted(self.view_probe(format!("rule-close:{id}")))
                        .id(super::sid(format!("rule-close:{id}")))
                        .px_1()
                        .rounded_sm()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover).text_color(theme.text))
                        .child("Done")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(Modal::Perspective { form }) = &mut this.modal {
                                form.editing = None;
                            }
                            cx.notify();
                        })),
                ),
        );
        if node.kind.is_group() {
            let current = match node.kind {
                RuleKind::All(_) => GroupOperator::All,
                RuleKind::Any(_) => GroupOperator::Any,
                RuleKind::None(_) => GroupOperator::None,
                _ => GroupOperator::All,
            };
            let mut chips = div().flex().items_center().gap_1();
            for operator in [
                GroupOperator::All,
                GroupOperator::Any,
                GroupOperator::None,
            ] {
                chips = chips.child(
                    div()
                        .id(super::sid(format!("rule-op-choice:{id}:{}", operator.label())))
                        .child(widgets::toggle_chip(
                            theme,
                            operator.label(),
                            operator == current,
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(Modal::Perspective { form }) = &mut this.modal {
                                form.set_group_operator(id, operator);
                            }
                            cx.notify();
                        })),
                );
            }
            card = card
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_dim)
                        .child("Combine the rules below with"),
                )
                .child(chips);
        } else if matches!(node.kind, RuleKind::Comment) {
            card = card.child(self.rule_text_input(0, "Comment", cx));
        } else if let Some(condition) = node.condition_ref().cloned() {
            card = card.child(self.render_condition_editor(id, &condition, cx));
        }
        card.into_any_element()
    }

    fn rule_text_input(
        &self,
        index: usize,
        placeholder: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div().into_any_element();
        };
        let Some(buffer) = form.editing_buffer(index) else {
            return div().into_any_element();
        };
        let focus_index = 8 + index;
        let focused = self.modal_focus == focus_index;
        let field = widgets::text_field_with_cursor(
            theme,
            buffer,
            placeholder,
            focused,
            cx.listener(move |this, hit: &widgets::TextHit, _, cx| {
                this.set_modal_cursor(focus_index, hit.index, hit.extend, hit.click_count, cx);
            }),
        );
        div()
            .id(super::sid(format!("rule-text:{index}")))
            .flex_1()
            .min_w_0()
            .child(field)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.modal_focus = focus_index;
                this.focus = super::FocusArea::Modal;
                cx.notify();
            }))
            .into_any_element()
    }

    fn rule_date_button(
        &self,
        id: u64,
        second: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let anchors = self.rule_date_anchors.clone();
        let boxes = self.view_boxes.clone();
        let anchor_key = format!("{id}:{second}");
        let probe_key = format!("rule-date:{id}:{second}");
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    anchors
                        .borrow_mut()
                        .insert(anchor_key.clone(), *bounds);
                    boxes.borrow_mut().insert(
                        probe_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .id(super::sid(format!("rule-date:{id}:{second}")))
            .w(px(24.))
            .h(px(24.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .text_color(theme.text_faint)
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover).text_color(theme.text))
            .child(icons::icon(Icon::Calendar, 12.0, theme.text_faint))
            .flow_tip(self, cx, theme, "Pick a date")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_date_picker(
                    PickerDateField::RuleDate { rule: id, second },
                    cx,
                )
            }))
            .into_any_element()
    }

    fn rule_chip(
        &self,
        id: String,
        label: String,
        active: bool,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .on_children_prepainted(self.view_probe(id.clone()))
            .id(super::sid(id))
            .child(widgets::toggle_chip(theme, label, active))
            .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
            .into_any_element()
    }

    fn with_rule(
        &mut self,
        id: u64,
        change: impl FnOnce(&mut RuleNode),
        cx: &mut Context<Self>,
    ) {
        if let Some(Modal::Perspective { form }) = &mut self.modal
            && let Some(node) = form.rule_mut(id)
        {
            change(node);
            node.sync_text();
        }
        cx.notify();
    }

    fn render_condition_editor(
        &self,
        id: u64,
        condition: &Condition,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let mut body = div().flex().flex_col().gap_2();

        let chip_row = |body: gpui::Div, children: Vec<AnyElement>| {
            body.child(div().flex().flex_wrap().items_center().gap_1().children(children))
        };

        match condition {
            Condition::StatusKind { value } => {
                let mut chips = Vec::new();
                for choice in FilterStatus::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-state:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::StatusKind { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::Availability { value } => {
                let mut chips = Vec::new();
                for choice in AvailabilityFilter::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-availability:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::Availability { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::EstimateLessThan { minutes }
            | Condition::EstimateGreaterThan { minutes } => {
                let greater = matches!(condition, Condition::EstimateGreaterThan { .. });
                let mut chips = Vec::new();
                for choice in [5u32, 15, 30, 60] {
                    chips.push(self.rule_chip(
                        format!(
                            "rule-estimate:{}:{id}:{choice}",
                            if greater { "gt" } else { "lt" }
                        ),
                        format!("{choice} minutes"),
                        *minutes == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| match &mut node.kind {
                                    RuleKind::Condition(Condition::EstimateLessThan {
                                        minutes,
                                    }) if !greater => *minutes = choice,
                                    RuleKind::Condition(Condition::EstimateGreaterThan {
                                        minutes,
                                    }) if greater => *minutes = choice,
                                    _ => {}
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child("Custom minutes"),
                        )
                        .child(self.rule_text_input(0, "e.g. 45", cx)),
                );
            }
            Condition::HasDate { field, value } => {
                let mut chips = Vec::new();
                for choice in DateField::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-hasdate-field:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *field == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::HasDate { field, .. }) =
                                        &mut node.kind
                                    {
                                        *field = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
                body = chip_row(
                    body,
                    vec![
                        self.rule_chip(
                            format!("rule-hasdate:{id}:yes"),
                            "Has".to_string(),
                            *value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::HasDate {
                                            value, ..
                                        }) = &mut node.kind
                                        {
                                            *value = true;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                        self.rule_chip(
                            format!("rule-hasdate:{id}:no"),
                            "Does not have".to_string(),
                            !*value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::HasDate {
                                            value, ..
                                        }) = &mut node.kind
                                        {
                                            *value = false;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                    ],
                );
            }
            Condition::DateRange { field, range } => {
                let mut field_chips = Vec::new();
                for choice in DateField::ALL {
                    field_chips.push(self.rule_chip(
                        format!("rule-range-field:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *field == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::DateRange {
                                        field, ..
                                    }) = &mut node.kind
                                    {
                                        *field = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, field_chips);

                let modes: [(&str, DateRangeFilter); 7] = [
                    ("On", DateRangeFilter::On { date: flowlist_core::time::local_date(now()) }),
                    ("Yesterday", DateRangeFilter::Yesterday),
                    ("Today", DateRangeFilter::Today),
                    ("Tomorrow", DateRangeFilter::Tomorrow),
                    (
                        "In the past",
                        DateRangeFilter::InPast {
                            amount: 7,
                            unit: DateUnit::Days,
                        },
                    ),
                    (
                        "In the next",
                        DateRangeFilter::InNext {
                            amount: 7,
                            unit: DateUnit::Days,
                        },
                    ),
                    (
                        "Between",
                        DateRangeFilter::Between {
                            from: Some(flowlist_core::time::local_date(now())),
                            to: Some(
                                flowlist_core::time::local_date(now())
                                    + chrono::Duration::days(7),
                            ),
                        },
                    ),
                ];
                let mut mode_chips = Vec::new();
                for (label, mode) in modes {
                    let active = std::mem::discriminant(range) == std::mem::discriminant(&mode);
                    mode_chips.push(self.rule_chip(
                        format!("rule-range-mode:{id}:{label}"),
                        label.to_string(),
                        active,
                        move |this, cx| {
                            let today = flowlist_core::time::local_date(now());
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::DateRange {
                                        range, ..
                                    }) = &mut node.kind
                                    {
                                        *range = mode.clone();
                                        match &mode {
                                            DateRangeFilter::On { date } => {
                                                node.text.set(date.to_string());
                                                node.text2.set(String::new());
                                            }
                                            DateRangeFilter::Between { from, to } => {
                                                node.text.set(
                                                    from.map(|date| date.to_string())
                                                        .unwrap_or_default(),
                                                );
                                                node.text2.set(
                                                    to.map(|date| date.to_string())
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            DateRangeFilter::InPast { amount, .. }
                                            | DateRangeFilter::InNext { amount, .. } => {
                                                node.text.set(amount.to_string());
                                            }
                                            DateRangeFilter::Yesterday
                                            | DateRangeFilter::Today
                                            | DateRangeFilter::Tomorrow => {
                                                node.text.set(today.to_string());
                                            }
                                            _ => {}
                                        }
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, mode_chips);

                match range {
                    DateRangeFilter::On { .. } => {
                        body = body.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_xs().text_color(theme.text_faint).child("Date"))
                                .child(self.rule_text_input(0, "YYYY-MM-DD", cx))
                                .child(self.rule_date_button(id, false, cx)),
                        );
                    }
                    DateRangeFilter::Between { .. } => {
                        body = body.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_xs().text_color(theme.text_faint).child("From"))
                                .child(self.rule_text_input(0, "YYYY-MM-DD", cx))
                                .child(self.rule_date_button(id, false, cx))
                                .child(div().text_xs().text_color(theme.text_faint).child("To"))
                                .child(self.rule_text_input(1, "YYYY-MM-DD", cx))
                                .child(self.rule_date_button(id, true, cx)),
                        );
                    }
                    DateRangeFilter::InPast { unit, .. }
                    | DateRangeFilter::InNext { unit, .. } => {
                        let current = *unit;
                        body = body.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_xs().text_color(theme.text_faint).child("Amount"))
                                .child(self.rule_text_input(0, "7", cx)),
                        );
                        let mut chips = Vec::new();
                        for choice in DateUnit::ALL {
                            chips.push(self.rule_chip(
                                format!("rule-range-unit:{id}:{}", choice.label()),
                                choice.label().to_string(),
                                current == choice,
                                move |this, cx| {
                                    this.with_rule(
                                        id,
                                        |node| {
                                            if let RuleKind::Condition(Condition::DateRange {
                                                range, ..
                                            }) = &mut node.kind
                                            {
                                                match range {
                                                    DateRangeFilter::InPast { unit, .. }
                                                    | DateRangeFilter::InNext { unit, .. } => {
                                                        *unit = choice
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        },
                                        cx,
                                    )
                                },
                                cx,
                            ));
                        }
                        body = chip_row(body, chips);
                    }
                    _ => {}
                }
            }
            Condition::TagStatus { value } => {
                let mut chips = Vec::new();
                for choice in TagStatusFilter::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-tagstatus:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::TagStatus { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::TagsAny {
                ids,
                include_children,
            }
            | Condition::TagsAll {
                ids,
                include_children,
            } => {
                let all = matches!(condition, Condition::TagsAll { .. });
                body = body.child(self.render_tag_picker(id, ids, *include_children, all, cx));
            }
            Condition::IsUntagged { value } => {
                body = chip_row(
                    body,
                    vec![
                        self.rule_chip(
                            format!("rule-untagged:{id}:yes"),
                            "Is untagged".to_string(),
                            *value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::IsUntagged {
                                            value,
                                        }) = &mut node.kind
                                        {
                                            *value = true;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                        self.rule_chip(
                            format!("rule-untagged:{id}:no"),
                            "Is tagged".to_string(),
                            !*value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::IsUntagged {
                                            value,
                                        }) = &mut node.kind
                                        {
                                            *value = false;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                    ],
                );
            }
            Condition::ItemKind { value } => {
                let mut chips = Vec::new();
                for choice in ItemKindFilter::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-kind:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::ItemKind { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::ProjectStatus { value } => {
                let mut chips = Vec::new();
                for choice in ProjectStatusFilter::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-projectstatus:{id}:{}", choice.label()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::ProjectStatus {
                                        value,
                                    }) = &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::Within {
                project,
                folder,
                include_children,
            } => {
                body = body.child(self.render_within_picker(
                    id,
                    project.as_ref(),
                    folder.as_ref(),
                    *include_children,
                    cx,
                ));
            }
            Condition::Search { .. } => {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child("Terms, separated by commas. Every term must match."),
                        )
                        .child(self.rule_text_input(0, "deep, work", cx)),
                );
            }
            Condition::Text { .. } => {
                body = body.child(self.rule_text_input(0, "Text to find", cx));
            }
            Condition::Estimate { .. } => {
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_xs().text_color(theme.text_faint).child("Minimum"))
                        .child(self.rule_text_input(0, "minutes", cx))
                        .child(div().text_xs().text_color(theme.text_faint).child("Maximum"))
                        .child(self.rule_text_input(1, "minutes", cx)),
                );
            }
            Condition::Flagged { value } => {
                body = chip_row(
                    body,
                    vec![
                        self.rule_chip(
                            format!("rule-flagged:{id}:yes"),
                            "Is flagged".to_string(),
                            *value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::Flagged { value }) =
                                            &mut node.kind
                                        {
                                            *value = true;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                        self.rule_chip(
                            format!("rule-flagged:{id}:no"),
                            "Is not flagged".to_string(),
                            !*value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(Condition::Flagged { value }) =
                                            &mut node.kind
                                        {
                                            *value = false;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                    ],
                );
            }
            Condition::HasRepeat { value } | Condition::HasEstimate { value } => {
                let (yes, no) = if matches!(condition, Condition::HasRepeat { .. }) {
                    ("Is repeating", "Is not repeating")
                } else {
                    ("Has an estimate", "Has no estimate")
                };
                body = chip_row(
                    body,
                    vec![
                        self.rule_chip(
                            format!("rule-bool:{id}:yes"),
                            yes.to_string(),
                            *value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| match &mut node.kind {
                                        RuleKind::Condition(Condition::HasRepeat { value })
                                        | RuleKind::Condition(Condition::HasEstimate { value }) => {
                                            *value = true
                                        }
                                        _ => {}
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                        self.rule_chip(
                            format!("rule-bool:{id}:no"),
                            no.to_string(),
                            !*value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| match &mut node.kind {
                                        RuleKind::Condition(Condition::HasRepeat { value })
                                        | RuleKind::Condition(Condition::HasEstimate { value }) => {
                                            *value = false
                                        }
                                        _ => {}
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                    ],
                );
            }
            Condition::FullscreenReminder { value } => {
                body = chip_row(
                    body,
                    vec![
                        self.rule_chip(
                            format!("rule-fullscreen:{id}:yes"),
                            "Has full-screen".to_string(),
                            *value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(
                                            Condition::FullscreenReminder { value },
                                        ) = &mut node.kind
                                        {
                                            *value = true;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                        self.rule_chip(
                            format!("rule-fullscreen:{id}:no"),
                            "No full-screen".to_string(),
                            !*value,
                            move |this, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(
                                            Condition::FullscreenReminder { value },
                                        ) = &mut node.kind
                                        {
                                            *value = false;
                                        }
                                    },
                                    cx,
                                )
                            },
                            cx,
                        ),
                    ],
                );
            }
            Condition::Priority { value } => {
                let mut chips = Vec::new();
                for choice in Priority::ALL {
                    chips.push(self.rule_chip(
                        format!("rule-priority:{id}:{}", choice.slug()),
                        choice.label().to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::Priority { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::Status { value } => {
                let mut chips = Vec::new();
                for choice in [
                    TaskStatus::Any,
                    TaskStatus::Available,
                    TaskStatus::Blocked,
                    TaskStatus::Deferred,
                    TaskStatus::Completed,
                ] {
                    chips.push(self.rule_chip(
                        format!("rule-taskstatus:{id}:{}", task_status_slug(choice)),
                        task_status_label(choice).to_string(),
                        *value == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| {
                                    if let RuleKind::Condition(Condition::Status { value }) =
                                        &mut node.kind
                                    {
                                        *value = choice;
                                    }
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::Due { window }
            | Condition::Deferred { window }
            | Condition::Planned { window } => {
                let mut chips = Vec::new();
                for choice in [
                    DueWindow::Overdue,
                    DueWindow::Today,
                    DueWindow::Tomorrow,
                    DueWindow::Next7Days,
                    DueWindow::Next30Days,
                    DueWindow::Later,
                    DueWindow::NoDate,
                ] {
                    chips.push(self.rule_chip(
                        format!("rule-window:{id}:{}", window_slug(choice)),
                        window_label(choice),
                        *window == choice,
                        move |this, cx| {
                            this.with_rule(
                                id,
                                |node| match &mut node.kind {
                                    RuleKind::Condition(Condition::Due { window })
                                    | RuleKind::Condition(Condition::Deferred { window })
                                    | RuleKind::Condition(Condition::Planned { window }) => {
                                        *window = choice
                                    }
                                    _ => {}
                                },
                                cx,
                            )
                        },
                        cx,
                    ));
                }
                body = chip_row(body, chips);
            }
            Condition::Project { id: project } => {
                let selected = vec![project.clone()];
                body = body.child(self.render_project_picker(id, &selected, false, cx));
            }
            Condition::Folder { id: folder, .. } => {
                let selected = vec![folder.clone()];
                body = body.child(self.render_folder_picker(id, &selected, false, cx));
            }
            Condition::Tag {
                id: tag,
                include_children,
            } => {
                let selected = vec![tag.clone()];
                body = body.child(self.render_tag_picker(id, &selected, *include_children, false, cx));
            }
            Condition::InInbox | Condition::InSingleActionsList => {
                body = body.child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("This rule has no options."),
                );
            }
        }

        body.into_any_element()
    }

    fn render_tag_picker(
        &self,
        id: u64,
        selected: &[TagId],
        include_children: bool,
        all: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let mut tags: Vec<_> = db.tags.values().collect();
        tags.sort_by_cached_key(|tag| db.tag_path(&tag.id).to_lowercase());
        let mut card = div()
            .id("rule-tag-picker")
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .flex()
            .flex_col()
            .gap_1()
            .max_h(px(170.))
            .overflow_y_scroll()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.clear);
        for tag in tags {
            let tag_id = tag.id.clone();
            let active = selected.contains(&tag_id);
            let label = format!("#{}", db.tag_path(&tag_id));
            card = card.child(
                div()
                    .id(super::sid(format!("rule-tag:{id}:{tag_id}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .bg(if active { theme.selection } else { theme.clear })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(view_checkbox(theme, active))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let target = tag_id.clone();
                        this.with_rule(
                            id,
                            move |node| {
                                if let RuleKind::Condition(condition) = &mut node.kind {
                                    match condition {
                                        Condition::TagsAny { ids, .. }
                                        | Condition::TagsAll { ids, .. } => {
                                            if let Some(index) =
                                                ids.iter().position(|id| id == &target)
                                            {
                                                ids.remove(index);
                                            } else {
                                                ids.push(target);
                                            }
                                        }
                                        Condition::Tag { id, .. } => *id = target,
                                        _ => {}
                                    }
                                }
                            },
                            cx,
                        )
                    })),
            );
        }
        if matches!(
            self.modal.as_ref(),
            Some(Modal::Perspective { form })
                if form
                    .rule(id)
                    .and_then(RuleNode::condition_ref)
                    .is_some_and(|condition| matches!(condition, Condition::TagsAny { .. } | Condition::TagsAll { .. }))
        ) {
            let labels = if all {
                ("Child tags", "Match every selected tag")
            } else {
                ("Child tags", "Match a child of a selected tag")
            };
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(super::sid(format!("rule-tag-children:{id}")))
                            .child(widgets::toggle_chip(theme, labels.0, include_children))
                            .flow_tip(self, cx, theme, labels.1)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.with_rule(
                                    id,
                                    |node| {
                                        if let RuleKind::Condition(condition) = &mut node.kind {
                                            match condition {
                                                Condition::TagsAny {
                                                    include_children, ..
                                                }
                                                | Condition::TagsAll {
                                                    include_children, ..
                                                } => *include_children = !*include_children,
                                                _ => {}
                                            }
                                        }
                                    },
                                    cx,
                                )
                            })),
                    ),
            );
        }
        card.into_any_element()
    }

    fn render_project_picker(
        &self,
        id: u64,
        selected: &[ProjectId],
        multiple: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let mut projects: Vec<_> = db.projects.values().collect();
        projects.sort_by_cached_key(|project| db.project_path(&project.id).to_lowercase());
        let mut card = div()
            .id("rule-project-picker")
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .flex()
            .flex_col()
            .gap_1()
            .max_h(px(170.))
            .overflow_y_scroll()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft);
        if selected.is_empty() {
            card = card.child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No project selected"),
            );
        }
        for project in projects {
            let project_id = project.id.clone();
            let active = selected.contains(&project_id);
            card = card.child(
                div()
                    .id(super::sid(format!("rule-project:{id}:{project_id}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .bg(if active { theme.selection } else { theme.clear })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(view_checkbox(theme, active))
                    .child(db.project_path(&project_id))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let target = project_id.clone();
                        this.with_rule(
                            id,
                            move |node| {
                                if let RuleKind::Condition(condition) = &mut node.kind {
                                    match condition {
                                        Condition::Project { id } => *id = target,
                                        Condition::Within { project, folder, .. } => {
                                            if project.as_ref() == Some(&target) {
                                                *project = None;
                                            } else {
                                                *project = Some(target);
                                                *folder = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            },
                            cx,
                        )
                    })),
            );
        }
        let _ = multiple;
        card.into_any_element()
    }

    fn render_folder_picker(
        &self,
        id: u64,
        selected: &[FolderId],
        multiple: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let mut folders: Vec<_> = db.folders.values().collect();
        folders.sort_by_cached_key(|folder| db.folder_path(&folder.id).to_lowercase());
        let mut card = div()
            .id("rule-folder-picker")
                .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .flex()
            .flex_col()
            .gap_1()
            .max_h(px(150.))
            .overflow_y_scroll()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft);
        if selected.is_empty() {
            card = card.child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No folder selected"),
            );
        }
        for folder in folders {
            let folder_id = folder.id.clone();
            let active = selected.contains(&folder_id);
            card = card.child(
                div()
                    .id(super::sid(format!("rule-folder:{id}:{folder_id}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .bg(if active { theme.selection } else { theme.clear })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(view_checkbox(theme, active))
                    .child(db.folder_path(&folder_id))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let target = folder_id.clone();
                        this.with_rule(
                            id,
                            move |node| {
                                if let RuleKind::Condition(condition) = &mut node.kind {
                                    match condition {
                                        Condition::Folder { id, .. } => *id = target,
                                        Condition::Within { project, folder, .. } => {
                                            if folder.as_ref() == Some(&target) {
                                                *folder = None;
                                            } else {
                                                *folder = Some(target);
                                                *project = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            },
                            cx,
                        )
                    })),
            );
        }
        let _ = multiple;
        card.into_any_element()
    }

    fn render_within_picker(
        &self,
        id: u64,
        project: Option<&ProjectId>,
        folder: Option<&FolderId>,
        include_children: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let projects = project.cloned().into_iter().collect::<Vec<_>>();
        let folders = folder.cloned().into_iter().collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Projects"),
            )
            .child(self.render_project_picker(id, &projects, false, cx))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Folders"),
            )
            .child(self.render_folder_picker(id, &folders, false, cx))
            .child(
                div()
                    .id(super::sid(format!("rule-within-children:{id}")))
                    .child(widgets::toggle_chip(theme, "Child folders", include_children))
                    .flow_tip(self, cx, theme, "Include projects in nested folders")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.with_rule(
                            id,
                            |node| {
                                if let RuleKind::Condition(Condition::Within {
                                    include_children, ..
                                }) = &mut node.kind
                                {
                                    *include_children = !*include_children;
                                }
                            },
                            cx,
                        )
                    })),
            )
            .into_any_element()
    }

    fn render_tag_location_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::TagLocation { tag, .. }) = &self.modal else {
            return div();
        };
        let tag = tag.clone();
        div().flex().items_center().gap_2().child(
            div()
                .id("modal-clear-location")
                .child(widgets::button(theme, "Clear location", false))
                .flow_tip(self, cx, theme, "Remove coordinates from this tag")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.execute(flowlist_core::command::Command::UpdateTag {
                        id: tag.clone(),
                        patch: flowlist_core::command::TagPatch {
                            location: Some(None),
                            ..Default::default()
                        },
                    });
                    this.close_modal(cx);
                })),
        )
    }

    fn render_new_project_icon(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let (icon, icon_open, icon_tab) = match &self.modal {
            Some(Modal::NewProject {
                icon,
                icon_open,
                icon_tab,
                ..
            }) => (icon.clone(), *icon_open, *icon_tab),
            _ => (IconSpec::default(), false, IconTab::Icons),
        };
        let icon_label = match &icon {
            IconSpec::Builtin(spec) => spec.label().to_string(),
            IconSpec::Emoji(_) => String::new(),
        };
        let icon_tip = match &icon {
            IconSpec::Builtin(spec) => format!("Icon · {}", spec.label()),
            IconSpec::Emoji(_) => "Change the icon".to_string(),
        };
        let icon_row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child("Icon"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("new-project-icon-toggle".to_string()),
                            )
                            .id("new-project-icon-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(if icon_open {
                                theme.accent
                            } else {
                                theme.border_soft
                            })
                            .bg(if icon_open {
                                theme.selection
                            } else {
                                theme.clear
                            })
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, icon_tip.clone())
                            .hover(|style| style.bg(theme.hover))
                            .child(widgets::icon_spec(&icon, 14.0, theme.text_dim))
                            .child(icons::icon(
                                super::icons::Icon::ChevronDown,
                                10.0,
                                theme.text_faint,
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::NewProject { icon_open, .. }) = &mut this.modal {
                                    *icon_open = !*icon_open;
                                    if *icon_open {
                                        this.modal_focus = 6;
                                    }
                                }
                                cx.notify();
                            })),
                    )
                    .when(!icon_label.is_empty(), |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(icon_label),
                        )
                    }),
            )
            .when(icon_open, |this| {
                this.child(self.render_icon_picker_card(
                    IconTarget::NewProject,
                    6,
                    Some(&icon),
                    icon_tab,
                    cx,
                ))
            });

        div().w_full().child(icon_row).into_any_element()
    }

    fn render_new_project_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let (kind, fullscreen) = match &self.modal {
            Some(Modal::NewProject {
                kind,
                fullscreen_reminder,
                ..
            }) => (kind.clone(), *fullscreen_reminder),
            _ => (ProjectKind::Parallel, false),
        };
        let mut row = div().flex().flex_col().gap_1().child(
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child("Project type"),
        );
        let mut chips = div().flex().items_center().gap_1();
        for candidate in ProjectKind::ALL {
            let active = kind == candidate;
            chips = chips.child(
                div()
                    .id(super::sid(format!("project-kind:{}", candidate.slug())))
                    .cursor_pointer()
                    .child(widgets::toggle_chip(theme, candidate.label(), active))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::NewProject { kind, .. }) = &mut this.modal {
                            *kind = candidate;
                        }
                        cx.notify();
                    })),
            );
        }
        row = row.child(chips).child(
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child(kind.hint()),
        );
        row.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(div().text_sm().child("Full-screen reminder"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child("Take over the screen when a task in this project is due."),
                        ),
                )
                .child(
                    div()
                        .id("new-project-fullscreen")
                        .child(widgets::switch(theme, fullscreen))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(Modal::NewProject {
                                fullscreen_reminder,
                                ..
                            }) = &mut this.modal
                            {
                                *fullscreen_reminder = !*fullscreen_reminder;
                            }
                            cx.notify();
                        })),
                ),
        )
    }

    fn render_new_folder_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        if matches!(
            &self.modal,
            Some(Modal::NewFolder {
                editing: Some(_),
                ..
            })
        ) {
            return div();
        }
        let fullscreen = matches!(
            &self.modal,
            Some(Modal::NewFolder {
                fullscreen_reminder: true,
                ..
            })
        );
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(div().text_sm().child("Full-screen reminder"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Take over the screen when a task in this folder is due."),
                    ),
            )
            .child(
                div()
                    .id("new-folder-fullscreen")
                    .child(widgets::switch(theme, fullscreen))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::NewFolder {
                            fullscreen_reminder,
                            ..
                        }) = &mut this.modal
                        {
                            *fullscreen_reminder = !*fullscreen_reminder;
                        }
                        cx.notify();
                    })),
            )
    }

    /// The Settings sheet: a fixed section rail on the left and one section at
    /// a time on the right. More library-wide preferences can land as new rail
    /// items without lengthening a single nameless scroll.
    fn render_settings_sheet(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Settings { section, .. }) = &self.modal else {
            return div().into_any_element();
        };
        let section = *section;
        let mut rail = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .w(px(168.))
            .flex_shrink_0()
            .pr_3()
            .border_r_1()
            .border_color(theme.border_soft);
        for item in SettingsSection::ALL {
            rail = rail.child(self.settings_nav_row(item, section, cx));
        }
        let content = match section {
            SettingsSection::Appearance => self.render_settings_controls(cx),
            SettingsSection::Sidebar => self.render_sidebar_settings(cx).into_any_element(),
            SettingsSection::Workflow => self.render_workflow_settings(cx),
            SettingsSection::Reminders => self.render_reminder_settings(cx),
            SettingsSection::Calendar => self.render_calendar_settings(cx),
            SettingsSection::Nearby => self.render_nearby_settings(cx),
            SettingsSection::Trash => self.render_trash_settings(cx),
            SettingsSection::Mobile => self.render_mobile_settings(cx),
            SettingsSection::About => self.render_about_settings(cx),
        };
        // A section change fades its content in, matching the sheet's own
        // 120 ms entrance. The id carries the section so the animation
        // restarts on every switch rather than only on the first open.
        let content = div().child(content).with_animation(
            gpui::ElementId::Name(format!("settings-pane-{}", section.slug()).into()),
            Animation::new(Duration::from_millis(120)),
            |content, delta| content.opacity(delta),
        );
        // The rail sits outside the scroll region so choosing a section never
        // moves it. Only the pane scrolls, with its own scrollbar at the sheet
        // edge and breathing room before the content.
        let pane = div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .pl_3()
            .child(self.settings_section_header(section))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("settings-scroll")
                            .flex()
                            .flex_col()
                            .gap_3()
                            .flex_1()
                            .min_h_0()
                            .pr(px(14.))
                            .overflow_y_scroll()
                            .track_scroll(&self.modal_scroll)
                            .child(content),
                    )
                    .child(self.modal_scrollbar(theme, cx)),
            );
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(rail)
            .child(pane)
            .into_any_element()
    }

    fn settings_nav_row(
        &self,
        item: SettingsSection,
        section: SettingsSection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let active = section == item;
        let boxes = self.settings_toggle_boxes.clone();
        let key = format!("settings-section-{}", item.slug());
        let probe_key = key.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        probe_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(
                div()
                    .id(super::sid(key.clone()))
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(30.))
                    .px_2()
                    .rounded_md()
                    .bg(if active { theme.selection } else { theme.clear })
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(icons::icon(
                        item.icon(),
                        14.0,
                        if active {
                            theme.accent
                        } else {
                            theme.text_faint
                        },
                    ))
                    .child(div().text_sm().child(item.label()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Settings { section, .. }) = &mut this.modal {
                            *section = item;
                        }
                        this.modal_focus = item.fields().first().copied().unwrap_or(0);
                        this.settings_toggle_boxes.borrow_mut().clear();
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn settings_section_header(&self, section: SettingsSection) -> AnyElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(section.label()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(section.description()),
            )
            .into_any_element()
    }

    /// One bordered group inside a section. Grouping is the main defence
    /// against a settings sheet turning into a wall of controls.
    /// The destructive card at the end of Settings. It looks like every other
    /// card except the heading and border are red, so removal is visibly
    /// separate from ordinary preferences.
    pub(super) fn danger_card(
        &self,
        title: &'static str,
        hint: Option<&'static str>,
        body: impl IntoElement,
    ) -> AnyElement {
        let theme = self.theme;
        let mut card = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.danger)
            .bg(theme.field)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.danger)
                    .child(title),
            )
            .child(body);
        if let Some(hint) = hint {
            card = card.child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(theme.text_faint)
                    .child(hint),
            );
        }
        card.into_any_element()
    }

    pub(super) fn settings_card(
        &self,
        title: &'static str,
        hint: Option<&'static str>,
        body: impl IntoElement,
    ) -> AnyElement {
        let theme = self.theme;
        let mut card = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child(title),
            )
            .child(body);
        if let Some(hint) = hint {
            card = card.child(div().text_xs().text_color(theme.text_faint).child(hint));
        }
        card.into_any_element()
    }

    fn choose_app_wallpaper(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        let slot = match appearance {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        };
        let separate = matches!(
            &self.modal,
            Some(Modal::Settings {
                wallpaper_separate: true,
                ..
            })
        );
        let prompt = if separate {
            format!("Choose {slot} background")
        } else {
            "Choose background".to_string()
        };
        let picked = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(prompt.into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            this.update(cx, |this, cx| {
                match super::background::install_wallpaper(
                    &path,
                    &this.store.paths().root,
                    &format!("app-{slot}"),
                ) {
                    Ok(copied) => {
                        if let Some(Modal::Settings {
                            wallpaper_light,
                            wallpaper_dark,
                            wallpaper_separate,
                            use_wallpaper,
                            wallpaper_note,
                            ..
                        }) = &mut this.modal
                        {
                            match appearance {
                                Appearance::Light => *wallpaper_light = Some(copied),
                                Appearance::Dark => *wallpaper_dark = Some(copied),
                            }
                            *use_wallpaper = true;
                            *wallpaper_note = Some(if *wallpaper_separate {
                                format!(
                                    "{} background selected · Save to apply",
                                    appearance.label()
                                )
                            } else {
                                "Background selected · Save to apply".to_string()
                            });
                        }
                    }
                    Err(error) => {
                        if let Some(Modal::Settings { wallpaper_note, .. }) = &mut this.modal {
                            *wallpaper_note = Some(error.to_string());
                        }
                        this.toast = Some(error.to_string());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn choose_reminder_wallpaper(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        let slot = match appearance {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        };
        let separate = matches!(
            &self.modal,
            Some(Modal::Settings {
                reminder_wallpaper_separate: true,
                ..
            })
        );
        let prompt = if separate {
            format!("Choose {slot} reminder background")
        } else {
            "Choose reminder background".to_string()
        };
        let picked = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(prompt.into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            this.update(cx, |this, cx| {
                match super::background::install_wallpaper(
                    &path,
                    &this.store.paths().root,
                    &format!("reminder-{slot}"),
                ) {
                    Ok(copied) => {
                        if let Some(Modal::Settings {
                            reminder_wallpaper_light,
                            reminder_wallpaper_dark,
                            reminder_wallpaper_separate,
                            reminder_use_wallpaper,
                            reminder_match_theme,
                            reminder_wallpaper_note,
                            ..
                        }) = &mut this.modal
                        {
                            match appearance {
                                Appearance::Light => *reminder_wallpaper_light = Some(copied),
                                Appearance::Dark => *reminder_wallpaper_dark = Some(copied),
                            }
                            *reminder_use_wallpaper = true;
                            *reminder_match_theme = false;
                            *reminder_wallpaper_note = Some(if *reminder_wallpaper_separate {
                                format!(
                                    "{} reminder background selected · Save to apply",
                                    appearance.label()
                                )
                            } else {
                                "Reminder background selected · Save to apply".to_string()
                            });
                        }
                    }
                    Err(error) => {
                        if let Some(Modal::Settings {
                            reminder_wallpaper_note,
                            ..
                        }) = &mut this.modal
                        {
                            *reminder_wallpaper_note = Some(error.to_string());
                        }
                        this.toast = Some(error.to_string());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn render_settings_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let (selected, appearance) = match &self.modal {
            Some(Modal::Settings {
                theme, appearance, ..
            }) => (*theme, *appearance),
            _ => (theme.kind(), theme.appearance()),
        };
        let follow_system = matches!(
            &self.modal,
            Some(Modal::Settings {
                follow_system: true,
                ..
            })
        );
        let traffic_lights = matches!(
            &self.modal,
            Some(Modal::Settings {
                traffic_lights: true,
                ..
            })
        );
        let (
            app_wallpaper_light,
            app_wallpaper_dark,
            app_wallpaper_separate,
            use_app_wallpaper,
            app_wallpaper_opacity,
            app_wallpaper_note,
        ) = match &self.modal {
            Some(Modal::Settings {
                wallpaper_light,
                wallpaper_dark,
                wallpaper_separate,
                use_wallpaper,
                wallpaper_opacity,
                wallpaper_note,
                ..
            }) => (
                wallpaper_light.clone(),
                wallpaper_dark.clone(),
                *wallpaper_separate,
                *use_wallpaper,
                *wallpaper_opacity,
                wallpaper_note.clone(),
            ),
            _ => (None, None, false, false, 0.35, None),
        };
        let has_app_wallpaper = app_wallpaper_light.is_some()
            || (app_wallpaper_separate && app_wallpaper_dark.is_some());
        let wallpaper_pick_mode = if app_wallpaper_separate {
            appearance
        } else {
            Appearance::Light
        };

        let mut modes = div()
            .flex()
            .items_center()
            .gap_1()
            .p_0p5()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field);
        for mode in Appearance::ALL {
            let active = appearance == mode;
            modes = modes.child(
                div()
                    .on_children_prepainted(self.theme_probe(format!("appearance:{}", mode.slug())))
                    .child(
                        div()
                            .id(mode.slug())
                            .h(px(24.))
                            .px_3()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(if active { theme.raised } else { theme.clear })
                            .text_color(if active { theme.text } else { theme.text_dim })
                            .text_xs()
                            .when(active, |this| this.font_weight(gpui::FontWeight::SEMIBOLD))
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .child(mode.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(Modal::Settings {
                                    appearance,
                                    follow_system,
                                    ..
                                }) = &mut this.modal
                                {
                                    // Picking a mode is a deliberate override,
                                    // so it stops following macOS rather than
                                    // saving a choice the window ignores.
                                    *appearance = mode;
                                    *follow_system = false;
                                }
                                let preview = match &this.modal {
                                    Some(Modal::Settings {
                                        theme, appearance, ..
                                    }) => Some((*theme, *appearance)),
                                    _ => None,
                                };
                                if let Some((kind, mode)) = preview {
                                    // Preview the choice right away; Cancel
                                    // restores the theme the sheet opened with.
                                    this.theme = this.preview_theme(kind, mode);
                                }
                                cx.notify();
                            })),
                    ),
            );
        }

        let mut grid = div().flex().flex_col().gap_2().w_full();
        for chunk in ThemeKind::ALL.chunks(4) {
            let mut row = div().flex().items_start().gap_2().w_full();
            for kind in chunk {
                row = row.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .on_children_prepainted(self.theme_probe(format!("theme:{}", kind.slug())))
                        .child(self.theme_swatch(*kind, selected == *kind, appearance, cx)),
                );
            }
            for _ in chunk.len()..4 {
                row = row.child(div().flex_1());
            }
            grid = grid.child(row);
        }

        let follow = div().flex().items_center().gap_2().child(
            div()
                .id("modal-follow-system")
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.text)
                        .child("Follow system appearance"),
                )
                .child(widgets::switch(theme, follow_system))
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Settings { follow_system, .. }) = &mut this.modal {
                        *follow_system = !*follow_system;
                    }
                    cx.notify();
                })),
        );

        let theme_card = self.settings_card(
            "Theme",
            Some("Live preview. Cancel restores the theme and accent the sheet opened with."),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child("Light or dark"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(follow)
                                .child(modes),
                        ),
                )
                .child(grid)
                .child(div().text_xs().text_color(theme.text_faint).child(format!(
                    "{} · {} · ⌘T light/dark · ⇧⌘T next family",
                    selected.hint(),
                    appearance.label(),
                ))),
        );

        let icon_card = self.settings_card(
            "Dock icon",
            Some("Changes the icon in the Dock and app switcher. Finder keeps the bundle artwork."),
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(AppIcon::ALL.into_iter().map(|icon| {
                    let selected = self.app_icon == icon;
                    div()
                        .id(gpui::ElementId::Name(
                            format!("dock-icon-{}", icon.slug()).into(),
                        ))
                        .size(px(26.))
                        .rounded_md()
                        .bg(rgb(icon.color()))
                        .border_2()
                        .border_color(if selected { theme.text } else { theme.border })
                        .cursor_pointer()
                        .hover(|style| style.opacity(0.9))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.app_icon = icon;
                            app_icon::apply(icon);
                            this.save_session();
                            this.toast = Some(format!("{} Dock icon", icon.label()));
                            cx.notify();
                        }))
                })),
        );

        let accent = if selected == ThemeKind::Plain {
            self.render_plain_accent(cx)
        } else {
            self.settings_card(
                "Accent",
                Some("Accent colours are a Plain feature. This family carries its own palette."),
                div(),
            )
        };

        let mut background_body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(div().text_sm().child("Use custom background"))
                            .child(
                                div()
                                    .text_xs()
                                    .whitespace_normal()
                                    .text_color(theme.text_faint)
                                    .child("A photo behind the window chrome and list."),
                            ),
                    )
                    .child(
                        div()
                            .id("settings-use-wallpaper")
                            .child(widgets::switch(theme, use_app_wallpaper))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !has_app_wallpaper {
                                    this.choose_app_wallpaper(wallpaper_pick_mode, cx);
                                } else if let Some(Modal::Settings { use_wallpaper, .. }) =
                                    &mut this.modal
                                {
                                    *use_wallpaper = !*use_wallpaper;
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .when(use_app_wallpaper, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(div().text_sm().child(
                                    "Use separate backgrounds for light and dark modes",
                                ))
                                .child(
                                    div()
                                        .text_xs()
                                        .whitespace_normal()
                                        .text_color(theme.text_faint)
                                        .child(
                                            "Off keeps one background for both appearances. On shows a picker for each.",
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe("wallpaper-separate-toggle".to_string()),
                                )
                                .id("wallpaper-separate-toggle")
                                .child(widgets::switch(theme, app_wallpaper_separate))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        wallpaper_separate,
                                        wallpaper_light,
                                        wallpaper_dark,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *wallpaper_separate = !*wallpaper_separate;
                                        if *wallpaper_separate && wallpaper_dark.is_none() {
                                            *wallpaper_dark = wallpaper_light.clone();
                                        }
                                        if !*wallpaper_separate && wallpaper_light.is_none() {
                                            *wallpaper_light = wallpaper_dark.clone();
                                        }
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            });
        let wallpaper_modes: &[Appearance] = if app_wallpaper_separate {
            &[Appearance::Light, Appearance::Dark]
        } else {
            &[Appearance::Light]
        };
        for &mode in wallpaper_modes {
            let path = match mode {
                Appearance::Light => app_wallpaper_light.clone(),
                Appearance::Dark => app_wallpaper_dark.clone(),
            };
            let choose_id = match mode {
                Appearance::Light => "settings-choose-wallpaper",
                Appearance::Dark => "settings-choose-wallpaper-dark",
            };
            let choose_probe = match mode {
                Appearance::Light => "settings-choose-wallpaper".to_string(),
                Appearance::Dark => "settings-choose-wallpaper-dark".to_string(),
            };
            let remove_id = match mode {
                Appearance::Light => "settings-remove-wallpaper",
                Appearance::Dark => "settings-remove-wallpaper-dark",
            };
            let title = if app_wallpaper_separate {
                format!("{} background", mode.label())
            } else {
                "Background".to_string()
            };
            let hint = if app_wallpaper_separate {
                format!("Shown while the app is {}.", mode.label().to_lowercase())
            } else {
                "Shown in both light and dark modes.".to_string()
            };
            let on_screen = app_wallpaper_separate && appearance == mode;
            let mut section = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(div().text_sm().child(title))
                                .child(div().text_xs().text_color(theme.text_faint).child(hint)),
                        )
                        .when(on_screen, |this| {
                            this.child(div().text_xs().text_color(theme.accent).child("On screen"))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .on_children_prepainted(self.theme_probe(choose_probe))
                                .id(choose_id)
                                .child(widgets::button(
                                    theme,
                                    if path.is_some() {
                                        "Replace background"
                                    } else {
                                        "Choose background"
                                    },
                                    false,
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_app_wallpaper(mode, cx);
                                })),
                        )
                        .when(path.is_some(), |this| {
                            this.child(
                                div()
                                    .id(remove_id)
                                    .child(widgets::button(theme, "Remove", false))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(Modal::Settings {
                                            wallpaper_light,
                                            wallpaper_dark,
                                            use_wallpaper,
                                            ..
                                        }) = &mut this.modal
                                        {
                                            match mode {
                                                Appearance::Light => *wallpaper_light = None,
                                                Appearance::Dark => *wallpaper_dark = None,
                                            }
                                            if wallpaper_light.is_none() && wallpaper_dark.is_none()
                                            {
                                                *use_wallpaper = false;
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                        }),
                );
            if let Some(path) = &path {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Background".to_string());
                section = section.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            img(path.clone())
                                .object_fit(ObjectFit::Cover)
                                .w(px(72.))
                                .h(px(48.))
                                .rounded_md()
                                .opacity(app_wallpaper_opacity),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(name)),
                );
            }
            if use_app_wallpaper {
                background_body = background_body.child(section);
            }
        }
        if use_app_wallpaper && has_app_wallpaper {
            let mut opacity_row = div().flex().items_center().gap_1();
            for percent in [20, 35, 50, 70, 90] {
                let value = percent as f32 / 100.0;
                let active = (app_wallpaper_opacity - value).abs() < 0.005;
                let key = format!("app-wallpaper-opacity:{percent}");
                opacity_row = opacity_row.child(
                    div()
                        .id(super::sid(key))
                        .cursor_pointer()
                        .child(widgets::toggle_chip(theme, format!("{percent}%"), active))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(Modal::Settings {
                                wallpaper_opacity, ..
                            }) = &mut this.modal
                            {
                                *wallpaper_opacity = value;
                            }
                            cx.notify();
                        })),
                );
            }
            background_body = background_body
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("Background opacity"),
                )
                .child(opacity_row);
        }
        if use_app_wallpaper && let Some(note) = &app_wallpaper_note {
            let success = {
                let lower = note.to_lowercase();
                lower.contains("saved") || lower.contains("selected")
            };
            background_body = background_body.child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(if success { theme.success } else { theme.danger })
                    .child(note.clone()),
            );
        }
        let app_background_card = self.settings_card(
            "Background",
            Some("A custom background behind the main window, with its own opacity."),
            background_body,
        );
        let boxes = self.settings_toggle_boxes.clone();
        // One slim row rather than a full card: the toggle belongs with the
        // window's appearance, but it does not need its own heading.
        let window_card = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().text_sm().child("Window controls"))
                            .child(div().text_xs().text_color(theme.text_faint).child(
                                if traffic_lights {
                                    "· shown in the header"
                                } else {
                                    "· hidden"
                                },
                            )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Close, minimise and zoom in the header. W toggles them."),
                    ),
            )
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            boxes.borrow_mut().insert(
                                "settings-window-controls".to_string(),
                                [
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ],
                            );
                        }
                    })
                    .id("settings-window-controls")
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .child(widgets::switch(theme, traffic_lights))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Settings { traffic_lights, .. }) = &mut this.modal {
                            *traffic_lights = !*traffic_lights;
                            // Preview the choice on the window; Cancel
                            // restores the value the sheet opened with.
                            this.traffic_lights_visible = *traffic_lights;
                        }
                        cx.notify();
                    })),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(theme_card)
            .child(icon_card)
            .child(accent)
            .child(app_background_card)
            .child(window_card)
            .into_any_element()
    }

    /// Plain's accent row. Presets carry a tone for each mode; the picker edits
    /// the appearance that is currently on screen, so a custom colour can be
    /// tuned separately for light and dark.
    fn render_plain_accent(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Settings {
            appearance,
            accent_light,
            accent_dark,
            ..
        }) = &self.modal
        else {
            return div().into_any_element();
        };
        let appearance = *appearance;
        // An accent set for one appearance is inherited by the other until it
        // gets a tone of its own, so the swatch and hex must show the colour
        // the window is actually painting, not a reset to ink.
        let current = super::accent_for(appearance, *accent_light, *accent_dark);
        // With no custom tone the accent is Plain's ink, and the row has to
        // say so: the swatch, hex and marker all follow the painted colour.
        let ink = match appearance {
            Appearance::Light => 0x181818,
            Appearance::Dark => 0xF2F2F2,
        };
        let effective = current.unwrap_or(ink);
        let resolved = theme::readable_plain_accent(effective, appearance);
        let (hue, saturation, value) = theme::rgb_to_hsv(effective);
        // A neutral ink has no hue to show; park the strip on a vivid blue so
        // the first drag in the square yields a colour rather than grey.
        let hue = if saturation < 0.08 { 0.60 } else { hue };
        let hue_color = theme::hsv_to_rgb(hue, 1.0, 1.0);
        let preview = div()
            .size(px(24.))
            .rounded_full()
            .border_1()
            .border_color(theme.border)
            .bg(gpui::rgb(resolved));

        let mut presets = div().flex().flex_wrap().items_center().gap_2();
        for preset in PLAIN_ACCENTS {
            let active = effective == preset.color(appearance);
            let slug = preset.name.to_lowercase();
            presets = presets.child(
                div()
                    .on_children_prepainted(self.theme_probe(format!("accent:{slug}")))
                    .child(
                        div()
                            .id(super::sid(format!("accent-{slug}")))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(24.))
                            .rounded_full()
                            .border_2()
                            .border_color(if active { theme.text } else { theme.border })
                            .bg(gpui::rgb(preset.color(appearance)))
                            .cursor_pointer()
                            .hover(|style| style.border_color(theme.accent))
                            .flow_tip(self, cx, theme, preset.name)
                            .when(active, |this| {
                                let ink = if appearance == Appearance::Light {
                                    theme.text
                                } else {
                                    theme.bg
                                };
                                this.child(icons::icon(Icon::Check, 11.0, ink))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_plain_accents(Some(preset.light), Some(preset.dark), cx);
                            })),
                    ),
            );
        }

        // Saturation/value square. The marker is positioned from the derived
        // HSV, so presets, swatches and the picker always point at one colour.
        const SV_W: f32 = 176.0;
        const SV_H: f32 = 84.0;
        const HUE_H: f32 = 12.0;
        let sv_bounds: Rc<RefCell<Option<Bounds<gpui::Pixels>>>> = Rc::new(RefCell::new(None));
        let sv_boxes = self.theme_boxes.clone();
        let sv_probe = sv_bounds.clone();
        let sv_down = sv_bounds.clone();
        let sv_move = sv_bounds.clone();
        let sv_square = div()
            .relative()
            .w(px(SV_W))
            .h(px(SV_H))
            .rounded_md()
            .overflow_hidden()
            .cursor(CursorStyle::Crosshair)
            .bg(gpui::rgb(hue_color))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(gpui::linear_gradient(
                        90.,
                        gpui::linear_color_stop(gpui::white(), 0.),
                        gpui::linear_color_stop(gpui::transparent_black(), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(gpui::linear_gradient(
                        0.,
                        gpui::linear_color_stop(gpui::transparent_black(), 0.),
                        gpui::linear_color_stop(gpui::black(), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .left(px(saturation * SV_W - 5.0))
                    .top(px((1.0 - value) * SV_H - 5.0))
                    .size(px(10.))
                    .rounded_full()
                    .border_2()
                    .border_color(gpui::white())
                    .shadow_lg(),
            )
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let Some(bounds) = *sv_down.borrow() else {
                        return;
                    };
                    let x =
                        ((event.position.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
                    let y =
                        ((event.position.y - bounds.origin.y) / bounds.size.height).clamp(0.0, 1.0);
                    this.set_plain_accent(appearance, Some(theme::hsv_to_rgb(hue, x, 1.0 - y)), cx);
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                    if event.pressed_button != Some(gpui::MouseButton::Left) {
                        return;
                    }
                    let Some(bounds) = *sv_move.borrow() else {
                        return;
                    };
                    let x =
                        ((event.position.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
                    let y =
                        ((event.position.y - bounds.origin.y) / bounds.size.height).clamp(0.0, 1.0);
                    this.set_plain_accent(appearance, Some(theme::hsv_to_rgb(hue, x, 1.0 - y)), cx);
                }),
            );
        let sv = div()
            .w(px(SV_W))
            .h(px(SV_H))
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    *sv_probe.borrow_mut() = Some(*bounds);
                    sv_boxes.borrow_mut().insert(
                        "accent-sv".to_string(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(sv_square);

        // Hue strip: six fixed-width gradients make a continuous wheel without
        // a shader, and the marker is a thin white line over the chosen hue.
        let hue_bounds: Rc<RefCell<Option<Bounds<gpui::Pixels>>>> = Rc::new(RefCell::new(None));
        let hue_boxes = self.theme_boxes.clone();
        let hue_probe = hue_bounds.clone();
        let hue_down = hue_bounds.clone();
        let hue_move = hue_bounds.clone();
        let segment = (SV_W / 6.0).ceil();
        let mut hue_bar = div()
            .relative()
            .flex()
            .w(px(SV_W))
            .h(px(HUE_H))
            .rounded_full()
            .overflow_hidden()
            .cursor(CursorStyle::Crosshair);
        for step in 0..6 {
            let from = theme::hsv_to_rgb(step as f32 / 6.0, 1.0, 1.0);
            let to = theme::hsv_to_rgb((step + 1) as f32 / 6.0, 1.0, 1.0);
            hue_bar = hue_bar.child(div().w(px(segment)).h(px(HUE_H)).flex_shrink_0().bg(
                gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(gpui::rgb(from), 0.),
                    gpui::linear_color_stop(gpui::rgb(to), 1.),
                ),
            ));
        }
        let hue_bar = hue_bar
            .child(
                div()
                    .absolute()
                    .left(px(hue * SV_W - 2.0))
                    .top_0()
                    .w(px(4.))
                    .h(px(HUE_H))
                    .rounded_full()
                    .border_1()
                    .border_color(gpui::white())
                    .bg(gpui::transparent_black()),
            )
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let Some(bounds) = *hue_down.borrow() else {
                        return;
                    };
                    let x =
                        ((event.position.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
                    this.set_plain_accent(
                        appearance,
                        Some(theme::hsv_to_rgb(x, saturation.max(0.55), value.max(0.55))),
                        cx,
                    );
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                    if event.pressed_button != Some(gpui::MouseButton::Left) {
                        return;
                    }
                    let Some(bounds) = *hue_move.borrow() else {
                        return;
                    };
                    let x =
                        ((event.position.x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
                    this.set_plain_accent(
                        appearance,
                        Some(theme::hsv_to_rgb(x, saturation.max(0.55), value.max(0.55))),
                        cx,
                    );
                }),
            );
        let hue_bar = div()
            .w(px(SV_W))
            .h(px(HUE_H))
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    *hue_probe.borrow_mut() = Some(*bounds);
                    hue_boxes.borrow_mut().insert(
                        "accent-hue".to_string(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(hue_bar);

        let picker = div().flex().flex_col().gap_2().child(sv).child(hue_bar);

        let note = match &self.modal {
            Some(Modal::Settings { accent_note, .. }) => accent_note.clone(),
            _ => None,
        };
        let meta = div()
            .flex()
            .items_center()
            .gap_2()
            .child(preview)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text)
                    .child(theme::hex(resolved)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(format!("Accent · {}", appearance.label())),
            );
        let actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("accent-pick".to_string()))
                    .child(
                        div()
                            .id(super::sid("accent-pick"))
                            .child(widgets::button_with_icon(
                                theme,
                                Icon::Eyedropper,
                                "Pick from screen",
                                false,
                            ))
                            .flow_tip(self, cx, theme, "Sample any pixel — background included")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pick_accent_from_screen(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .on_children_prepainted(self.theme_probe("accent-reset".to_string()))
                    .child(
                        div()
                            .id(super::sid("accent-reset"))
                            .child(widgets::button(theme, "Reset to ink", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_plain_accents(None, None, cx);
                            })),
                    ),
            );

        self.settings_card(
            "Accent",
            Some(
                "Presets set light and dark together; keys 1–9 pick one. The picker and eyedropper edit the appearance on screen.",
            ),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(presets)
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_4()
                        .child(picker)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(meta)
                                .child(
                                    div()
                                        .max_w(px(260.))
                                        .text_xs()
                                        .text_color(theme.text_faint)
                                        .child(
                                            "Drag the square for saturation and brightness, or the strip for hue. The eyedropper samples anything on screen, background included.",
                                        ),
                                )
                                .child(actions)
                                .when(note.is_some(), |this| {
                                    this.child(
                                        div()
                                            .max_w(px(260.))
                                            .text_xs()
                                            .text_color(theme.danger)
                                            .child(note.unwrap_or_default()),
                                    )
                                }),
                        ),
                ),
        )
    }

    /// Open the system eyedropper. It resolves outside this process, so the
    /// modal polls a shared slot rather than blocking the frame. Cancelling
    /// the sampler leaves the accent untouched.
    fn pick_accent_from_screen(&mut self, cx: &mut Context<Self>) {
        let Some(Modal::Settings {
            appearance,
            accent_note,
            ..
        }) = &mut self.modal
        else {
            return;
        };
        let appearance = *appearance;
        *accent_note = None;
        let slot: screen_picker::PickSlot = Rc::new(RefCell::new(None));
        if !screen_picker::sample(slot.clone()) {
            if let Some(Modal::Settings { accent_note, .. }) = &mut self.modal {
                *accent_note = Some("Screen sampling is not available here.".to_string());
            }
            cx.notify();
            return;
        }
        cx.spawn(async move |this, cx| {
            // A cancelled sampler may never call back on older systems, so the
            // wait is bounded rather than open-ended.
            for _ in 0..300 {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                match slot.borrow_mut().take() {
                    None => continue,
                    Some(None) => return,
                    Some(Some(color)) => {
                        this.update(cx, |this, cx| {
                            this.apply_picked_accent(appearance, color, cx);
                        })
                        .ok();
                        return;
                    }
                }
            }
            this.update(cx, |this, cx| {
                if let Some(Modal::Settings { accent_note, .. }) = &mut this.modal {
                    *accent_note = Some("The screen picker timed out.".to_string());
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Store the sampled pixel in the tone that is on screen, clamped the same
    /// way the picker is so the stored hex is what the window actually paints.
    fn apply_picked_accent(&mut self, appearance: Appearance, color: u32, cx: &mut Context<Self>) {
        let resolved = theme::readable_plain_accent(color, appearance);
        self.set_plain_accent(appearance, Some(resolved), cx);
    }

    fn set_plain_accent(
        &mut self,
        appearance: Appearance,
        color: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let Some(Modal::Settings {
            theme,
            appearance: mode,
            accent_light,
            accent_dark,
            ..
        }) = &mut self.modal
        else {
            return;
        };
        match appearance {
            Appearance::Light => *accent_light = color,
            Appearance::Dark => *accent_dark = color,
        }
        let (kind, mode) = (*theme, *mode);
        self.theme = self.preview_theme(kind, mode);
        cx.notify();
    }

    pub(super) fn set_plain_accents(
        &mut self,
        light: Option<u32>,
        dark: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let Some(Modal::Settings {
            theme,
            appearance,
            accent_light,
            accent_dark,
            ..
        }) = &mut self.modal
        else {
            return;
        };
        *accent_light = light;
        *accent_dark = dark;
        let (kind, mode) = (*theme, *appearance);
        self.theme = self.preview_theme(kind, mode);
        cx.notify();
    }

    fn render_reminder_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let snooze_minutes = match &self.modal {
            Some(Modal::Settings {
                fullscreen_snooze_minutes,
                ..
            }) => *fullscreen_snooze_minutes,
            _ => 15,
        };
        // Reminders are a per-device setting, unlike the library defaults in
        // the next card. The toggle is a draft until Save, matching Vim mode.
        let reminders = matches!(
            &self.modal,
            Some(Modal::Settings {
                notifications: true,
                ..
            })
        );
        let fullscreen_reminders = matches!(
            &self.modal,
            Some(Modal::Settings {
                fullscreen_reminders: true,
                ..
            })
        );
        let pause_media_on_reminder = matches!(
            &self.modal,
            Some(Modal::Settings {
                pause_media_on_reminder: true,
                ..
            })
        );
        let (
            reminder_style,
            reminder_match,
            reminder_wallpaper_light,
            reminder_wallpaper_dark,
            reminder_wallpaper_separate,
            reminder_use_wallpaper,
            reminder_opacity,
            reminder_wallpaper_note,
        ) = match &self.modal {
            Some(Modal::Settings {
                reminder_background,
                reminder_match_theme,
                reminder_wallpaper_light,
                reminder_wallpaper_dark,
                reminder_wallpaper_separate,
                reminder_use_wallpaper,
                reminder_wallpaper_opacity,
                reminder_wallpaper_note,
                ..
            }) => (
                *reminder_background,
                *reminder_match_theme,
                reminder_wallpaper_light.clone(),
                reminder_wallpaper_dark.clone(),
                *reminder_wallpaper_separate,
                *reminder_use_wallpaper,
                *reminder_wallpaper_opacity,
                reminder_wallpaper_note.clone(),
            ),
            _ => (
                super::background::AbstractStyle::Aurora,
                true,
                None,
                None,
                false,
                false,
                0.35,
                None,
            ),
        };
        let reminder_wallpaper = if reminder_wallpaper_separate {
            match theme.appearance() {
                Appearance::Light => reminder_wallpaper_light.clone(),
                Appearance::Dark => reminder_wallpaper_dark.clone(),
            }
        } else {
            reminder_wallpaper_light.clone()
        };
        let has_reminder_wallpaper = reminder_wallpaper_light.is_some()
            || (reminder_wallpaper_separate && reminder_wallpaper_dark.is_some());
        let draft_backdrop = if reminder_use_wallpaper {
            reminder_wallpaper
                .clone()
                .map(|path| super::reminder::ReminderBackdrop::Wallpaper(path, reminder_opacity))
                .unwrap_or_else(|| {
                    super::reminder::ReminderBackdrop::Abstract(if reminder_match {
                        super::background::AbstractStyle::for_theme(theme.kind())
                    } else {
                        reminder_style
                    })
                })
        } else {
            super::reminder::ReminderBackdrop::Abstract(if reminder_match {
                super::background::AbstractStyle::for_theme(theme.kind())
            } else {
                reminder_style
            })
        };
        let permission = self.notification_permission;
        let supported = super::notifications::supported();
        let blocked = super::notifications::signature_blocked();
        // The status describes the toggle first. A permission is only relevant
        // while reminders are switched on; showing "Allowed" next to an off
        // toggle read as if reminders were active.
        let status = if !supported {
            "Notifications need the packaged Flowlist.app."
        } else if !reminders {
            "Off"
        } else if blocked {
            "Blocked for this ad-hoc build"
        } else if permission.allowed() {
            "On"
        } else if permission == super::notifications::Permission::Denied {
            "Denied in System Settings"
        } else {
            "Waiting for macOS permission…"
        };
        let mut actions = div().flex().flex_wrap().items_center().gap_2();
        if supported && reminders && permission.allowed() && !blocked {
            actions = actions.child(
                div()
                    .on_children_prepainted(self.theme_probe("notify-test".to_string()))
                    .id("notify-test")
                    .child(widgets::button(theme, "Send test", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if super::notifications::send_test() {
                            this.toast = Some("Test notification sent".to_string());
                        } else {
                            this.toast = Some("Allow notifications first".to_string());
                        }
                        cx.notify();
                    })),
            );
        }
        if fullscreen_reminders {
            actions = actions.child(
                div()
                    .on_children_prepainted(self.theme_probe("fullscreen-test".to_string()))
                    .id("fullscreen-test")
                    .child(widgets::button(theme, "Send full-screen test", false))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.send_fullscreen_test(
                            draft_backdrop.clone(),
                            snooze_minutes,
                            pause_media_on_reminder,
                            cx,
                        );
                    })),
            );
        }
        if supported
            && reminders
            && !blocked
            && permission == super::notifications::Permission::Denied
        {
            actions = actions.child(
                div()
                    .on_children_prepainted(self.theme_probe("notify-settings".to_string()))
                    .id("notify-settings")
                    .child(widgets::button(theme, "Open System Settings", false))
                    .on_click(cx.listener(|_, _, _, cx| {
                        super::notifications::open_settings();
                        cx.notify();
                    })),
            );
        }
        let reminders_card = self.settings_card(
            "Reminders",
            Some(
                "Due dates can send a macOS notification, even when Flowlist is in the background. Full-screen reminders are opt-in per task, project or folder.",
            ),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(div().text_sm().child("Due reminders"))
                                .child(
                                    div()
                                        .text_xs()
                                        .whitespace_normal()
                                        .text_color(theme.text_faint)
                                        .child(status),
                                ),
                        )
                        .child(
                            div()
                                .on_children_prepainted(self.theme_probe("notify-toggle".to_string()))
                                .id("notify-toggle")
                                .child(widgets::switch(theme, reminders))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings { notifications, .. }) =
                                        &mut this.modal
                                    {
                                        *notifications = !*notifications;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .whitespace_normal()
                        .text_color(theme.text_faint)
                        .child(
                            "Date-only tasks remind at 09:00; timestamped tasks at their time. The next 30 days are scheduled.",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(div().text_sm().child("Full-screen reminders"))
                                .child(
                                    div()
                                        .text_xs()
                                        .whitespace_normal()
                                        .text_color(theme.text_faint)
                                        .child(
                                            "Only tasks, projects and folders marked for full-screen reminders take over the screen.",
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe("notify-fullscreen-toggle".to_string()),
                                )
                                .id("notify-fullscreen-toggle")
                                .child(widgets::switch(theme, fullscreen_reminders))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        fullscreen_reminders,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *fullscreen_reminders = !*fullscreen_reminders;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(div().text_sm().child(
                                    "Pause media on full-screen reminders",
                                ))
                                .child(
                                    div()
                                        .text_xs()
                                        .whitespace_normal()
                                        .text_color(theme.text_faint)
                                        .child(
                                            "Pauses the current player when the reminder opens and resumes it on close.",
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe(
                                        "notify-fullscreen-media-toggle".to_string(),
                                    ),
                                )
                                .id("notify-fullscreen-media-toggle")
                                .child(widgets::switch(theme, pause_media_on_reminder))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        pause_media_on_reminder,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *pause_media_on_reminder =
                                            !*pause_media_on_reminder;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .when(pause_media_on_reminder, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .whitespace_normal()
                            .text_color(theme.warning)
                            .child(
                                "macOS requires Device control and data access for media control. Allow Flowlist in System Settings → Privacy & Security.",
                            ),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child("Snooze length"),
                        )
                        .child({
                            let mut row = div().flex().flex_wrap().items_center().gap_1();
                            for minutes in [15u32, 30, 60] {
                                let label = if minutes == 60 {
                                    "1 hour".to_string()
                                } else {
                                    format!("{minutes} min")
                                };
                                let active = snooze_minutes == minutes;
                                let key = format!("snooze-length:{minutes}");
                                row = row.child(
                                    div()
                                        .id(super::sid(key))
                                        .cursor_pointer()
                                        .child(widgets::toggle_chip(theme, label, active))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(Modal::Settings {
                                                fullscreen_snooze_minutes,
                                                ..
                                            }) = &mut this.modal
                                            {
                                                *fullscreen_snooze_minutes = minutes;
                                            }
                                            cx.notify();
                                        })),
                                );
                            }
                            row
                        }),
                )
                .child(actions)
                .when(blocked, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .whitespace_normal()
                            .text_color(theme.danger)
                            .child(
                                "macOS only grants notifications to a signed app. Build with an Apple Development or Developer ID certificate to enable reminders (see docs/packaging.md).",
                            ),
                    )
                }),
        );
        let mut background_body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(div().text_sm().child("Match theme"))
                            .child(
                                div()
                                    .text_xs()
                                    .whitespace_normal()
                                    .text_color(theme.text_faint)
                                    .child(
                                        "Use the abstract style that belongs to the current theme.",
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("reminder-background-match")
                            .child(widgets::switch(theme, reminder_match))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::Settings {
                                    reminder_match_theme,
                                    reminder_use_wallpaper,
                                    ..
                                }) = &mut this.modal
                                {
                                    *reminder_match_theme = !*reminder_match_theme;
                                    if *reminder_match_theme {
                                        *reminder_use_wallpaper = false;
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Abstract style"),
            );
        let mut styles = div().flex().flex_wrap().items_center().gap_1();
        for style in super::background::AbstractStyle::ALL {
            let active = !reminder_match && reminder_style == style;
            let follows = reminder_match
                && super::background::AbstractStyle::for_theme(theme.kind()) == style;
            let key = format!("reminder-background:{}", style.slug());
            styles = styles.child(
                div()
                    .on_children_prepainted(
                        self.theme_probe(format!("reminder-style-{}", style.slug())),
                    )
                    .id(super::sid(key))
                    .w(px(84.))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .cursor_pointer()
                    .child(
                        div()
                            .rounded_md()
                            .border_2()
                            .border_color(if active || follows {
                                theme.accent
                            } else {
                                theme.border_soft
                            })
                            .child(super::background::preview(style, theme)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if active || follows {
                                theme.accent
                            } else {
                                theme.text_dim
                            })
                            .child(style.label()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::Settings {
                            reminder_background,
                            reminder_match_theme,
                            reminder_use_wallpaper,
                            ..
                        }) = &mut this.modal
                        {
                            *reminder_background = style;
                            *reminder_match_theme = false;
                            *reminder_use_wallpaper = false;
                        }
                        cx.notify();
                    })),
            );
        }
        background_body = background_body.child(styles);
        background_body = background_body.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(div().text_sm().child(
                            "Use separate reminder backgrounds for light and dark modes",
                        ))
                        .child(
                            div()
                                .text_xs()
                                .whitespace_normal()
                                .text_color(theme.text_faint)
                                .child(
                                    "Off keeps one reminder background for both appearances. On shows a picker for each.",
                                ),
                        ),
                )
                .child(
                    div()
                        .on_children_prepainted(
                            self.theme_probe("reminder-wallpaper-separate-toggle".to_string()),
                        )
                        .id("reminder-wallpaper-separate-toggle")
                        .child(widgets::switch(theme, reminder_wallpaper_separate))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(Modal::Settings {
                                reminder_wallpaper_separate,
                                reminder_wallpaper_light,
                                reminder_wallpaper_dark,
                                ..
                            }) = &mut this.modal
                            {
                                *reminder_wallpaper_separate =
                                    !*reminder_wallpaper_separate;
                                if *reminder_wallpaper_separate
                                    && reminder_wallpaper_dark.is_none()
                                {
                                    *reminder_wallpaper_dark =
                                        reminder_wallpaper_light.clone();
                                }
                                if !*reminder_wallpaper_separate
                                    && reminder_wallpaper_light.is_none()
                                {
                                    *reminder_wallpaper_light =
                                        reminder_wallpaper_dark.clone();
                                }
                            }
                            cx.notify();
                        })),
                ),
        );
        if has_reminder_wallpaper {
            background_body = background_body
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(div().text_sm().child("Use custom background"))
                                .child(
                                    div()
                                        .text_xs()
                                        .whitespace_normal()
                                        .text_color(theme.text_faint)
                                        .child("Show the reminder background chosen below."),
                                ),
                        )
                        .child(
                            div()
                                .id("reminder-background-wallpaper")
                                .child(widgets::switch(theme, reminder_use_wallpaper))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        reminder_use_wallpaper,
                                        reminder_match_theme,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *reminder_use_wallpaper = !*reminder_use_wallpaper;
                                        if *reminder_use_wallpaper {
                                            *reminder_match_theme = false;
                                        }
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .when(reminder_use_wallpaper, |this| {
                    let mut row = div().flex().items_center().gap_1();
                    for percent in [20, 35, 50, 70, 90] {
                        let value = percent as f32 / 100.0;
                        let active = (reminder_opacity - value).abs() < 0.005;
                        let key = format!("reminder-opacity:{percent}");
                        row = row.child(
                            div()
                                .id(super::sid(key))
                                .cursor_pointer()
                                .child(widgets::toggle_chip(theme, format!("{percent}%"), active))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        reminder_wallpaper_opacity,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *reminder_wallpaper_opacity = value;
                                    }
                                    cx.notify();
                                })),
                        );
                    }
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child("Background opacity"),
                            )
                            .child(row),
                    )
                });
        }
        let reminder_modes: &[Appearance] = if reminder_wallpaper_separate {
            &[Appearance::Light, Appearance::Dark]
        } else {
            &[Appearance::Light]
        };
        for &mode in reminder_modes {
            let path = match mode {
                Appearance::Light => reminder_wallpaper_light.clone(),
                Appearance::Dark => reminder_wallpaper_dark.clone(),
            };
            let choose_id = match mode {
                Appearance::Light => "reminder-background-choose",
                Appearance::Dark => "reminder-background-choose-dark",
            };
            let choose_probe = match mode {
                Appearance::Light => "settings-choose-reminder-wallpaper".to_string(),
                Appearance::Dark => "settings-choose-reminder-wallpaper-dark".to_string(),
            };
            let remove_id = match mode {
                Appearance::Light => "reminder-background-remove",
                Appearance::Dark => "reminder-background-remove-dark",
            };
            let title = if reminder_wallpaper_separate {
                format!("{} reminder background", mode.label())
            } else {
                "Reminder background".to_string()
            };
            let hint = if reminder_wallpaper_separate {
                format!("Shown while the app is {}.", mode.label().to_lowercase())
            } else {
                "Shown in both light and dark modes.".to_string()
            };
            let mut section = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(div().text_sm().child(title))
                                .child(div().text_xs().text_color(theme.text_faint).child(hint)),
                        )
                        .when(
                            reminder_wallpaper_separate && theme.appearance() == mode,
                            |this| {
                                this.child(
                                    div().text_xs().text_color(theme.accent).child("On screen"),
                                )
                            },
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .on_children_prepainted(self.theme_probe(choose_probe))
                                .id(choose_id)
                                .child(widgets::button(
                                    theme,
                                    if path.is_some() {
                                        "Replace background"
                                    } else {
                                        "Choose background"
                                    },
                                    false,
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_reminder_wallpaper(mode, cx);
                                })),
                        )
                        .when(path.is_some(), |this| {
                            this.child(
                                div()
                                    .id(remove_id)
                                    .child(widgets::button(theme, "Remove", false))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(Modal::Settings {
                                            reminder_wallpaper_light,
                                            reminder_wallpaper_dark,
                                            reminder_wallpaper_separate,
                                            reminder_use_wallpaper,
                                            ..
                                        }) = &mut this.modal
                                        {
                                            match mode {
                                                Appearance::Light => {
                                                    *reminder_wallpaper_light = None
                                                }
                                                Appearance::Dark => *reminder_wallpaper_dark = None,
                                            }
                                            let left = reminder_wallpaper_light.is_some()
                                                || (*reminder_wallpaper_separate
                                                    && reminder_wallpaper_dark.is_some());
                                            if !left {
                                                *reminder_use_wallpaper = false;
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                        }),
                );
            if let Some(path) = &path {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Background".to_string());
                section = section.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            img(path.clone())
                                .object_fit(ObjectFit::Cover)
                                .w(px(72.))
                                .h(px(48.))
                                .rounded_md()
                                .opacity(reminder_opacity),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(name)),
                );
            }
            background_body = background_body.child(section);
        }
        if let Some(note) = &reminder_wallpaper_note {
            let success = {
                let lower = note.to_lowercase();
                lower.contains("saved") || lower.contains("selected")
            };
            background_body = background_body.child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(if success { theme.success } else { theme.danger })
                    .child(note.clone()),
            );
        }
        let default_defer = matches!(
            &self.modal,
            Some(Modal::Settings {
                default_defer_notification: true,
                ..
            })
        );
        let default_planned = matches!(
            &self.modal,
            Some(Modal::Settings {
                default_planned_notification: true,
                ..
            })
        );
        let default_latest = matches!(
            &self.modal,
            Some(Modal::Settings {
                default_latest_start_notification: true,
                ..
            })
        );
        let default_due = matches!(
            &self.modal,
            Some(Modal::Settings {
                default_due_notification: true,
                ..
            })
        );
        let mut defaults_body = div().flex().flex_col().gap_2();
        for (id, label, enabled) in [
            ("default-defer-notification", "Defer", default_defer),
            ("default-planned-notification", "Planned", default_planned),
            (
                "default-latest-start-notification",
                "Latest start",
                default_latest,
            ),
            ("default-due-notification", "Due", default_due),
        ] {
            defaults_body = defaults_body.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().flex_1().text_sm().child(label))
                    .child(
                        div()
                            .on_children_prepainted(self.theme_probe(id.to_string()))
                            .id(id)
                            .child(widgets::switch(theme, enabled))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(Modal::Settings {
                                    default_defer_notification,
                                    default_planned_notification,
                                    default_latest_start_notification,
                                    default_due_notification,
                                    ..
                                }) = &mut this.modal
                                {
                                    match id {
                                        "default-defer-notification" => {
                                            *default_defer_notification =
                                                !*default_defer_notification
                                        }
                                        "default-planned-notification" => {
                                            *default_planned_notification =
                                                !*default_planned_notification
                                        }
                                        "default-latest-start-notification" => {
                                            *default_latest_start_notification =
                                                !*default_latest_start_notification
                                        }
                                        _ => *default_due_notification = !*default_due_notification,
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            );
        }
        defaults_body = defaults_body.child(self.render_modal_field(6, cx));
        let default_notifications_card = self.settings_card(
            "Default notifications",
            Some("Automatic alerts for every item, unless removed from that item. Latest start is due minus estimate. The time applies to date-only custom notifications."),
            defaults_body,
        );
        let reminder_background_card = self.settings_card(
            "Reminder background",
            Some(
                "Full-screen reminders can sit on a theme-synced abstract backdrop or your own background. The main window keeps its normal surfaces.",
            ),
            background_body,
        );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(reminders_card)
            .child(default_notifications_card)
            .child(reminder_background_card)
            .into_any_element()
    }

    fn render_calendar_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let calendar_events = matches!(
            &self.modal,
            Some(Modal::Settings {
                calendar_events: true,
                ..
            })
        );
        let calendar_hidden = match &self.modal {
            Some(Modal::Settings {
                calendar_hidden, ..
            }) => calendar_hidden.clone(),
            _ => BTreeSet::new(),
        };
        let calendar_permission = super::calendar::permission();
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().flex_1().text_sm().child("Show Calendar events in Forecast"))
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("settings-calendar-events".to_string()),
                            )
                            .id("settings-calendar-events")
                            .child(widgets::switch(theme, calendar_events))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::Settings { calendar_events, .. }) = &mut this.modal
                                {
                                    *calendar_events = !*calendar_events;
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(if calendar_permission.allowed() {
                        theme.success
                    } else {
                        theme.text_faint
                    })
                    .child(format!("Calendar access: {}", calendar_permission.label())),
            );
        if !calendar_permission.allowed() {
            let can_ask = matches!(
                calendar_permission,
                super::calendar::Permission::Unknown
                    | super::calendar::Permission::NotDetermined
            );
            let label = if can_ask {
                "Allow Calendar access"
            } else {
                "Open Calendar privacy settings"
            };
            body = body.child(
                div()
                    .id("calendar-permission-button")
                    .w(px(220.))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.field)
                    .text_xs()
                    .text_color(theme.accent)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(label)
                    .on_click(cx.listener(move |_this, _, _, cx| {
                        if can_ask {
                            super::calendar::request_permission();
                        } else {
                            super::calendar::open_settings();
                        }
                        cx.notify();
                    })),
            );
        }
        let main_card = self.settings_card(
            "Calendar",
            Some(
                "Read-only macOS Calendar events appear beside tasks in Forecast. Access is per device and Flowlist never changes your calendars.",
            ),
            body,
        );
        let mut calendars = div().flex().flex_col().gap_2();
        if calendar_permission.allowed() && !self.calendar_calendars.is_empty() {
            for calendar in &self.calendar_calendars {
                let id = calendar.id.clone();
                let hidden = calendar_hidden.contains(&id);
                let color: gpui::Hsla = calendar
                    .color
                    .map(|color| rgb(color).into())
                    .unwrap_or(theme.accent);
                calendars = calendars.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .flex_1()
                                .min_w_0()
                                .child(div().size(px(7.)).rounded_full().bg(color))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_sm()
                                        .child(calendar.title.clone()),
                                )
                                .when_some(calendar.source.clone(), |this, source| {
                                    this.child(
                                        div()
                                            .flex_shrink_0()
                                            .text_xs()
                                            .text_color(theme.text_faint)
                                            .child(source),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe(format!("calendar-keep:{id}")),
                                )
                                .id(super::sid(format!("calendar-keep:{id}")))
                                .child(widgets::switch(theme, !hidden))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(Modal::Settings { calendar_hidden, .. }) =
                                        &mut this.modal
                                    {
                                        if !calendar_hidden.remove(&id) {
                                            calendar_hidden.insert(id.clone());
                                        }
                                    }
                                    cx.notify();
                                })),
                        ),
                );
            }
        } else if calendar_permission.allowed() {
            calendars = calendars.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No calendars are available."),
            );
        }
        let calendars_card = self.settings_card(
            "Calendars",
            Some("Turn a calendar off to keep its events out of Forecast. New calendars appear on by default."),
            calendars,
        );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(main_card)
            .child(calendars_card)
            .into_any_element()
    }

    fn render_workflow_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let vim = matches!(&self.modal, Some(Modal::Settings { vim_mode: true, .. }));
        let keyboard = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .id("modal-vim-mode")
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .cursor_pointer()
                    .child(div().flex_1().text_sm().child("Vim mode"))
                    .child(widgets::switch(theme, vim))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Settings { vim_mode, .. }) = &mut this.modal {
                            *vim_mode = !*vim_mode;
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(theme.text_faint)
                    .child(
                        "hjkl motions, counts, operators and pane focus. ⌘ shortcuts stay available.",
                    ),
            );
        let keep_subtasks = matches!(
            &self.modal,
            Some(Modal::Settings {
                keep_completed_subtasks: true,
                ..
            })
        );
        let subtasks_card = self.settings_card(
            "Subtasks",
            Some(
                "Finished subtasks can stay under their parent instead of disappearing when the list hides completed work.",
            ),
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().flex_1().text_sm().child("Keep completed subtasks"))
                .child(
                    div()
                        .on_children_prepainted(self.theme_probe("keep-subtasks-toggle".to_string()))
                        .id("keep-subtasks-toggle")
                        .child(widgets::switch(theme, keep_subtasks))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(Modal::Settings {
                                keep_completed_subtasks,
                                ..
                            }) = &mut this.modal
                            {
                                *keep_completed_subtasks = !*keep_completed_subtasks;
                            }
                            cx.notify();
                        })),
                ),
        );
        let toolbar_labels = matches!(
            &self.modal,
            Some(Modal::Settings {
                toolbar_labels: true,
                ..
            })
        );
        let toolbar_card = self.settings_card(
            "Toolbar",
            Some("Show a small title under the header icons. Search keeps its placeholder and no title."),
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().flex_1().text_sm().child("Show button titles"))
                .child(
                    div()
                        .on_children_prepainted(
                            self.theme_probe("toolbar-labels-toggle".to_string()),
                        )
                        .id("toolbar-labels-toggle")
                        .child(widgets::switch(theme, toolbar_labels))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(Modal::Settings {
                                toolbar_labels, ..
                            }) = &mut this.modal
                            {
                                *toolbar_labels = !*toolbar_labels;
                            }
                            cx.notify();
                        })),
                ),
        );
        let due_counts = matches!(
            &self.modal,
            Some(Modal::Settings {
                forecast_due_counts: true,
                ..
            })
        );
        let forecast_flagged = matches!(
            &self.modal,
            Some(Modal::Settings {
                forecast_flagged: true,
                ..
            })
        );
        let forecast_due_only = matches!(
            &self.modal,
            Some(Modal::Settings {
                forecast_due_only: true,
                ..
            })
        );
        let forecast_card = self.settings_card(
            "Forecast",
            Some("The Forecast tag pulls undated actions into Forecast. Flagged undated actions can join them. Badges can count only due work."),
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(self.render_modal_field(5, cx))
                .child(self.render_modal_field(7, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().flex_1().text_sm().child("Show flagged undated tasks"))
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe("forecast-flagged-toggle".to_string()),
                                )
                                .id("forecast-flagged-toggle")
                                .child(widgets::switch(theme, forecast_flagged))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        forecast_flagged,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *forecast_flagged = !*forecast_flagged;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().flex_1().text_sm().child("Count only due items"))
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe("forecast-due-counts-toggle".to_string()),
                                )
                                .id("forecast-due-counts-toggle")
                                .child(widgets::switch(theme, due_counts))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        forecast_due_counts,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *forecast_due_counts = !*forecast_due_counts;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().flex_1().text_sm().child("Show only due dates"))
                        .child(
                            div()
                                .on_children_prepainted(
                                    self.theme_probe("forecast-due-only-toggle".to_string()),
                                )
                                .id("forecast-due-only-toggle")
                                .child(widgets::switch(theme, forecast_due_only))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Settings {
                                        forecast_due_only,
                                        ..
                                    }) = &mut this.modal
                                    {
                                        *forecast_due_only = !*forecast_due_only;
                                    }
                                    cx.notify();
                                })),
                        ),
                ),
        );
        let focus_banner = matches!(
            &self.modal,
            Some(Modal::Settings {
                focus_banner: true,
                ..
            })
        );
        let focus_card = self.settings_card(
            "Focus",
            Some("The eye beside the back and forward arrows always stays available. When the banner is off, a newly enabled focus still shows it for a few seconds."),
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().flex_1().text_sm().child("Keep the Focus banner visible"))
                .child(
                    div()
                        .on_children_prepainted(
                            self.theme_probe("focus-banner-toggle".to_string()),
                        )
                        .id("focus-banner-toggle")
                        .child(widgets::switch(theme, focus_banner))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(Modal::Settings {
                                focus_banner, ..
                            }) = &mut this.modal
                            {
                                *focus_banner = !*focus_banner;
                            }
                            cx.notify();
                        })),
                ),
        );
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.settings_card("Keyboard", None, keyboard))
            .child(toolbar_card)
            .child(subtasks_card)
            .child(forecast_card)
            .child(focus_card)
            .child(
                self.settings_card(
                    "Review and history",
                    Some(
                        "Defaults for the library. Projects can override their own review cadence.",
                    ),
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(self.render_modal_field(0, cx))
                        .child(self.render_modal_field(1, cx))
                        .child(self.render_modal_field(2, cx)),
                ),
            )
            .into_any_element()
    }

    fn render_nearby_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.settings_card(
                "Distance",
                Some(
                    "The Nearby perspective keeps tags within this radius of the library location.",
                ),
                self.render_modal_field(3, cx),
            ))
            .into_any_element()
    }

    fn render_trash_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.settings_card(
                "Retention",
                Some("Zero keeps deleted tasks forever. Expired items are swept when Settings is saved."),
                self.render_modal_field(4, cx),
            ))
            .into_any_element()
    }

    fn render_mobile_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let root = self.store.paths().root.clone();
        let mobile = crate::serve::load_mobile(&root);
        let running = crate::serve::running(&root);
        let installed = crate::serve::agent::installed();
        let addresses = crate::serve::addresses();
        let port = mobile.port;
        let status = if running.is_some() {
            "Running"
        } else if installed {
            "Starting"
        } else {
            "Off"
        };
        let status_color = if running.is_some() {
            theme.success
        } else if installed {
            theme.warning
        } else {
            theme.text_faint
        };

        let toggle = {
            let paths = self.store.paths().clone();
            div()
                .id("mobile-server-toggle")
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    let installed = crate::serve::agent::installed();
                    if installed {
                        match crate::serve::agent::remove() {
                            Ok(_) => {
                                this.toast =
                                    Some("Mobile access is off; the login item was removed".into())
                            }
                            Err(error) => this.toast = Some(error.to_string()),
                        }
                    } else {
                        let mobile = crate::serve::load_mobile(&paths.root);
                        match crate::serve::agent::install(&paths, mobile.port) {
                            Ok(_) => {
                                this.toast =
                                    Some("Mobile access is on; the server starts at login".into())
                            }
                            Err(error) => this.toast = Some(error.to_string()),
                        }
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text)
                                .child("Serve to my phone"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(format!("{status} · port {port} · starts when you log in")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().size(px(7.)).rounded_full().bg(status_color))
                        .child(widgets::switch(theme, installed)),
                )
        };

        let server = self.settings_card(
            "Mobile access",
            Some(
                "Your phone connects straight to this Mac. No account, no cloud and no telemetry.",
            ),
            div().flex().flex_col().gap_3().child(toggle),
        );

        let pairing = if let Some(info) = &running {
            let remaining = (info.pairing_expires_at - chrono::Utc::now().timestamp()).max(0);
            let mut addresses_column = div().flex().flex_col().gap_1();
            if addresses.is_empty() {
                addresses_column = addresses_column.child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("No network address found. Connect to Wi-Fi or Tailscale."),
                );
            }
            for address in &addresses {
                addresses_column = addresses_column.child(
                    div()
                        .text_xs()
                        .text_color(theme.text)
                        .child(format!("http://{address}:{}", info.port)),
                );
            }
            let folders = (if remaining >= 60 {
                format!("{} minutes", remaining / 60)
            } else {
                format!("{} seconds", remaining)
            });
            let code_row = div()
                .flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(info.pairing_code.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child(format!("expires in {folders}")),
                );
            // The QR carries every address this Mac answers on, so the phone
            // can try the LAN first and Tailscale second without retyping.
            let payload = if addresses.is_empty() {
                None
            } else {
                let mut payload = format!("flowlist://pair?code={}", info.pairing_code);
                for address in &addresses {
                    payload.push_str(&format!("&address={address}:{}", info.port));
                }
                Some(payload)
            };
            let mut body = div().flex().flex_col().gap_3();
            if let Some(qr) = payload.as_deref().and_then(qr_code_block) {
                body = body.child(
                    div().flex().items_start().gap_4().child(qr).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(code_row)
                            .child(addresses_column),
                    ),
                );
            } else {
                body = body.child(code_row).child(addresses_column);
            }
            self.settings_card(
                "Pair a phone",
                Some(
                    "Scan this code with Flowlist on Android, or enter one of these addresses and the code.",
                ),
                body,
            )
        } else {
            self.settings_card(
                "Pair a phone",
                Some(
                    "Turn on Mobile access above, then the pairing code and this Mac's addresses appear here.",
                ),
                div().into_any_element(),
            )
        };

        let mut devices = div().flex().flex_col().gap_1();
        if mobile.devices.is_empty() {
            devices = devices.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No phones paired yet."),
            );
        }
        for device in &mobile.devices {
            let id = device.id.clone();
            let paths = self.store.paths().clone();
            devices = devices.child(
                div()
                    .id(gpui::ElementId::Name(
                        format!("mobile-device-{}", device.id).into(),
                    ))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .py_1()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text)
                                    .child(device.name.clone()),
                            )
                            .child(div().text_xs().text_color(theme.text_faint).child(format!(
                                "Paired {}",
                                device.created_at.get(..10).unwrap_or("")
                            ))),
                    )
                    .child(
                        div()
                            .id(gpui::ElementId::Name(
                                format!("mobile-revoke-{}", device.id).into(),
                            ))
                            .child(widgets::button(theme, "Revoke", false))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let mut file = crate::serve::load_mobile(&paths.root);
                                file.devices.retain(|device| device.id != id);
                                match crate::serve::save_mobile(&paths.root, &file) {
                                    Ok(()) => this.toast = Some("Phone removed".into()),
                                    Err(error) => this.toast = Some(error.to_string()),
                                }
                                cx.notify();
                            })),
                    ),
            );
        }
        let devices = self.settings_card(
            "Paired phones",
            Some("Revoking a phone makes it pair again. Nothing else changes."),
            devices,
        );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(server)
            .child(pairing)
            .child(devices)
            .into_any_element()
    }

    fn render_sidebar_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if !matches!(self.modal, Some(Modal::Settings { .. })) {
            return div().into_any_element();
        }
        let db = self.store.db();
        let builtin_items: Vec<(String, String, Vec<String>, Vec<String>)> =
            BuiltinPerspective::ALL
                .iter()
                .map(|perspective| {
                    (
                        format!("builtin:{}", perspective.slug()),
                        perspective.label().to_string(),
                        Vec::new(),
                        Vec::new(),
                    )
                })
                .collect();
        let priority_items: Vec<(String, String, Vec<String>, Vec<String>)> = Priority::ALL
            .iter()
            .map(|priority| {
                (
                    format!("priority:{}", priority.slug()),
                    priority.label().to_string(),
                    Vec::new(),
                    Vec::new(),
                )
            })
            .collect();
        let library_items: Vec<(String, String, Vec<String>, Vec<String>)> = db
            .flatten_containers()
            .into_iter()
            .filter_map(|row| match row.container {
                ContainerId::Folder(id) => {
                    let related = db
                        .folder_containers(&id)
                        .into_iter()
                        .map(|child| match child {
                            ContainerId::Folder(id) => format!("folder:{id}"),
                            ContainerId::Project(id) => format!("project:{id}"),
                        })
                        .collect();
                    let ancestors = db
                        .folder_ancestors(&id)
                        .into_iter()
                        .map(|folder| format!("folder:{folder}"))
                        .collect();
                    Some((
                        format!("folder:{id}"),
                        db.folder_path(&id),
                        related,
                        ancestors,
                    ))
                }
                ContainerId::Project(id) => {
                    let project = db.projects.get(&id)?;
                    if !project.status.is_open() {
                        return None;
                    }
                    let mut ancestors = Vec::new();
                    if let Some(folder) = &project.folder {
                        ancestors.push(format!("folder:{folder}"));
                        ancestors.extend(
                            db.folder_ancestors(folder)
                                .into_iter()
                                .map(|folder| format!("folder:{folder}")),
                        );
                    }
                    Some((
                        format!("project:{id}"),
                        db.project_path(&id),
                        Vec::new(),
                        ancestors,
                    ))
                }
            })
            .collect();
        let mut saved_items: Vec<(String, String, Vec<String>, Vec<String>)> = db
            .perspectives
            .values()
            .map(|perspective| {
                (
                    format!("custom:{}", perspective.id),
                    perspective.name.clone(),
                    Vec::new(),
                    Vec::new(),
                )
            })
            .collect();
        saved_items.sort_by_cached_key(|item| item.1.to_lowercase());
        let mut tag_items: Vec<(String, String, Vec<String>, Vec<String>)> = db
            .tags
            .keys()
            .map(|id| {
                let related = db
                    .tag_descendants(id)
                    .into_iter()
                    .map(|tag| format!("tag:{tag}"))
                    .collect();
                let ancestors = db
                    .tag_ancestors(id)
                    .into_iter()
                    .map(|tag| format!("tag:{tag}"))
                    .collect();
                (format!("tag:{id}"), db.tag_path(id), related, ancestors)
            })
            .collect();
        tag_items.sort_by_cached_key(|item| item.1.to_lowercase());
        let theme = self.theme;
        let hide_empty_views = matches!(
            &self.modal,
            Some(Modal::Settings {
                hide_empty_views: true,
                ..
            })
        );
        let hide_empty_row = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().text_sm().child("Hide empty custom views"))
                    .child(
                        div()
                            .text_xs()
                            .whitespace_normal()
                            .text_color(theme.text_faint)
                            .child(
                                "A saved view leaves the rail while its filter matches nothing, and returns with the first matching task.",
                            ),
                    ),
            )
            .child(
                div()
                    .on_children_prepainted(
                        self.theme_probe("hide-empty-views-toggle".to_string()),
                    )
                    .id("hide-empty-views-toggle")
                    .child(widgets::switch(theme, hide_empty_views))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Settings {
                            hide_empty_views, ..
                        }) = &mut this.modal
                        {
                            *hide_empty_views = !*hide_empty_views;
                        }
                        cx.notify();
                    })),
            );
        self.settings_card(
            "Sidebar",
            Some(
                "Open one heading at a time. Tick Hide to take a row out of the rail, and \
                 Hide count to quiet its badge. Inbox, Today and Trash always stay.",
            ),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(hide_empty_row)
                .child(widgets::divider(theme))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_end()
                        .gap_2()
                        .pb_1()
                        .child(
                            div()
                                .flex_1()
                                .pl(px(20.))
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text_faint)
                                .child("ITEM"),
                        )
                        .child(
                            div()
                                .w(px(54.))
                                .text_center()
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text_faint)
                                .child("HIDE"),
                        )
                        .child(
                            div()
                                .w(px(78.))
                                .text_center()
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text_faint)
                                .child("HIDE COUNT"),
                        ),
                )
                .child(self.sidebar_matrix_group(
                    SidebarGroup::Perspectives,
                    "PERSPECTIVES",
                    builtin_items,
                    cx,
                ))
                .child(self.sidebar_matrix_group(
                    SidebarGroup::Priorities,
                    "PRIORITIES",
                    priority_items,
                    cx,
                ))
                .when(!library_items.is_empty(), |this| {
                    this.child(self.sidebar_matrix_group(
                        SidebarGroup::Library,
                        "LIBRARY",
                        library_items,
                        cx,
                    ))
                })
                .when(!saved_items.is_empty(), |this| {
                    this.child(self.sidebar_matrix_group(
                        SidebarGroup::Saved,
                        "SAVED",
                        saved_items,
                        cx,
                    ))
                })
                .when(!tag_items.is_empty(), |this| {
                    this.child(self.sidebar_matrix_group(SidebarGroup::Tags, "TAGS", tag_items, cx))
                }),
        )
        .into_any_element()
    }

    fn sidebar_matrix_group(
        &self,
        group: SidebarGroup,
        title: &'static str,
        items: Vec<(String, String, Vec<String>, Vec<String>)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Settings {
            sidebar,
            sidebar_group,
            ..
        }) = &self.modal
        else {
            return div().into_any_element();
        };
        let open = *sidebar_group == Some(group);
        let boxes = self.settings_toggle_boxes.clone();
        let probe_key = format!("sidebar-group-{}", group.slug());
        let row = div()
            .id(gpui::ElementId::Name(
                format!("sidebar-heading-{}", group.slug()).into(),
            ))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .pt_2()
            .pb_1()
            .cursor_pointer()
            .hover(|style| style.opacity(0.85))
            .child(icons::icon(
                if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                12.0,
                theme.text_faint,
            ))
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_faint)
                    .child(title),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(Modal::Settings { sidebar_group, .. }) = &mut this.modal {
                    *sidebar_group = if *sidebar_group == Some(group) {
                        None
                    } else {
                        Some(group)
                    };
                }
                cx.notify();
            }));
        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        probe_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(row);
        if open {
            for (key, label, related, ancestors) in items {
                let parent_hidden = ancestors.iter().any(|ancestor| sidebar.is_hidden(ancestor));
                let visible = !sidebar.is_hidden(&key) && !parent_hidden;
                let enabled = SidebarOptions::can_hide(&key) && !parent_hidden;
                container = container.child(self.sidebar_matrix_row(
                    key,
                    label,
                    enabled,
                    visible,
                    parent_hidden,
                    related,
                    cx,
                ));
            }
        }
        container.into_any_element()
    }

    fn sidebar_matrix_row(
        &self,
        key: String,
        label: String,
        enabled: bool,
        visible: bool,
        dimmed: bool,
        related: Vec<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Settings { sidebar, .. }) = &self.modal else {
            return div().into_any_element();
        };
        let count_hidden = sidebar.is_count_hidden(&key);
        let hide_key = key.clone();
        let count_key = key.clone();
        let hide_box = format!("sidebar-item-{key}");
        let count_box = format!("sidebar-count-{key}");
        let hide_related = related;
        // Protected rows have no Hide box at all. A row dimmed by a hidden
        // parent keeps a disabled box, because it becomes usable as soon as
        // the parent comes back.
        let hide = if enabled {
            self.sidebar_matrix_box(hide_box, !visible, true)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(Modal::Settings { sidebar, .. }) = &mut this.modal {
                        if !sidebar.is_hidden(&hide_key) {
                            sidebar.hidden.insert(hide_key.clone());
                            for child in &hide_related {
                                sidebar.hidden.insert(child.clone());
                            }
                        } else {
                            sidebar.hidden.remove(&hide_key);
                            for child in &hide_related {
                                sidebar.hidden.remove(child);
                            }
                        }
                    }
                    cx.notify();
                }))
                .into_any_element()
        } else if !SidebarOptions::can_hide(&key) {
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child("—")
                .into_any_element()
        } else {
            self.sidebar_matrix_box(hide_box, !visible, false)
                .into_any_element()
        };
        let count_enabled = visible;
        let count = self
            .sidebar_matrix_box(count_box, count_hidden, count_enabled)
            .when(count_enabled, |this| {
                this.on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(Modal::Settings { sidebar, .. }) = &mut this.modal {
                        sidebar.counts_configured = true;
                        if !sidebar.counts_hidden.remove(&count_key) {
                            sidebar.counts_hidden.insert(count_key.clone());
                        }
                    }
                    cx.notify();
                }))
            })
            .into_any_element();
        let label_color = if dimmed || !visible {
            theme.text_faint
        } else {
            theme.text
        };
        div()
            .id(gpui::ElementId::Name(format!("sidebar-row-{key}").into()))
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .py_0p5()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pl(px(20.))
                    .line_clamp(1)
                    .text_ellipsis()
                    .text_sm()
                    .text_color(label_color)
                    .when(!visible, |this| this.line_through())
                    .child(label),
            )
            .child(div().w(px(54.)).flex().justify_center().child(hide))
            .child(div().w(px(78.)).flex().justify_center().child(count))
            .into_any_element()
    }

    fn sidebar_matrix_box(
        &self,
        key: String,
        checked: bool,
        enabled: bool,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = self.theme;
        let boxes = self.settings_toggle_boxes.clone();
        let key_for_id = key.clone();
        let color = if enabled {
            theme.accent
        } else {
            theme.text_faint
        };
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .p_0p5()
            .child(
                div()
                    .size(px(15.))
                    .rounded_sm()
                    .border_1()
                    .border_color(if checked {
                        color
                    } else if enabled {
                        theme.border
                    } else {
                        theme.border_soft
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |this| {
                        this.child(icons::icon(Icon::Check, 10.0, color))
                    }),
            )
            .id(gpui::ElementId::Name(key_for_id.into()))
    }

    /// Record where a theme control painted so a GUI probe can click it.
    pub(super) fn theme_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.theme_boxes.clone();
        move |bounds, _, _| {
            let Some(bounds) = bounds.first() else {
                return;
            };
            boxes.borrow_mut().insert(
                key.clone(),
                [
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ],
            );
        }
    }

    fn theme_swatch(
        &self,
        kind: ThemeKind,
        selected: bool,
        appearance: Appearance,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let light = Theme::new(kind, Appearance::Light);
        let dark = Theme::new(kind, Appearance::Dark);
        div()
            .id(kind.slug())
            .flex_1()
            .min_w_0()
            .p_0p5()
            .rounded_lg()
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border })
            .bg(if selected {
                theme.selection
            } else {
                theme.field
            })
            .cursor_pointer()
            .hover(|style| {
                style.bg(if selected {
                    theme.selection
                } else {
                    theme.hover
                })
            })
            .child(
                div()
                    .flex()
                    .h(px(28.))
                    .w_full()
                    .rounded_md()
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.border_soft)
                    .child(self.half_preview(light, appearance == Appearance::Light))
                    .child(self.half_preview(dark, appearance == Appearance::Dark)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .pt_0p5()
                    .child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .font_weight(if selected {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if selected { theme.text } else { theme.text_dim })
                            .child(kind.label()),
                    )
                    .when(selected, |this| {
                        this.child(icons::icon(icons::Icon::Check, 11.0, theme.accent))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(Modal::Settings {
                    theme, appearance, ..
                }) = &mut this.modal
                {
                    *theme = kind;
                    // Show the family on the whole window before Save.
                    this.theme = Theme::new(kind, *appearance);
                }
                cx.notify();
            }))
    }

    /// One half of a theme swatch: the selected appearance is drawn at full
    /// strength, the other side recedes so the pair reads as one family. Each
    /// half is a tiny window — rail, chrome and two lines — so the palette
    /// reads as a place rather than as two rectangles.
    fn half_preview(&self, preview: Theme, active: bool) -> impl IntoElement {
        div()
            .flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(preview.bg)
            .opacity(if active { 1.0 } else { 0.4 })
            .child(
                div()
                    .w(px(10.))
                    .h_full()
                    .bg(preview.glass_fill(90.))
                    .border_r_1()
                    .border_color(preview.border_soft),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(div().h(px(6.)).w_full().bg(preview.glass_fill(90.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .justify_center()
                            .gap(px(3.))
                            .px(px(4.))
                            .child(
                                div()
                                    .h(px(2.5))
                                    .w(px(20.))
                                    .rounded_full()
                                    .bg(preview.text.opacity(0.55)),
                            )
                            .child(
                                div()
                                    .h(px(2.5))
                                    .w(px(13.))
                                    .rounded_full()
                                    .bg(preview.text_faint.opacity(0.6)),
                            )
                            .child(div().h(px(3.)).w(px(7.)).rounded_full().bg(preview.accent)),
                    ),
            )
    }

    fn render_import_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let merge = matches!(&self.modal, Some(Modal::Import { merge: true, .. }));
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("modal-merge")
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .child(div().text_sm().child("Merge into the current library"))
                    .child(widgets::switch(theme, merge))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Import { merge, .. }) = &mut this.modal {
                            *merge = !*merge;
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(if merge {
                        "Merge adds new items and keeps existing ids."
                    } else {
                        "Replace swaps the whole library for the pasted one."
                    }),
            )
    }
}

fn help_body(theme: super::theme::Theme) -> impl IntoElement {
    let sections: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "Work with tasks",
            vec![
                ("Space", "complete or reopen"),
                ("!", "flag or unflag"),
                ("⏎", "edit the selected title"),
                ("⌘⌫", "delete selection or the sidebar entity"),
                ("Tab / ⇧Tab", "nest / unnest"),
                ("za / zo / zc", "toggle / open / close subtasks"),
                ("⌥↑ / ⌥↓", "move within siblings"),
                ("⌘D", "toggle flag on selection"),
            ],
        ),
        (
            "Subtasks",
            vec![
                ("⌘L", "add a subtask"),
                ("Click ○", "complete or reopen a subtask"),
                ("Click title", "open the subtask"),
                ("Drag ⠿", "reorder within the parent"),
                ("Separate", "move a subtask out from under its parent"),
            ],
        ),
        (
            "Navigate",
            vec![
                ("↑ / ↓", "move selection"),
                ("⇧↑ / ⇧↓", "extend selection"),
                ("⌘1…⌘8", "jump to perspective"),
                ("n", "quick capture"),
                ("/", "search"),
                ("⌘F", "search"),
                ("i", "toggle inspector"),
            ],
        ),
        (
            "Vim mode",
            vec![
                ("j / k", "move down / up (counts: 3j)"),
                ("h / l", "outdent / indent, or collapse / expand"),
                ("gg / G", "top / bottom (5G jumps to line 5)"),
                ("Ctrl-d / Ctrl-u", "half page down / up"),
                ("o / O", "new task below / above"),
                ("x", "complete or reopen"),
                ("!", "flag or unflag"),
                ("dd / yy / p", "delete / yank / paste"),
                ("u / Ctrl-r", "undo / redo"),
                ("r / e", "rename / edit title"),
                ("dd", "delete the sidebar entity"),
                ("⇧L", "set a tag or library location"),
                ("⇧J / ⇧K", "move task down / up among siblings"),
                ("Ctrl-w h l j", "focus sidebar / content / checklist"),
                ("?", "this help"),
                ("Esc", "clear counts, selection and search"),
            ],
        ),
        (
            "Library",
            vec![
                ("⌘Z / ⇧⌘Z", "undo / redo"),
                ("⌘N", "quick capture"),
                ("⇧⌘N", "new project"),
                ("⌘T", "toggle light / dark"),
                ("⇧⌘T / ⌥⌘T", "next / previous theme family"),
                ("⇧T", "toggle light / dark in vim mode"),
                ("⌘,", "settings"),
                ("⌘C", "copy selected tasks"),
                ("⌘R", "rename current project/folder/tag/perspective"),
                ("⌥⌘N", "new folder"),
                ("⌃⌘N", "new tag"),
                ("⇧⌘P", "new saved perspective"),
                ("⌘O", "quick open"),
                ("⇧⌘O", "import"),
                ("⇧⌘E", "copy TaskPaper export"),
                ("? / ⌘?", "this help"),
            ],
        ),
        (
            "Settings",
            vec![
                (
                    "⌘1…⌘7",
                    "Appearance, Sidebar, Workflow, Nearby, Trash, Mobile, About",
                ),
                ("⇥", "next field in the open section"),
                ("1…9", "Plain accent preset while Appearance is open"),
            ],
        ),
        (
            "Mouse",
            vec![
                ("Click", "select a task"),
                ("⌘Click", "add or remove from selection"),
                ("⇧Click", "select a range"),
                ("Double-click", "edit the title"),
                ("Drag", "nest under a task, or drop on a project/tag"),
                ("Click the flag", "toggle flag"),
                ("Click ○", "toggle complete"),
            ],
        ),
    ];
    let mut body = div()
        .id("help-scroll")
        .flex()
        .flex_col()
        .gap_3()
        .max_h(px(480.))
        .overflow_y_scroll();
    for (title, rows) in sections {
        let mut block = div().flex().flex_col().gap_1().child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(title),
        );
        for (key, label) in rows {
            block = block.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(92.))
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(theme.text_dim)
                            .child(key),
                    )
                    .child(div().text_sm().text_color(theme.text_dim).child(label)),
            );
        }
        body = body.child(block);
    }
    body.child(
        div()
            .text_xs()
            .text_color(theme.text_faint)
            .child("The GUI, CLI and MCP server all write to the same library."),
    )
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Any => "Any",
        TaskStatus::Available => "Available",
        TaskStatus::Blocked => "Blocked",
        TaskStatus::Deferred => "Deferred",
        TaskStatus::Completed => "Completed",
    }
}

fn task_status_slug(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Any => "any",
        TaskStatus::Available => "available",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Deferred => "deferred",
        TaskStatus::Completed => "completed",
    }
}

fn window_slug(window: flowlist_core::model::DueWindow) -> &'static str {
    use flowlist_core::model::DueWindow;
    match window {
        DueWindow::Overdue => "overdue",
        DueWindow::Today => "today",
        DueWindow::Tomorrow => "tomorrow",
        DueWindow::Next7Days => "next7",
        DueWindow::Next30Days => "next30",
        DueWindow::Later => "later",
        DueWindow::NoDate => "none",
        DueWindow::On(_) => "on",
        DueWindow::Before(_) => "before",
        DueWindow::After(_) => "after",
    }
}

fn window_label(window: flowlist_core::model::DueWindow) -> String {
    use flowlist_core::model::DueWindow;
    match window {
        DueWindow::Overdue => "Overdue".to_string(),
        DueWindow::Today => "Today".to_string(),
        DueWindow::Tomorrow => "Tomorrow".to_string(),
        DueWindow::Next7Days => "Next 7 days".to_string(),
        DueWindow::Next30Days => "Next 30 days".to_string(),
        DueWindow::Later => "Later".to_string(),
        DueWindow::NoDate => "No date".to_string(),
        DueWindow::On(date) => format!("On {date}"),
        DueWindow::Before(date) => format!("Before {date}"),
        DueWindow::After(date) => format!("After {date}"),
    }
}

/// Draws a pairing QR as a grid of pixel-snapped modules. True black on white
/// keeps it readable in both themes and in a phone camera.
fn qr_code_block(payload: &str) -> Option<AnyElement> {
    let code = qrcodegen::QrCode::encode_text(payload, qrcodegen::QrCodeEcc::Medium).ok()?;
    let modules = code.size();
    let cell = 4.0;
    let mut grid = div()
        .flex()
        .flex_wrap()
        .w(px(modules as f32 * cell))
        .h(px(modules as f32 * cell));
    for y in 0..modules {
        for x in 0..modules {
            grid = grid.child(div().w(px(cell)).h(px(cell)).flex_shrink_0().bg(
                if code.get_module(x, y) {
                    gpui::black()
                } else {
                    gpui::white()
                },
            ));
        }
    }
    Some(
        div()
            .p(px(16.))
            .rounded_md()
            .bg(gpui::white())
            .child(grid)
            .into_any_element(),
    )
}
