//! The Flowlist window: state, commands, keyboard routing and layout.
//!
//! The GUI shares its entire domain layer with the CLI and MCP server. Every
//! edit goes through `Store::execute`, so undo, validation and cross-process
//! locking behave exactly the same everywhere.

mod background;
mod blur;
mod calendar;
mod content;
mod drag;
mod emoji;
mod favicon;
mod hotkey;
mod icons;
mod inspector;
mod media;
mod modals;
mod notifications;
mod perspective;
mod preview;
mod quick_capture;
mod reminder;
mod screen_picker;
mod session;
mod sidebar;
mod text;
mod theme;
mod update;
mod vim;
mod widgets;

mod actions {
    gpui::actions!(flowlist, [Quit]);
}

use actions::Quit;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{Datelike, Timelike};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Application, Bounds, ClipboardItem, Context,
    CursorStyle, FocusHandle, Focusable, IntoElement, KeyDownEvent, Keystroke, MouseButton,
    ObjectFit, Render, ScrollHandle, SharedString, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, ease_out_quint, img, point, prelude::*, px, size,
};

use flowlist_core::command::{
    Command, ContainerPlacement, NewProject, NewTask, Outcome, ProjectPatch, SettingsPatch,
    TagPatch, TaskPatch, TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, DateConflict, Db, OrderKey, Position};
use flowlist_core::ids::{ChecklistId, ContainerId, FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{
    BuiltinPerspective, GroupBy, Priority, ProjectKind, SortBy, TagStatus, TaskAlert,
};
use flowlist_core::parse::{parse_duration, parse_quick, parse_stamp};
use flowlist_core::paths::DataPaths;
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, format_duration, local_date, now};
use flowlist_core::{export, taskpaper};

use crate::app_icon::{self, AppIcon};
use background::AbstractStyle;
use drag::{DragFolder, DragProject, DragTask};
use icons::Icon;

pub use session::{FocusTarget, Session, View};
use theme::{Appearance, Theme};
use widgets::TipExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TaskStepMode {
    Parallel,
    Sequential,
    OneOff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusArea {
    Content,
    Sidebar,
    Capture,
    Search,
    Field,
    TagInput,
    Checklist,
    ChecklistList,
    Modal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditField {
    Title,
    Notes,
    Project,
    Defer,
    Planned,
    Due,
    Reminder,
    Estimate,
    Repeat,
}

impl EditField {
    fn date_field(self) -> Option<DateField> {
        match self {
            EditField::Defer => Some(DateField::Defer),
            EditField::Planned => Some(DateField::Planned),
            EditField::Due => Some(DateField::Due),
            EditField::Reminder => Some(DateField::Reminder),
            _ => None,
        }
    }

    fn next(self) -> Self {
        match self {
            EditField::Title => EditField::Notes,
            EditField::Notes => EditField::Project,
            EditField::Project => EditField::Defer,
            EditField::Defer => EditField::Planned,
            EditField::Planned => EditField::Due,
            EditField::Due => EditField::Reminder,
            EditField::Reminder => EditField::Estimate,
            EditField::Estimate => EditField::Repeat,
            EditField::Repeat => EditField::Title,
        }
    }

    fn previous(self) -> Self {
        match self {
            EditField::Title => EditField::Repeat,
            EditField::Notes => EditField::Title,
            EditField::Project => EditField::Notes,
            EditField::Planned => EditField::Defer,
            EditField::Defer => EditField::Project,
            EditField::Due => EditField::Planned,
            EditField::Reminder => EditField::Due,
            EditField::Estimate => EditField::Reminder,
            EditField::Repeat => EditField::Estimate,
        }
    }
}

/// The two pages of the single-task inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InspectorTab {
    Properties,
    Subtasks,
}

impl InspectorTab {
    fn slug(self) -> &'static str {
        match self {
            InspectorTab::Properties => "properties",
            InspectorTab::Subtasks => "subtasks",
        }
    }
}

/// The item whose notification schedule is being edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NotificationTarget {
    Task(TaskId),
    Project(ProjectId),
}

/// Fields in the multi-task inspector. They are staged locally and committed
/// together by the Apply button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BatchField {
    Project,
    Tags,
    Defer,
    Planned,
    Due,
    Estimate,
}

impl BatchField {
    const ALL: [BatchField; 6] = [
        BatchField::Project,
        BatchField::Tags,
        BatchField::Defer,
        BatchField::Planned,
        BatchField::Due,
        BatchField::Estimate,
    ];

    fn label(self) -> &'static str {
        match self {
            BatchField::Project => "Project",
            BatchField::Tags => "Tags",
            BatchField::Defer => "Defer",
            BatchField::Planned => "Planned",
            BatchField::Due => "Due",
            BatchField::Estimate => "Estimate",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            BatchField::Project => "Leave unchanged, or type a project path",
            BatchField::Tags => "Add tags, or -tag to remove",
            BatchField::Defer => "Leave unchanged, or clear",
            BatchField::Planned => "Leave unchanged, or clear",
            BatchField::Due => "Leave unchanged, or clear",
            BatchField::Estimate => "Leave unchanged, or clear",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            BatchField::Project => "project",
            BatchField::Tags => "tags",
            BatchField::Defer => "defer",
            BatchField::Planned => "planned",
            BatchField::Due => "due",
            BatchField::Estimate => "estimate",
        }
    }
}

impl Default for BatchField {
    fn default() -> Self {
        BatchField::Project
    }
}

#[derive(Clone, Default)]
struct BatchDraft {
    project: String,
    tags: String,
    defer: String,
    planned: String,
    due: String,
    estimate: String,
    /// None leaves priority untouched; Some(None) clears it.
    priority: Option<Option<Priority>>,
    /// None leaves the flag untouched.
    flagged: Option<bool>,
    error: Option<String>,
}

/// The two task dates that can be changed from the inspector's calendar
/// popover. Each has its own measured anchor so the popover opens under the
/// field it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DateField {
    Defer,
    Planned,
    Due,
    Reminder,
    /// A date typed into a perspective rule. `second` selects the end of a
    /// Between range; the rule id keeps the calendar attached to the right row.
    RuleDate {
        rule: u64,
        second: bool,
    },
}

/// The date popover is a two-step surface: pick a day on the calendar, then
/// (when a time is wanted) set it on a clock face.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum DatePickerStep {
    #[default]
    Calendar,
    Clock,
}

/// Which hand the clock face is editing. Picking an hour advances to minutes,
/// the way the Android time picker does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ClockUnit {
    #[default]
    Hour,
    Minute,
}

impl DateField {
    pub(super) fn edit_field(self) -> EditField {
        match self {
            DateField::Defer => EditField::Defer,
            DateField::Planned => EditField::Planned,
            DateField::Due => EditField::Due,
            DateField::Reminder => EditField::Reminder,
            DateField::RuleDate { .. } => EditField::Due,
        }
    }

    pub(super) fn slug(self) -> &'static str {
        match self {
            DateField::Defer => "defer",
            DateField::Planned => "planned",
            DateField::Due => "due",
            DateField::Reminder => "reminder",
            DateField::RuleDate { .. } => "rule-date",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            DateField::Defer => "Defer",
            DateField::Planned => "Planned",
            DateField::Due => "Due",
            DateField::Reminder => "Reminder",
            DateField::RuleDate { .. } => "Rule date",
        }
    }

    pub(super) fn placeholder(self) -> &'static str {
        match self {
            DateField::Defer => "No defer date",
            DateField::Planned => "No planned date",
            DateField::Due => "No due date",
            DateField::Reminder => "No reminder time",
            DateField::RuleDate { .. } => "Pick a date",
        }
    }

    /// The mark each date field carries in the inspector and on task rows.
    pub(super) fn icon(self) -> Icon {
        match self {
            DateField::Defer => Icon::DeferDate,
            DateField::Planned => Icon::PlannedDate,
            DateField::Due => Icon::DueDate,
            DateField::Reminder => Icon::Clock,
            DateField::RuleDate { .. } => Icon::Calendar,
        }
    }
}

pub(super) enum BufferKey {
    Ignored,
    Changed,
    Submit,
    Cancel,
    Tab,
}

/// Where to paint a dropdown below a measured chip. Root overlays are
/// positioned this way so the painted menu and its hitboxes agree.
fn menu_position(
    anchor: Bounds<gpui::Pixels>,
    viewport: (f32, f32),
    width: f32,
    height: f32,
) -> (f32, f32) {
    let x = f32::from(anchor.origin.x);
    let y = f32::from(anchor.origin.y) + f32::from(anchor.size.height) + 4.0;
    (
        x.min((viewport.0 - width).max(0.0)).max(0.0),
        y.min((viewport.1 - height).max(0.0)).max(0.0),
    )
}

/// `1 task` / `3 tasks`. Prose in the header and toasts reads as words
/// rather than as a template with an optional bracket.
fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

/// Fade a freshly mounted overlay in. 120 ms reads as motion without making
/// a menu feel late, and the animation is keyed by id so it runs once per
/// open rather than on every render.
fn fade_in<E>(id: &'static str, element: E) -> impl IntoElement
where
    E: IntoElement + Styled + 'static,
{
    element.with_animation(
        id,
        Animation::new(Duration::from_millis(120)).with_easing(ease_out_quint()),
        |element, delta| element.opacity(delta),
    )
}

/// Size of the inspector's date popover. The constants are shared with the
/// root overlay so its hitboxes match the painted card.
pub(super) const DATE_PICKER_WIDTH: f32 = 288.0;
pub(super) const DATE_PICKER_HEIGHT: f32 = 440.0;

/// The first day of a date's month; the calendar paints whole months.
fn first_of_month(date: chrono::NaiveDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

/// The stamp the picker will apply: a chosen day plus either the staged time
/// or nothing, which keeps the date all-day.
fn picker_stamp(date: chrono::NaiveDate, time: Option<chrono::NaiveTime>) -> Stamp {
    match time {
        Some(time) => Stamp::on(date, time),
        None => Stamp::date_only(date),
    }
}

/// The time a picker starts from when the field is currently all-day. It uses
/// the library's due/defer default, falling back to 9 AM when there is none.
fn picker_default_time(
    field: DateField,
    due: Option<chrono::NaiveTime>,
    defer: Option<chrono::NaiveTime>,
) -> chrono::NaiveTime {
    let fallback = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default();
    match field {
        DateField::Due | DateField::Planned => due.unwrap_or(fallback),
        DateField::Defer => defer.unwrap_or(fallback),
        DateField::Reminder => due.unwrap_or(fallback),
        DateField::RuleDate { .. } => due.unwrap_or(fallback),
    }
}

/// A time step for the picker. `NaiveTime` wraps at midnight, so stepping past
/// 11:45 PM lands on 12:00 AM rather than failing.
fn stepped_time(time: chrono::NaiveTime, minutes: i64) -> chrono::NaiveTime {
    time + chrono::Duration::minutes(minutes)
}

/// A 24-hour hour as the clock face shows it: midnight and noon both read 12.
fn clock_hour_12(hour: u32) -> u32 {
    match hour % 12 {
        0 => 12,
        hour => hour,
    }
}

/// Turn a clock-face hour and half of day back into a 24-hour hour.
fn clock_hour_24(hour12: u32, pm: bool) -> u32 {
    let hour12 = hour12.clamp(1, 12);
    match (hour12, pm) {
        (12, false) => 0,
        (12, true) => 12,
        (hour, false) => hour,
        (hour, true) => hour + 12,
    }
}

/// Where a clock-face number sits inside a square dial. Index 0 is 12 at the
/// top; the rest run clockwise.
fn clock_dial_position(index: usize, center: f32, radius: f32) -> (f32, f32) {
    let angle = (index as f32 * 30.0 - 90.0).to_radians();
    (center + radius * angle.cos(), center + radius * angle.sin())
}

/// A calendar popover hangs below its icon and right-aligns to it, so it
/// stays inside the inspector instead of half off the window edge.
fn date_picker_position(
    anchor: Bounds<gpui::Pixels>,
    viewport: (f32, f32),
    width: f32,
    height: f32,
) -> (f32, f32) {
    let x = f32::from(anchor.origin.x) + f32::from(anchor.size.width) - width;
    let y = f32::from(anchor.origin.y) + f32::from(anchor.size.height) + 4.0;
    (
        x.min((viewport.0 - width).max(0.0)).max(0.0),
        y.min((viewport.1 - height).max(0.0)).max(0.0),
    )
}

/// One task row painted on the last frame: id, outline depth, title, and
/// whether the row has children (so a probe knows a disclosure belongs there).
pub(super) type PaintedTaskRow = (String, u32, String, bool);

/// The two fixed panes around the content. The title-bar search and Forecast
/// calendar line up with the content pane, so they read these instead of
/// hard-coding their own copies.
pub(super) const SIDEBAR_WIDTH: f32 = 240.0;
pub(super) const INSPECTOR_WIDTH: f32 = 320.0;

struct QuickOpenState {
    query: text::TextBuffer,
    cursor: usize,
}

#[derive(Clone)]
enum QuickOpenAction {
    Open(sidebar::SidebarTarget),
    Focus(FocusTarget),
    ClearFocus,
}

impl QuickOpenAction {
    fn key(&self) -> String {
        match self {
            QuickOpenAction::Open(target) => target.key(),
            QuickOpenAction::Focus(target) => format!("focus:{}", target.key()),
            QuickOpenAction::ClearFocus => "focus:clear".to_string(),
        }
    }
}

struct QuickOpenEntry {
    label: String,
    detail: String,
    icon: Icon,
    action: QuickOpenAction,
    rank: u8,
}

pub struct FlowlistApp {
    store: Store,
    theme: Theme,
    /// Alternate Dock icon, saved with the session.
    app_icon: AppIcon,
    view: View,
    /// Projects and folders scoping every perspective.
    focus_targets: Vec<FocusTarget>,
    /// The last focus, restored by the header eye.
    last_focus_targets: Vec<FocusTarget>,
    /// Keep the focus banner visible while focused.
    show_focus_banner: bool,
    /// Show button titles under the header's icon controls.
    show_toolbar_labels: bool,
    /// When the banner is disabled, a freshly enabled focus shows it briefly.
    focus_banner_until: Option<Instant>,
    /// Projects and folders Cmd-clicked in the sidebar, waiting for a command.
    sidebar_selection: Vec<FocusTarget>,
    /// First day of the month the Forecast calendar shows.
    forecast_month: chrono::NaiveDate,
    /// Monday of the week the collapsed Forecast strip shows.
    forecast_week: chrono::NaiveDate,
    /// A clicked calendar date; `None` shows the normal Forecast buckets.
    forecast_date: Option<chrono::NaiveDate>,
    /// The Past tile is selected.
    forecast_past: bool,
    /// The Future tile is selected.
    forecast_future: bool,
    /// Whether any Forecast calendar is showing.
    forecast_calendar_open: bool,
    /// Whether Forecast lists items on their planned date.
    forecast_planned: bool,
    /// Whether the open calendar is the full month grid; otherwise it is the
    /// compact week strip.
    forecast_calendar_expanded: bool,
    /// The height the user dragged the Forecast month grid to. `None` keeps
    /// the grid at its natural size.
    forecast_calendar_height: Option<f32>,
    /// A debounce flag for the session write that follows a calendar resize.
    forecast_height_dirty: bool,
    /// Forecast day cells and task chips measured for the GUI probe.
    forecast_day_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    forecast_chip_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Collapsed Forecast week cells and header controls, measured for the
    /// GUI probe.
    forecast_strip_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    forecast_strip_next_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    forecast_strip_expand_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    forecast_calendar_collapse_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The box the Forecast calendar painted on the last frame, so a GUI
    /// probe can confirm the block spans the pane.
    forecast_calendar_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The header chip that folds the Forecast calendar away.
    forecast_toggle_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The bottom grip that resizes the Forecast calendar.
    forecast_grip_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// Sidebar row boxes keyed by target, so a GUI probe can drag a project
    /// onto a folder without guessing at coordinates.
    sidebar_row_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Painted boxes for the start of each sidebar row's name (its icon), plus
    /// `empty:<parent>` for the "no projects" note, so a probe can check that
    /// nested rows are indented under their folder.
    sidebar_name_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// The title-bar back and forward arrows, measured so a GUI probe can
    /// click them instead of guessing at the header layout.
    history_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Settings-sheet sidebar toggles, measured for the same reason.
    settings_toggle_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// The focus banner, measured so a probe can clear focus by clicking it.
    focus_banner_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The header eye that clears or restores focus.
    focus_eye_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// Collapsible sidebar headings, measured so a probe can fold a section.
    sidebar_section_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Sidebar count badges keyed by row, so a probe can see which numbers a
    /// given Settings choice actually paints.
    sidebar_count_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// The number painted in each badge, so a probe can assert what a folder
    /// or tag row actually counts, not just that it painted something.
    sidebar_count_values: Rc<RefCell<BTreeMap<String, usize>>>,
    /// Which list the window is showing, and the views it came through. The
    /// history stores session keys so a deleted project can be skipped when
    /// stepping back instead of bringing the app down.
    view_history: Vec<String>,
    view_history_index: usize,
    /// The sidebar row under the pointer while a project is being dragged, so
    /// that row can highlight and the test state can report the drop target.
    /// A project drag owns the sidebar highlight only while it is live.
    sidebar_drag_target: Option<String>,
    /// The root-level gap under the pointer while a task is dragged in the
    /// middle pane, so the drop line can show where it will land.
    content_drop_target: Option<String>,
    /// Task rows painted on the last frame, in order, with their depth. The
    /// list reflects which subtasks are visible, not just what the query
    /// matched.
    content_rows: Rc<RefCell<Vec<PaintedTaskRow>>>,
    /// Group header boxes on the last frame, keyed by group key.
    content_group_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Task row boxes, keyed by task id.
    content_row_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Forecast project row boxes, keyed by project id.
    content_project_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Site favicons keyed by host, fetched once per session.
    favicons: Rc<RefCell<BTreeMap<String, Arc<gpui::RenderImage>>>>,
    /// Hosts currently being fetched, so a row does not ask twice.
    favicon_requests: Rc<RefCell<BTreeSet<String>>>,
    /// Hosts that answered without a usable icon; never retried this session.
    favicon_misses: Rc<RefCell<BTreeSet<String>>>,
    /// Subtask disclosure chevrons, measured so the GUI probe can click them.
    task_disclosures: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Inspector subtask rows, measured so a probe can drag one onto another.
    inspector_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Applies to every list, and is what the sidebar priority rows set.
    priority_filter: Option<Priority>,
    /// Hide everything blocked behind another task (First Available).
    only_available: bool,
    /// Today's header toggle: leave past plans and overdue work out.
    hide_past: bool,
    /// The message the content pane painted for an empty list, so a probe can
    /// check the calm Today copy without reading pixels.
    empty_note: Rc<RefCell<Option<String>>>,
    /// Keep the theme family, but let the OS pick light or dark.
    follow_system: bool,
    /// Per-perspective sort/group/filter memory.
    view_options: std::collections::BTreeMap<String, session::ViewOptions>,
    /// The header dropdowns, which list every option instead of cycling.
    sort_menu_open: bool,
    group_menu_open: bool,
    priority_menu_open: bool,
    /// Where the header chips painted on the last frame. The menus are root
    /// overlays positioned from these bounds, so their hitboxes line up with
    /// what is on screen (anchored/deferred paint offsets do not move
    /// hitboxes in GPUI 0.2).
    sort_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// Today's Hide past chip, measured so a GUI probe can click it.
    hide_past_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    group_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    priority_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The first text field in the open sheet, measured so a GUI probe can
    /// click and drag inside it.
    modal_field_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The open inspector field editor, measured so a probe can confirm the
    /// title editor wraps instead of clipping its caret.
    field_editor_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The inspector's add-subtask field, measured so a probe can click it.
    checklist_add_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The Due and Defer field rows, measured so their calendar popover can
    /// paint under the right field.
    due_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    defer_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    planned_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    reminder_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    rule_date_anchors: Rc<RefCell<BTreeMap<String, Bounds<gpui::Pixels>>>>,
    /// Which date field's calendar is open, the month on screen and the day
    /// the arrow keys point at. `date_picker_time` is the time that will be
    /// written with the day, or None for an all-day date.
    date_picker: Option<DateField>,
    date_picker_month: chrono::NaiveDate,
    date_picker_cursor: chrono::NaiveDate,
    date_picker_time: Option<chrono::NaiveTime>,
    /// Which half of the two-step popover is showing, and which hand the clock
    /// is editing.
    date_picker_step: DatePickerStep,
    date_picker_unit: ClockUnit,
    /// Controls in the date popover, measured so the GUI probe can click them.
    date_picker_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Theme swatches and appearance chips in Settings, so a probe can click
    /// a family or light/dark without guessing at the grid.
    theme_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// The perspective editor's open select, the row the arrow keys point at,
    /// and where each select painted so its menu can be a root overlay.
    view_menu: Option<perspective::ViewField>,
    view_menu_cursor: usize,
    view_menu_scroll: ScrollHandle,
    rule_menu_scroll: ScrollHandle,
    /// Where editor controls painted, so a GUI probe can click them without
    /// guessing at pixels.
    view_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Archive project rows and their Reopen buttons, measured so a probe can
    /// open a finished project or bring it back without pixel guessing.
    archive_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// How wide the title's slot painted on the last frame. A view name that
    /// does not fit there is left out entirely instead of showing a stub.
    title_space: Rc<Cell<Option<f32>>>,
    /// Whether the last frame painted the view title. Exposed through the
    /// test-state file so the layout rule can be checked without a screenshot.
    title_visible: Rc<Cell<bool>>,
    /// The right-click menu on a sidebar row, opened at the mouse position.
    sidebar_menu: Option<sidebar::SidebarMenu>,
    /// Boxes for the items in the open sidebar menu, keyed by item id.
    sidebar_menu_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// A sheet to open after the current one commits, used when an edit needs
    /// a follow-up date confirmation.
    pending_modal: Option<modals::Modal>,
    /// The most recent command refusal. Callers with an inline surface show it
    /// there and clear it; otherwise it falls back to the toast.
    command_error: Option<String>,
    /// A refusal shown inside the open sheet, under its title.
    modal_error: Option<String>,
    /// A date warning shown under the inspector field that caused it.
    date_error: Option<(TaskId, DateField, String)>,
    /// The multi-task inspector's staged values.
    batch_draft: BatchDraft,
    batch_editing: Option<BatchField>,
    /// Which selection the staged batch belongs to.
    batch_selection: Vec<TaskId>,
    /// Batch inspector controls, measured for GUI probes.
    batch_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Notification inspector controls, measured for GUI probes.
    notification_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Calendar event agenda rows and month-grid chips, measured for probes.
    calendar_event_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    calendar_chip_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Inspector section headers, measured so a probe can collapse one.
    inspector_section_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Explicit inspector disclosure choices: true means collapsed. Sections
    /// absent from the map use their content-aware default.
    inspector_sections: BTreeMap<String, bool>,
    /// The active single-task inspector tab.
    inspector_tab: InspectorTab,
    /// The open "Add Notification" menu and the control it points at.
    notification_menu: Option<NotificationTarget>,
    notification_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The open repeat-unit menu and the interval button it points at.
    repeat_unit_menu: Option<TaskId>,
    repeat_unit_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// A click's mouse-up closed the sort or group menu. The chip under the
    /// same click must not immediately reopen it.
    sort_closed_on_up: bool,
    group_closed_on_up: bool,
    priority_closed_on_up: bool,
    /// Highlighted completion in the project and tag fields.
    suggestion_index: usize,
    /// True once the user moved through the project dropdown with the arrow
    /// keys. Until then Tab still advances to the next field instead of
    /// assigning the first suggestion.
    suggestion_engaged: bool,
    selected: Vec<TaskId>,
    anchor: Option<TaskId>,
    expanded: BTreeSet<String>,
    collapsed_groups: BTreeSet<String>,
    show_inspector: bool,
    show_completed: bool,
    /// Finished subtasks stay under their parent, struck through, even when
    /// the active view hides completed work.
    keep_completed_subtasks: bool,
    /// Operate the sidebar like a to-do list: a saved view whose filter
    /// matches nothing disappears until it has work again.
    hide_empty_views: bool,
    vim_mode: bool,
    /// Send macOS notifications for upcoming due dates (this device only).
    due_notifications: bool,
    /// Show macOS Calendar events in Forecast. Per device.
    calendar_events_enabled: bool,
    /// Read-only events for the visible Forecast range.
    calendar_events: Vec<calendar::Event>,
    calendar_calendars: Vec<calendar::CalendarInfo>,
    calendar_hidden: BTreeSet<String>,
    calendar_permission: calendar::Permission,
    calendar_last_refresh: Option<Instant>,
    /// Show only due-date entries in Forecast.
    forecast_due_only: bool,
    /// Show a full-screen reminder when a task comes due while the app runs.
    fullscreen_reminders: bool,
    /// Pause system media while a full-screen reminder is on screen.
    pause_media_on_reminder: bool,
    /// Due instants already shown, so a full-screen reminder fires once.
    reminder_seen: BTreeSet<String>,
    /// Tasks snoozed from the reminder, and when to show them again.
    reminder_snoozed: BTreeMap<TaskId, chrono::DateTime<chrono::Utc>>,
    /// Whether a full-screen reminder window is open.
    reminder_open: bool,
    /// The backdrop of the last test reminder, for the GUI probe.
    last_reminder_backdrop: Option<String>,
    /// Abstract backdrop for full-screen reminders.
    reminder_background: AbstractStyle,
    /// Follow the theme's default abstract backdrop.
    reminder_match_theme: bool,
    /// Custom reminder wallpaper and how strongly it shows.
    reminder_wallpaper_light: Option<PathBuf>,
    reminder_wallpaper_dark: Option<PathBuf>,
    /// Show a different reminder wallpaper in light and dark.
    reminder_wallpaper_separate: bool,
    reminder_use_wallpaper: bool,
    reminder_wallpaper_opacity: f32,
    reminder_snooze_minutes: u32,
    /// Custom wallpaper behind the main window, one per appearance.
    wallpaper_light: Option<PathBuf>,
    wallpaper_dark: Option<PathBuf>,
    /// Show a different wallpaper in light and dark; otherwise one is shared.
    wallpaper_separate: bool,
    use_wallpaper: bool,
    wallpaper_opacity: f32,
    notification_permission: notifications::Permission,
    /// How many reminders the last scheduling pass added.
    notification_scheduled: usize,
    /// Store revision reflected in the pending notifications.
    notified_revision: Option<u64>,
    /// Whether the current launch already asked for notification permission.
    notification_requested: bool,
    vim: vim::VimState,
    sidebar_cursor: Option<sidebar::SidebarTarget>,
    /// The ⌘O navigation palette. `None` when it is closed.
    quick_open: Option<QuickOpenState>,
    /// Keeps the highlighted Quick Open row inside the palette viewport.
    quick_open_scroll: ScrollHandle,
    /// The sidebar row under the pointer, if any. Hover-only row actions
    /// render only for this row so an idle row spends no width on them.
    checklist_cursor: Option<usize>,
    focus: FocusArea,
    focus_handle: FocusHandle,
    capture: text::TextBuffer,
    /// Opt the next inline capture into a full-screen reminder.
    search: text::TextBuffer,
    /// An image or PDF shown over the list, closed with Esc.
    preview: Option<preview::Preview>,
    /// The last update check, shown in Settings → About.
    update: update::UpdateState,
    /// Whether the search field filters the current view or matches across the
    /// whole library. Transient: a new session starts on the current view.
    search_library: bool,
    field: text::TextBuffer,
    tag_buffer: text::TextBuffer,
    checklist_buffer: text::TextBuffer,
    checklist_editing: Option<ChecklistId>,
    editing: Option<EditField>,
    field_error: Option<String>,
    /// The task created by `o`/`O` while its title is still being typed.
    /// Inspector actions stay out of the way until the title is committed.
    draft_task: Option<TaskId>,
    modal: Option<modals::Modal>,
    /// The theme the Settings sheet opened with. Theme and appearance clicks
    /// preview on the live window; Cancel puts this one back.
    theme_on_open: Option<Theme>,
    /// Plain's custom accent per appearance. Other families ignore these and
    /// keep their own accent.
    plain_accent_light: Option<u32>,
    plain_accent_dark: Option<u32>,
    /// The accent pair the Settings sheet opened with, restored on Cancel.
    accent_on_open: Option<(Option<u32>, Option<u32>)>,
    modal_focus: usize,
    modal_scroll: ScrollHandle,
    /// Last size chosen by dragging the Settings sheet's corner. `None` uses
    /// the width and height that fit the current window.
    settings_size: Option<(f32, f32)>,
    perspective_size: Option<(f32, f32)>,
    /// Where the pointer grabbed the Settings scrollbar thumb, in points.
    scroll_drag_grab: Rc<Cell<Option<f32>>>,
    /// Where the pointer grabbed the inspector scrollbar thumb, in points.
    inspector_scroll_drag_grab: Rc<Cell<Option<f32>>>,
    toast: Option<String>,
    /// Last measured content size, persisted so the next launch reopens at it.
    window_size: Option<(f32, f32)>,
    window_dirty: bool,
    /// The message currently on screen and when it appeared. A toast retires
    /// itself after a few seconds so the window never keeps stale advice.
    toast_shown: Option<(String, Instant)>,
    /// The tooltip whose element the pointer is over, and the label a click
    /// muted. A click quiets the tooltip until the pointer leaves the button,
    /// so a resting pointer does not bring it straight back.
    hovered_tip: Option<SharedString>,
    suppressed_tip: Option<SharedString>,
    sort: Option<SortBy>,
    group: Option<GroupBy>,
    scroll: ScrollHandle,
    sidebar_scroll: ScrollHandle,
    inspector_scroll: ScrollHandle,
    last_sync: Instant,
    /// Set by `FLOWLIST_TEST_STATE`; the window writes a small JSON state file
    /// for the GUI smoke test. Off in normal use.
    test_state_path: Option<PathBuf>,
    test_state_last: Option<String>,
    /// The appearance the window glass was last pinned to, if the platform had
    /// a blur view to tune.
    blur_mode: Option<theme::Appearance>,
    /// Show the macOS close/miniaturize/zoom buttons over the header.
    traffic_lights_visible: bool,
    /// What was last pushed to AppKit. `None` retries each frame until the
    /// window has a titlebar to move.
    traffic_lights_applied: Option<bool>,
    /// The window-buttons choice the Settings sheet opened with, restored on
    /// Cancel like the theme preview.
    traffic_lights_on_open: Option<bool>,
}

/// Plain keeps one accent per appearance so each background can carry its own
/// tone, but an accent chosen for one mode should not disappear in the other.
/// When only one side has been set, the other side inherits it, clamped by the
/// theme builder so text and button labels stay readable on that background.
fn accent_for(appearance: theme::Appearance, light: Option<u32>, dark: Option<u32>) -> Option<u32> {
    match appearance {
        theme::Appearance::Light => {
            light.or_else(|| dark.map(|value| theme::readable_plain_accent(value, appearance)))
        }
        theme::Appearance::Dark => {
            dark.or_else(|| light.map(|value| theme::readable_plain_accent(value, appearance)))
        }
    }
}

impl FlowlistApp {
    fn new(store: Store, cx: &mut Context<Self>) -> Self {
        let session: Session = store.read_session().unwrap_or_default();
        let mut view = session.view(store.db());
        if !view_exists(store.db(), &view) {
            view = View::Builtin(BuiltinPerspective::Inbox);
        }
        let mut focus_targets: Vec<FocusTarget> = session
            .focus_targets
            .iter()
            .filter_map(|key| FocusTarget::from_key(store.db(), key))
            .collect();
        if focus_targets.is_empty()
            && let Some(key) = session.focus.as_deref()
            && let Some(target) = FocusTarget::from_key(store.db(), key)
        {
            focus_targets.push(target);
        }
        focus_targets.dedup_by(|left, right| left.key() == right.key());
        let last_focus_targets: Vec<FocusTarget> = session
            .last_focus_targets
            .iter()
            .filter_map(|key| FocusTarget::from_key(store.db(), key))
            .collect();
        let show_focus_banner = session.focus_banner.unwrap_or(true);
        let show_toolbar_labels = session.toolbar_labels.unwrap_or(false);
        if let Some(target) = focus_targets.first()
            && matches!(view, View::Project(_) | View::Folder(_))
            && !focus_targets
                .iter()
                .any(|target| target.allows_view(store.db(), &view))
        {
            view = target.view();
        }
        let selected = session
            .selected_tasks(store.db())
            .into_iter()
            .filter(|id| store.db().tasks.contains_key(id))
            .filter(|id| {
                focus_targets.is_empty()
                    || focus_targets
                        .iter()
                        .any(|target| target.allows_task(store.db(), id))
            })
            .collect();
        let (theme_kind, theme_appearance) = session.theme_choice();
        let plain_accent_light = session.plain_accent(theme::Appearance::Light);
        let plain_accent_dark = session.plain_accent(theme::Appearance::Dark);
        let theme = Theme::with_accent(
            theme_kind,
            theme_appearance,
            match theme_appearance {
                theme::Appearance::Light => plain_accent_light,
                theme::Appearance::Dark => plain_accent_dark,
            },
        );
        let sort = session.sort.as_deref().and_then(sort_from_slug);
        let group = session.group.as_deref().and_then(group_from_slug);
        let show_completed = session.show_completed.unwrap_or(false);
        let keep_completed_subtasks = session.keep_completed_subtasks.unwrap_or(false);
        let hide_empty_views = session.hide_empty_views.unwrap_or(false);
        let show_inspector = session.inspector.unwrap_or(true);
        let vim_mode = session.vim_mode.unwrap_or(true);
        let due_notifications = session.notifications.unwrap_or(false);
        let calendar_events_enabled = session.calendar_events.unwrap_or(false);
        let calendar_hidden = session.calendar_hidden.clone();
        let forecast_due_only = session.forecast_due_only.unwrap_or(false);
        let fullscreen_reminders = session.fullscreen_reminders.unwrap_or(true);
        let pause_media_on_reminder = session.pause_media_on_reminder.unwrap_or(false);
        let reminder_match_theme = session.reminder_match_theme.unwrap_or(true);
        let reminder_background = session
            .reminder_background
            .as_deref()
            .and_then(AbstractStyle::from_slug)
            .unwrap_or_else(|| AbstractStyle::for_theme(theme.kind()));
        let legacy_reminder_wallpaper = session
            .reminder_wallpaper
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.exists());
        let reminder_wallpaper_light = session
            .reminder_wallpaper_light
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| legacy_reminder_wallpaper.clone())
            .filter(|path| path.exists());
        let reminder_wallpaper_dark = session
            .reminder_wallpaper_dark
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| legacy_reminder_wallpaper)
            .filter(|path| path.exists());
        let reminder_wallpaper_separate =
            session.reminder_wallpaper_separate.unwrap_or_else(|| {
                reminder_wallpaper_light.is_some()
                    && reminder_wallpaper_dark.is_some()
                    && reminder_wallpaper_light != reminder_wallpaper_dark
            });
        let reminder_use_wallpaper = session.reminder_use_wallpaper.unwrap_or(false);
        let reminder_wallpaper_opacity = session
            .reminder_wallpaper_opacity
            .unwrap_or(0.35)
            .clamp(0.05, 0.95);
        let reminder_snooze_minutes = session
            .fullscreen_snooze_minutes
            .unwrap_or(15)
            .clamp(1, 1440);
        let legacy_wallpaper = session
            .wallpaper
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.exists());
        let wallpaper_light = session
            .wallpaper_light
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| legacy_wallpaper.clone())
            .filter(|path| path.exists());
        let wallpaper_dark = session
            .wallpaper_dark
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| legacy_wallpaper)
            .filter(|path| path.exists());
        let wallpaper_separate = session.wallpaper_separate.unwrap_or_else(|| {
            wallpaper_light.is_some()
                && wallpaper_dark.is_some()
                && wallpaper_light != wallpaper_dark
        });
        let use_wallpaper = session.use_wallpaper.unwrap_or(false);
        let wallpaper_opacity = session.wallpaper_opacity.unwrap_or(0.35).clamp(0.05, 0.95);
        let app_icon = session
            .app_icon
            .as_deref()
            .and_then(AppIcon::from_slug)
            .unwrap_or_default();
        let priority_filter = session.priority_filter();
        let only_available = session.only_available.unwrap_or(false);
        let follow_system = session.follow_system.unwrap_or(false);
        let traffic_lights_visible = session.traffic_lights.unwrap_or(true);
        let view_options = session.view_options.clone();
        let window_size = session.window;
        let focus_handle = cx.focus_handle();
        let toast = store_warning(&store);
        let initial_view = view.key();
        notifications::install();
        notifications::refresh_permission();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update(cx, |this, cx| {
                    if this.due_notifications
                        || matches!(this.modal, Some(modals::Modal::Settings { .. }))
                    {
                        notifications::refresh_permission();
                    }
                    if this.store.reload_if_changed().unwrap_or(false) {
                        this.prune_selection();
                        this.prune_focus();
                        cx.notify();
                    }
                    if this.calendar_events_enabled
                        || matches!(this.modal, Some(modals::Modal::Settings { .. }))
                    {
                        calendar::refresh_permission();
                    }
                    this.tick_calendar(cx);
                    this.sync_notifications(cx);
                    this.check_fullscreen_reminders(cx);
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        let today = local_date(now());
        let forecast_month =
            chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
        let forecast_calendar_open = session.forecast_calendar_open.unwrap_or(true);
        let forecast_planned = session.forecast_planned.unwrap_or(true);
        let forecast_calendar_expanded = session.forecast_calendar_expanded.unwrap_or(true);
        let forecast_calendar_height = session.forecast_calendar_height;

        let mut app = Self {
            store,
            theme,
            app_icon,
            view,
            focus_targets,
            last_focus_targets,
            show_focus_banner,
            show_toolbar_labels,
            focus_banner_until: None,
            sidebar_selection: Vec::new(),
            forecast_month,
            forecast_week: {
                let today = local_date(now());
                let monday = chrono::Datelike::weekday(&today).num_days_from_monday() as i64;
                today - chrono::Duration::days(monday)
            },
            forecast_date: None,
            forecast_past: false,
            forecast_future: false,
            forecast_calendar_open,
            forecast_planned,
            forecast_calendar_expanded,
            forecast_calendar_height,
            forecast_height_dirty: false,
            forecast_day_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            forecast_chip_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            forecast_strip_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            forecast_strip_next_anchor: Rc::new(Cell::new(None)),
            forecast_strip_expand_anchor: Rc::new(Cell::new(None)),
            forecast_calendar_collapse_anchor: Rc::new(Cell::new(None)),
            forecast_calendar_anchor: Rc::new(Cell::new(None)),
            forecast_toggle_anchor: Rc::new(Cell::new(None)),
            forecast_grip_anchor: Rc::new(Cell::new(None)),
            sidebar_row_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sidebar_name_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            history_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            settings_toggle_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            focus_banner_anchor: Rc::new(Cell::new(None)),
            focus_eye_anchor: Rc::new(Cell::new(None)),
            sidebar_section_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sidebar_count_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sidebar_count_values: Rc::new(RefCell::new(BTreeMap::new())),
            view_history: vec![initial_view],
            view_history_index: 0,
            sidebar_drag_target: None,
            content_drop_target: None,
            content_rows: Rc::new(RefCell::new(Vec::new())),
            content_group_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            content_row_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            content_project_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            favicons: Rc::new(RefCell::new(BTreeMap::new())),
            favicon_requests: Rc::new(RefCell::new(BTreeSet::new())),
            favicon_misses: Rc::new(RefCell::new(BTreeSet::new())),
            task_disclosures: Rc::new(RefCell::new(BTreeMap::new())),
            inspector_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            priority_filter,
            only_available,
            hide_past: false,
            empty_note: Rc::new(RefCell::new(None)),
            follow_system,
            view_options,
            sort_menu_open: false,
            group_menu_open: false,
            priority_menu_open: false,
            sort_anchor: Rc::new(Cell::new(None)),
            hide_past_anchor: Rc::new(Cell::new(None)),
            group_anchor: Rc::new(Cell::new(None)),
            priority_anchor: Rc::new(Cell::new(None)),
            modal_field_anchor: Rc::new(Cell::new(None)),
            field_editor_anchor: Rc::new(Cell::new(None)),
            checklist_add_anchor: Rc::new(Cell::new(None)),
            due_anchor: Rc::new(Cell::new(None)),
            defer_anchor: Rc::new(Cell::new(None)),
            planned_anchor: Rc::new(Cell::new(None)),
            reminder_anchor: Rc::new(Cell::new(None)),
            rule_date_anchors: Rc::new(RefCell::new(BTreeMap::new())),
            date_picker: None,
            date_picker_month: forecast_month,
            date_picker_cursor: today,
            date_picker_time: None,
            date_picker_step: DatePickerStep::Calendar,
            date_picker_unit: ClockUnit::Hour,
            date_picker_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            theme_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            view_menu: None,
            view_menu_cursor: 0,
            view_menu_scroll: ScrollHandle::new(),
            rule_menu_scroll: ScrollHandle::new(),
            view_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            archive_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            title_space: Rc::new(Cell::new(None)),
            title_visible: Rc::new(Cell::new(true)),
            sidebar_menu: None,
            sidebar_menu_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sort_closed_on_up: false,
            group_closed_on_up: false,
            priority_closed_on_up: false,
            suggestion_index: 0,
            suggestion_engaged: false,
            selected,
            anchor: None,
            expanded: BTreeSet::new(),
            collapsed_groups: BTreeSet::new(),
            show_inspector,
            show_completed,
            keep_completed_subtasks,
            hide_empty_views,
            vim_mode,
            due_notifications,
            calendar_events_enabled,
            calendar_events: Vec::new(),
            calendar_calendars: Vec::new(),
            calendar_hidden,
            calendar_permission: calendar::Permission::Unknown,
            calendar_last_refresh: None,
            forecast_due_only,
            fullscreen_reminders,
            pause_media_on_reminder,
            reminder_seen: BTreeSet::new(),
            reminder_snoozed: BTreeMap::new(),
            reminder_open: false,
            last_reminder_backdrop: None,
            reminder_background,
            reminder_match_theme,
            reminder_wallpaper_light,
            reminder_wallpaper_dark,
            reminder_wallpaper_separate,
            reminder_use_wallpaper,
            reminder_wallpaper_opacity,
            reminder_snooze_minutes,
            wallpaper_light,
            wallpaper_dark,
            wallpaper_separate,
            use_wallpaper,
            wallpaper_opacity,
            notification_permission: notifications::Permission::Unknown,
            notification_scheduled: 0,
            notified_revision: None,
            notification_requested: false,
            vim: vim::VimState::default(),
            sidebar_cursor: None,
            quick_open: None,
            quick_open_scroll: ScrollHandle::new(),
            checklist_cursor: None,
            focus: FocusArea::Content,
            focus_handle,
            capture: text::TextBuffer::empty(),
            search: text::TextBuffer::empty(),
            preview: None,
            update: update::UpdateState::Idle,
            search_library: false,
            field: text::TextBuffer::empty(),
            tag_buffer: text::TextBuffer::empty(),
            checklist_buffer: text::TextBuffer::empty(),
            checklist_editing: None,
            editing: None,
            field_error: None,
            draft_task: None,
            modal: None,
            pending_modal: None,
            command_error: None,
            modal_error: None,
            date_error: None,
            batch_draft: BatchDraft::default(),
            batch_editing: None,
            batch_selection: Vec::new(),
            batch_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            notification_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            calendar_event_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            calendar_chip_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            inspector_section_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            inspector_sections: session.inspector_sections.clone(),
            inspector_tab: InspectorTab::Properties,
            notification_menu: None,
            notification_anchor: Rc::new(Cell::new(None)),
            repeat_unit_menu: None,
            repeat_unit_anchor: Rc::new(Cell::new(None)),
            theme_on_open: None,
            plain_accent_light,
            plain_accent_dark,
            accent_on_open: None,
            modal_focus: 0,
            modal_scroll: ScrollHandle::new(),
            settings_size: session.settings_size,
            perspective_size: session.perspective_size,
            scroll_drag_grab: Rc::new(Cell::new(None)),
            inspector_scroll_drag_grab: Rc::new(Cell::new(None)),
            toast,
            window_size,
            window_dirty: false,
            toast_shown: None,
            hovered_tip: None,
            suppressed_tip: None,
            sort,
            group,
            scroll: ScrollHandle::new(),
            sidebar_scroll: ScrollHandle::new(),
            inspector_scroll: ScrollHandle::new(),
            last_sync: Instant::now(),
            test_state_path: std::env::var_os("FLOWLIST_TEST_STATE").map(PathBuf::from),
            test_state_last: None,
            blur_mode: None,
            traffic_lights_visible,
            traffic_lights_applied: None,
            traffic_lights_on_open: None,
        };
        app.restore_view_options();
        app_icon::apply(app_icon);
        calendar::refresh_permission();
        if app.calendar_events_enabled {
            app.refresh_calendar_events(cx);
        }
        app
    }

    // ------------------------------------------------------------- session

    /// Switch light/dark inside the current family. A manual switch opts out
    /// of following the system appearance; otherwise the next frame would put
    /// the old appearance straight back.
    fn toggle_appearance(&mut self) {
        // Rebuild through the saved accents instead of flipping the palette in
        // place: Plain stores one accent per appearance, and the previous
        // `Theme::toggled` dropped it on the floor.
        let appearance = self.theme.appearance().toggled();
        self.theme = self.build_theme(self.theme.kind(), appearance);
        self.follow_system = false;
        self.toast = Some(format!(
            "{} · {}",
            self.theme.kind().label(),
            self.theme.appearance().label()
        ));
        self.save_session();
    }

    /// Build the window palette from the saved family, mode and Plain accent.
    fn build_theme(&self, kind: theme::ThemeKind, appearance: theme::Appearance) -> Theme {
        let accent = accent_for(appearance, self.plain_accent_light, self.plain_accent_dark);
        Theme::with_accent(kind, appearance, accent)
    }

    /// The palette a Settings control should preview. While the sheet is open
    /// the draft accents live on the modal, so previews and the eventual Save
    /// always agree.
    pub(super) fn preview_theme(
        &self,
        kind: theme::ThemeKind,
        appearance: theme::Appearance,
    ) -> Theme {
        if let Some(modals::Modal::Settings {
            accent_light,
            accent_dark,
            ..
        }) = &self.modal
        {
            let accent = accent_for(appearance, *accent_light, *accent_dark);
            Theme::with_accent(kind, appearance, accent)
        } else {
            self.build_theme(kind, appearance)
        }
    }

    fn refresh_calendar_events(&mut self, cx: &mut Context<Self>) {
        self.calendar_permission = calendar::permission();
        if !self.calendar_events_enabled || !self.calendar_permission.allowed() {
            self.calendar_events.clear();
            self.calendar_calendars.clear();
            self.calendar_last_refresh = None;
            cx.notify();
            return;
        }
        let today = local_date(now());
        self.calendar_calendars = calendar::calendars();
        self.calendar_events = calendar::events(
            today - chrono::Duration::days(31),
            today + chrono::Duration::days(366),
        );
        self.calendar_events
            .retain(|event| !self.calendar_hidden.contains(&event.calendar_id));
        self.calendar_last_refresh = Some(Instant::now());
        cx.notify();
    }

    fn tick_calendar(&mut self, cx: &mut Context<Self>) {
        let status = calendar::permission();
        if status != self.calendar_permission {
            self.calendar_permission = status;
            cx.notify();
        }
        if status.allowed() && self.calendar_calendars.is_empty() {
            self.calendar_calendars = calendar::calendars();
            cx.notify();
        }
        if !self.calendar_events_enabled {
            return;
        }
        let stale = self
            .calendar_last_refresh
            .is_none_or(|refreshed| refreshed.elapsed() >= Duration::from_secs(300));
        if status.allowed() && stale {
            self.refresh_calendar_events(cx);
        }
    }

    /// Calendar events that start on one local day, sorted for display.
    pub(super) fn calendar_events_on(&self, date: chrono::NaiveDate) -> Vec<&calendar::Event> {
        let mut events: Vec<&calendar::Event> = self
            .calendar_events
            .iter()
            .filter(|event| event.local_date() == date)
            .collect();
        events.sort_by_key(|event| {
            (
                event.local_date(),
                !event.all_day,
                event.start,
                event.title.to_lowercase(),
            )
        });
        events
    }

    /// Keep the pending macOS reminders in step with the library, the toggle
    /// and the permission the user granted. Cheap when nothing changed: one
    /// revision comparison.
    fn sync_notifications(&mut self, cx: &mut Context<Self>) {
        let permission = notifications::permission();
        if permission != self.notification_permission {
            self.notification_permission = permission;
            cx.notify();
        }
        if self.due_notifications
            && permission == notifications::Permission::NotDetermined
            && !self.notification_requested
        {
            notifications::request_permission();
            self.notification_requested = true;
        }
        if !self.due_notifications || !permission.allowed() {
            if self.notified_revision.take().is_some() {
                notifications::clear();
                if self.notification_scheduled != 0 {
                    self.notification_scheduled = 0;
                    cx.notify();
                }
            }
            return;
        }
        let revision = self.store.revision();
        if self.notified_revision == Some(revision) {
            return;
        }
        let count = notifications::schedule(self.store.db());
        self.notified_revision = Some(revision);
        if self.notification_scheduled != count {
            self.notification_scheduled = count;
            cx.notify();
        }
    }

    /// The wallpaper the current appearance should draw, if any. Kept in one
    /// place so the content surface and the painted image never disagree.
    fn active_wallpaper(&self) -> Option<PathBuf> {
        if !self.use_wallpaper {
            return None;
        }
        if self.wallpaper_separate {
            match self.theme.appearance() {
                Appearance::Light => self.wallpaper_light.clone(),
                Appearance::Dark => self.wallpaper_dark.clone(),
            }
        } else {
            self.wallpaper_light.clone()
        }
    }

    fn reminder_backdrop(&self) -> reminder::ReminderBackdrop {
        let wallpaper = if self.reminder_wallpaper_separate {
            match self.theme.appearance() {
                Appearance::Light => self.reminder_wallpaper_light.clone(),
                Appearance::Dark => self.reminder_wallpaper_dark.clone(),
            }
        } else {
            self.reminder_wallpaper_light.clone()
        };
        if self.reminder_use_wallpaper
            && let Some(path) = wallpaper
        {
            return reminder::ReminderBackdrop::Wallpaper(path, self.reminder_wallpaper_opacity);
        }
        reminder::ReminderBackdrop::Abstract(if self.reminder_match_theme {
            AbstractStyle::for_theme(self.theme.kind())
        } else {
            self.reminder_background
        })
    }

    /// Open the reminder window with a sample task so the backdrop can be
    /// checked without waiting for a real due time.
    pub(super) fn send_fullscreen_test(
        &mut self,
        backdrop: reminder::ReminderBackdrop,
        snooze_minutes: u32,
        pause_media: bool,
        cx: &mut Context<Self>,
    ) {
        self.last_reminder_backdrop = Some(match &backdrop {
            reminder::ReminderBackdrop::Abstract(style) => {
                format!("abstract:{}", style.slug())
            }
            reminder::ReminderBackdrop::Wallpaper(path, opacity) => {
                format!("wallpaper:{}:{opacity}", path.display())
            }
        });
        let task = reminder::ReminderTask {
            id: TaskId::from_string("flowlist-test-reminder"),
            title: "Full-screen reminder test".to_string(),
            project: Some("Flowlist".to_string()),
            time: "09:00".to_string(),
            due_at: now(),
        };
        if reminder::show(
            cx.weak_entity(),
            self.theme,
            backdrop,
            snooze_minutes,
            pause_media,
            vec![task],
            cx,
        )
        .is_ok()
        {
            self.reminder_open = true;
        }
        cx.notify();
    }

    /// Open the full-screen reminder for anything that just came due. The
    /// check runs once a second from the same background tick that keeps
    /// notifications in step. Tasks that came due long before Flowlist opened
    /// are left to the normal notification rather than ambushing the user.
    fn check_fullscreen_reminders(&mut self, cx: &mut Context<Self>) {
        if !self.fullscreen_reminders {
            return;
        }
        let at = now();
        let grace = chrono::Duration::minutes(2);
        let mut due = Vec::new();
        let mut shown = Vec::new();
        {
            let db = self.store.db();
            for task in db.tasks.values() {
                if task.completed || task.is_trashed() {
                    continue;
                }
                let Some(stamp) = db.reminder_stamp(&task.id) else {
                    continue;
                };
                let due_at = notifications::reminder_at(stamp);
                if due_at > at {
                    continue;
                }
                let key = format!("{}:{}", task.id, due_at.timestamp());
                if self.reminder_seen.contains(&key) {
                    continue;
                }
                let snoozed_until = self.reminder_snoozed.get(&task.id).copied();
                if snoozed_until.is_some_and(|until| until > at) {
                    continue;
                }
                // A snoozed task is deliberately late; only unsnoozed tasks
                // use the short grace window.
                if snoozed_until.is_none() && at - due_at > grace {
                    continue;
                }
                if db
                    .effective_project(&task.id)
                    .and_then(|project| db.projects.get(&project))
                    .is_some_and(|project| project.status.is_closed())
                {
                    continue;
                }
                if !db.wants_fullscreen_reminder(&task.id) {
                    continue;
                }
                let project = db
                    .effective_project(&task.id)
                    .map(|project| db.project_path(&project));
                let time = if stamp.has_time {
                    due_at
                        .with_timezone(&chrono::Local)
                        .format("%H:%M")
                        .to_string()
                } else {
                    "09:00".to_string()
                };
                due.push(reminder::ReminderTask {
                    id: task.id.clone(),
                    title: task.title.clone(),
                    project,
                    time,
                    due_at,
                });
                shown.push(key);
            }
        }
        if due.is_empty() {
            return;
        }
        for key in shown {
            self.reminder_seen.insert(key);
        }
        for task in &due {
            self.reminder_snoozed.remove(&task.id);
        }
        let backdrop = self.reminder_backdrop();
        if let Err(error) = reminder::show(
            cx.weak_entity(),
            self.theme,
            backdrop,
            self.reminder_snooze_minutes,
            self.pause_media_on_reminder,
            due,
            cx,
        ) {
            self.toast = Some(format!("Could not open the reminder: {error}"));
            self.reminder_open = false;
        } else {
            self.reminder_open = true;
        }
    }

    /// Complete a task from the full-screen reminder. It is an ordinary
    /// command, so the main window's Undo still reverses it.
    pub(super) fn complete_reminder_task(&mut self, id: &TaskId, cx: &mut Context<Self>) {
        self.execute(Command::CompleteTasks {
            ids: vec![id.clone()],
            completed: true,
        });
        self.sync_notifications(cx);
        cx.notify();
    }

    /// Bring a snoozed task back after the requested delay.
    pub(super) fn snooze_reminder_task(&mut self, id: &TaskId, minutes: i64) {
        self.reminder_seen
            .retain(|key| !key.starts_with(&format!("{id}:")));
        self.reminder_snoozed
            .insert(id.clone(), now() + chrono::Duration::minutes(minutes));
        self.save_session();
    }

    /// Remember the length chosen on the reminder itself so the next one opens
    /// with the same buttons.
    pub(super) fn set_reminder_snooze_minutes(&mut self, minutes: u32) {
        let minutes = match minutes {
            30 => 30,
            60 => 60,
            _ => 15,
        };
        if self.reminder_snooze_minutes == minutes {
            return;
        }
        self.reminder_snooze_minutes = minutes;
        self.save_session();
    }

    /// The reminder window went away, so a later due task can open it again.
    pub(super) fn reminder_closed(&mut self) {
        self.reminder_open = false;
    }

    /// Open the list of tasks that have an effective full-screen reminder.
    pub(super) fn open_fullscreen_view(&mut self, cx: &mut Context<Self>) {
        self.set_view(View::Builtin(BuiltinPerspective::Fullscreen));
        self.focus = FocusArea::Content;
        self.save_session();
        cx.activate(true);
        cx.notify();
    }

    /// Show the task in the main window and bring that window forward.
    pub(super) fn open_reminder_task(&mut self, id: &TaskId, cx: &mut Context<Self>) {
        self.reveal_task(id.clone(), cx);
        cx.activate(true);
    }

    fn save_session(&self) {
        let session = Session {
            view: Some(self.view.key()),
            selected: self.selected.iter().map(|id| id.to_string()).collect(),
            inspector: Some(self.show_inspector),
            theme: Some(self.theme.kind().slug().to_string()),
            appearance: Some(self.theme.appearance().slug().to_string()),
            plain_accent_light: self.plain_accent_light.map(theme::hex),
            plain_accent_dark: self.plain_accent_dark.map(theme::hex),
            window: self.window_size,
            sort: self.sort.map(|sort| sort_slug(sort).to_string()),
            group: self.group.map(|group| group_slug(group).to_string()),
            show_completed: Some(self.show_completed),
            keep_completed_subtasks: Some(self.keep_completed_subtasks),
            hide_empty_views: Some(self.hide_empty_views),
            vim_mode: Some(self.vim_mode),
            notifications: Some(self.due_notifications),
            calendar_events: Some(self.calendar_events_enabled),
            calendar_hidden: self.calendar_hidden.clone(),
            forecast_due_only: Some(self.forecast_due_only),
            settings_size: self.settings_size,
            perspective_size: self.perspective_size,
            fullscreen_reminders: Some(self.fullscreen_reminders),
            pause_media_on_reminder: Some(self.pause_media_on_reminder),
            reminder_background: Some(self.reminder_background.slug().to_string()),
            reminder_match_theme: Some(self.reminder_match_theme),
            reminder_wallpaper: None,
            reminder_wallpaper_light: self
                .reminder_wallpaper_light
                .as_ref()
                .map(|path| path.display().to_string()),
            reminder_wallpaper_dark: self
                .reminder_wallpaper_dark
                .as_ref()
                .map(|path| path.display().to_string()),
            reminder_wallpaper_separate: Some(self.reminder_wallpaper_separate),
            reminder_use_wallpaper: Some(self.reminder_use_wallpaper),
            reminder_wallpaper_opacity: Some(self.reminder_wallpaper_opacity),
            fullscreen_snooze_minutes: Some(self.reminder_snooze_minutes),
            wallpaper: None,
            wallpaper_light: self
                .wallpaper_light
                .as_ref()
                .map(|path| path.display().to_string()),
            wallpaper_dark: self
                .wallpaper_dark
                .as_ref()
                .map(|path| path.display().to_string()),
            wallpaper_separate: Some(self.wallpaper_separate),
            use_wallpaper: Some(self.use_wallpaper),
            wallpaper_opacity: Some(self.wallpaper_opacity),
            app_icon: Some(self.app_icon.slug().to_string()),
            priority_filter: self.priority_filter.map(|p| p.slug().to_string()),
            only_available: Some(self.only_available),
            follow_system: Some(self.follow_system),
            traffic_lights: Some(self.traffic_lights_visible),
            forecast_calendar_open: Some(self.forecast_calendar_open),
            forecast_planned: Some(self.forecast_planned),
            forecast_calendar_expanded: Some(self.forecast_calendar_expanded),
            forecast_calendar_height: self.forecast_calendar_height,
            focus: None,
            focus_targets: self.focus_targets.iter().map(FocusTarget::key).collect(),
            last_focus_targets: self
                .last_focus_targets
                .iter()
                .map(FocusTarget::key)
                .collect(),
            focus_banner: Some(self.show_focus_banner),
            inspector_sections: self.inspector_sections.clone(),
            toolbar_labels: Some(self.show_toolbar_labels),
            view_options: {
                let mut map = self.view_options.clone();
                map.insert(self.view.key(), self.current_view_options());
                map
            },
        };
        let _ = self.store.write_session(&session);
    }

    /// The layout and filters to remember for the active view. A Priority
    /// view always takes its filter from the view itself, so it stores nothing
    /// here and cannot leak its priority into the next perspective.
    fn current_view_options(&self) -> session::ViewOptions {
        let priority = match self.view {
            View::Priority(_) => None,
            _ => self
                .priority_filter
                .map(|priority| priority.slug().to_string()),
        };
        session::ViewOptions {
            sort: self.sort.map(|sort| sort_slug(sort).to_string()),
            group: self.group.map(|group| group_slug(group).to_string()),
            show_completed: Some(self.show_completed),
            only_available: Some(self.only_available),
            priority,
            hide_past: Some(self.hide_past),
        }
    }

    /// Remember the current view's layout before switching away.
    fn remember_view_options(&mut self) {
        self.view_options
            .insert(self.view.key(), self.current_view_options());
    }

    /// Restore layout for the view that is now active. Views without a saved
    /// record start clean rather than inheriting the previous view's filters.
    fn restore_view_options(&mut self) {
        let options = self
            .view_options
            .get(&self.view.key())
            .cloned()
            .unwrap_or_default();
        self.sort = options.sort.as_deref().and_then(sort_from_slug);
        self.group = options.group.as_deref().and_then(group_from_slug);
        self.show_completed = options.show_completed.unwrap_or(false);
        self.only_available = options.only_available.unwrap_or(false);
        self.hide_past = options.hide_past.unwrap_or(false);
        self.priority_filter = effective_priority_filter(
            &self.view,
            options.priority.as_deref().and_then(Priority::from_slug),
        );
    }

    fn prune_selection(&mut self) {
        let db = self.store.db();
        self.selected.retain(|id| db.tasks.contains_key(id));
        if let Some(anchor) = &self.anchor {
            if !db.tasks.contains_key(anchor) {
                self.anchor = self.selected.first().cloned();
            }
        }
    }

    fn prune_focus(&mut self) {
        let db = self.store.db();
        self.focus_targets.retain(|target| match target {
            FocusTarget::Project(id) => db.projects.contains_key(id),
            FocusTarget::Folder(id) => db.folders.contains_key(id),
        });
        self.sidebar_selection.retain(|target| match target {
            FocusTarget::Project(id) => db.projects.contains_key(id),
            FocusTarget::Folder(id) => db.folders.contains_key(id),
        });
        self.last_focus_targets.retain(|target| match target {
            FocusTarget::Project(id) => db.projects.contains_key(id),
            FocusTarget::Folder(id) => db.folders.contains_key(id),
        });
        if self.focus_targets.is_empty() {
            self.focus_banner_anchor.set(None);
        }
    }

    // --------------------------------------------------------------- focus

    fn focus_allows_project(&self, project: &ProjectId) -> bool {
        self.focus_targets.is_empty()
            || self
                .focus_targets
                .iter()
                .any(|target| target.allows_project(self.store.db(), project))
    }

    fn focus_allows_task(&self, task: &TaskId) -> bool {
        self.focus_targets.is_empty()
            || self
                .focus_targets
                .iter()
                .any(|target| target.allows_task(self.store.db(), task))
    }

    /// Open tasks carrying this tag (or a descendant) inside the focus.
    fn focus_tag_usage(&self, tag: &TagId) -> usize {
        let db = self.store.db();
        db.tasks_tagged(tag, true)
            .into_iter()
            .filter(|id| {
                db.tasks.get(id).is_some_and(|task| !task.completed)
                    && self.focus_allows_task(id)
            })
            .count()
    }

    fn focus_groups(&self, groups: Vec<Group>) -> Vec<Group> {
        if self.focus_targets.is_empty() {
            return groups;
        }
        groups
            .into_iter()
            .filter_map(|mut group| {
                let group_project = group
                    .key
                    .strip_prefix("project:")
                    .and_then(|id| self.store.db().resolve_project(id).ok());
                if let Some(project) = &group_project
                    && !self.focus_allows_project(project)
                {
                    return None;
                }
                group
                    .projects
                    .retain(|project| self.focus_allows_project(project));
                group.rows.retain(|row| self.focus_allows_task(&row.id));
                if group.rows.is_empty() && group.projects.is_empty() && group_project.is_none() {
                    None
                } else {
                    Some(group)
                }
            })
            .collect()
    }

    fn set_focus(&mut self, target: FocusTarget, cx: &mut Context<Self>) {
        self.set_focus_targets(vec![target], cx);
    }

    fn set_focus_targets(&mut self, targets: Vec<FocusTarget>, cx: &mut Context<Self>) {
        let mut targets: Vec<FocusTarget> = targets;
        let mut seen = BTreeSet::new();
        targets.retain(|target| seen.insert(target.key()));
        if targets.is_empty() {
            self.clear_focus(cx);
            return;
        }
        let (ancestors, rebuild_view) = {
            let db = self.store.db();
            let mut ancestors = Vec::new();
            for target in &targets {
                let folder = match target {
                    FocusTarget::Folder(id) => Some(id.clone()),
                    FocusTarget::Project(id) => db
                        .projects
                        .get(id)
                        .and_then(|project| project.folder.clone()),
                };
                let mut current = folder;
                while let Some(id) = current {
                    if !ancestors.contains(&id) {
                        ancestors.push(id.clone());
                    }
                    current = db.folders.get(&id).and_then(|folder| folder.parent.clone());
                }
            }
            let rebuild = matches!(self.view, View::Project(_) | View::Folder(_))
                && !targets
                    .iter()
                    .any(|target| target.allows_view(db, &self.view));
            (ancestors, rebuild)
        };
        for id in ancestors {
            let key = format!("folder:{id}");
            self.expanded.insert(key.clone());
            self.collapsed_groups.remove(&key);
        }
        self.focus_targets = targets;
        self.last_focus_targets = self.focus_targets.clone();
        self.sidebar_selection.clear();
        self.focus_banner_anchor.set(None);
        if !self.show_focus_banner {
            self.show_focus_banner_transient(cx);
        }
        let db = self.store.db();
        self.selected.retain(|id| {
            self.focus_targets
                .iter()
                .any(|target| target.allows_task(db, id))
        });
        self.anchor = self.selected.first().cloned();
        if rebuild_view {
            self.set_view(self.focus_targets[0].view());
        }
        self.save_session();
        cx.notify();
    }

    fn toggle_sidebar_selection(&mut self, target: FocusTarget, cx: &mut Context<Self>) {
        if let Some(index) = self
            .sidebar_selection
            .iter()
            .position(|candidate| candidate == &target)
        {
            self.sidebar_selection.remove(index);
        } else {
            self.sidebar_selection.push(target);
        }
        cx.notify();
    }

    fn clear_sidebar_selection(&mut self, cx: &mut Context<Self>) {
        if !self.sidebar_selection.is_empty() {
            self.sidebar_selection.clear();
            cx.notify();
        }
    }

    /// A sidebar focus command uses the whole Cmd-click selection when the
    /// clicked row is part of it; otherwise it focuses that row alone.
    fn focus_from_sidebar(&mut self, target: FocusTarget, cx: &mut Context<Self>) {
        if self.sidebar_selection.len() > 1 && self.sidebar_selection.contains(&target) {
            let targets = self.sidebar_selection.clone();
            self.set_focus_targets(targets, cx);
        } else {
            self.set_focus_targets(vec![target], cx);
        }
    }

    fn show_focus_banner_transient(&mut self, cx: &mut Context<Self>) {
        self.focus_banner_until = Some(Instant::now() + Duration::from_secs(3));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            this.update(cx, |this, cx| {
                if this
                    .focus_banner_until
                    .is_some_and(|until| Instant::now() >= until)
                {
                    this.focus_banner_until = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn toggle_focus_eye(&mut self, cx: &mut Context<Self>) {
        if !self.focus_targets.is_empty() {
            self.clear_focus(cx);
            return;
        }
        if self.last_focus_targets.is_empty() {
            return;
        }
        let targets = self.last_focus_targets.clone();
        self.set_focus_targets(targets, cx);
    }

    fn clear_focus(&mut self, cx: &mut Context<Self>) {
        if !self.focus_targets.is_empty() {
            self.focus_targets.clear();
            self.sidebar_selection.clear();
            self.focus_banner_anchor.set(None);
            self.focus_banner_until = None;
            self.save_session();
            cx.notify();
        }
    }

    // -------------------------------------------------------------- views

    /// Add event-only Forecast groups for the coming week, so a day with
    /// calendar events still appears even when it has no tasks.
    fn add_forecast_calendar_groups(
        &self,
        groups: &mut Vec<Group>,
        at: chrono::DateTime<chrono::Utc>,
    ) {
        if !self.calendar_events_enabled {
            return;
        }
        let today = local_date(at);
        let mut existing: BTreeSet<String> = groups.iter().map(|group| group.key.clone()).collect();
        for offset in 0..=7 {
            let date = today + chrono::Duration::days(offset);
            if self.calendar_events_on(date).is_empty() {
                continue;
            }
            let (key, title, position) = match offset {
                0 => ("forecast:today".to_string(), "Today".to_string(), -2),
                1 => ("forecast:tomorrow".to_string(), "Tomorrow".to_string(), 0),
                _ => (
                    format!("forecast:{}", date.format("%Y-%m-%d")),
                    query::forecast::day_label(date, today),
                    offset,
                ),
            };
            if existing.insert(key.clone()) {
                groups.push(Group {
                    projects: Vec::new(),
                    key,
                    title,
                    position,
                    rows: Vec::new(),
                });
            }
        }
        groups.sort_by(|left, right| left.position.cmp(&right.position));
    }

    fn current_groups(&self) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let mut groups = match &self.view {
            // Review and Archive have their own surfaces; Nearby its own
            // ranked list. The rest still need ids here so j/k, x and undo
            // work on them.
            View::Builtin(BuiltinPerspective::Review | BuiltinPerspective::Archive) => Vec::new(),
            View::Builtin(BuiltinPerspective::Forecast)
                if self.forecast_calendar_open && self.forecast_past =>
            {
                let show_planned = self.forecast_planned && !self.forecast_due_only;
                let ids = query::forecast_past(db, at, show_planned, self.forecast_due_only);
                let projects =
                    query::forecast_past_projects(db, at, show_planned, self.forecast_due_only);
                vec![Group {
                    projects,
                    key: "forecast-focus:past".to_string(),
                    title: "Past".to_string(),
                    position: 0,
                    rows: query::rows_for(db, &ids, at),
                }]
            }
            View::Builtin(BuiltinPerspective::Forecast)
                if self.forecast_calendar_open && self.forecast_future =>
            {
                let after = self.forecast_week + chrono::Duration::days(6);
                let show_planned = self.forecast_planned && !self.forecast_due_only;
                let ids =
                    query::forecast_future(db, at, after, show_planned, self.forecast_due_only);
                let projects = query::forecast_future_projects(
                    db,
                    after,
                    show_planned,
                    self.forecast_due_only,
                );
                vec![Group {
                    projects,
                    key: "forecast-focus:future".to_string(),
                    title: "Future".to_string(),
                    position: 0,
                    rows: query::rows_for(db, &ids, at),
                }]
            }
            View::Builtin(BuiltinPerspective::Forecast)
                if self.forecast_calendar_open && self.forecast_date.is_some() =>
            {
                let date = self.forecast_date.expect("guard checked");
                let show_planned = self.forecast_planned && !self.forecast_due_only;
                let ids = query::tasks_on_date(db, date, at, show_planned, self.forecast_due_only);
                let projects =
                    query::projects_on_date(db, date, show_planned, self.forecast_due_only);
                let title = query::forecast::day_label(date, local_date(at));
                vec![Group {
                    projects,
                    key: format!("forecast-day:{date}"),
                    title,
                    position: 0,
                    rows: query::rows_for(db, &ids, at),
                }]
            }
            View::Builtin(BuiltinPerspective::Today) => query::today(db, !self.hide_past, at),
            View::Builtin(BuiltinPerspective::Forecast) => {
                let mut groups = query::forecast_view(
                    db,
                    at,
                    self.forecast_planned && !self.forecast_due_only,
                    self.forecast_due_only,
                );
                self.add_forecast_calendar_groups(&mut groups, at);
                groups
            }
            View::Builtin(perspective) => query::view(db, *perspective, at),
            View::Priority(priority) => query::priority(db, *priority, at),
            View::Project(id) => self.project_groups(id),
            View::Folder(id) => db
                .flatten_containers()
                .into_iter()
                .filter_map(|row| match row.container {
                    ContainerId::Project(project) => Some(project),
                    ContainerId::Folder(_) => None,
                })
                .filter(|project| project_in_folder(db, project, id))
                .flat_map(|project| self.project_groups(&project))
                .collect::<Vec<_>>(),
            View::Tag(id) => {
                let ids: Vec<TaskId> = db
                    .tasks_tagged(id, true)
                    .into_iter()
                    .filter(|task_id| {
                        db.tasks
                            .get(task_id)
                            .is_some_and(|task| self.show_completed || !task.completed)
                    })
                    .collect();
                let rows = query::rows_for(db, &ids, at);
                vec![Group {
                    projects: Vec::new(),
                    key: format!("tag:{id}"),
                    title: db.tag_path(id),
                    position: 0,
                    rows,
                }]
            }
            View::Custom(id) => match db.perspectives.get(id) {
                Some(perspective) => query::custom(db, perspective, at),
                None => Vec::new(),
            },
        };

        if let Some(sort) = self.sort {
            for group in &mut groups {
                query::sort::sort_rows(db, &mut group.rows, sort);
            }
        }

        if let Some(group_by) = self.group {
            let rows: Vec<TaskRow> = groups.into_iter().flat_map(|group| group.rows).collect();
            groups = query::regroup(db, rows, group_by, at);
        }

        if !self.search.is_empty() {
            groups = if self.search_library {
                self.library_groups()
            } else {
                self.filter_groups(groups)
            };
        }

        if let Some(priority) = self.priority_filter {
            groups = groups
                .into_iter()
                .filter_map(|mut group| {
                    group.rows.retain(|row| {
                        db.tasks
                            .get(&row.id)
                            .is_some_and(|task| task.priority == Some(priority))
                    });
                    (!group.rows.is_empty()).then_some(group)
                })
                .collect();
        }

        // First Available: keep only rows the user can start right now.
        if self.only_available {
            groups = groups
                .into_iter()
                .filter_map(|mut group| {
                    group.rows.retain(|row| row.availability.is_available());
                    (!group.rows.is_empty()).then_some(group)
                })
                .collect();
        }

        if !self.focus_targets.is_empty()
            && !matches!(
                self.view,
                View::Builtin(BuiltinPerspective::Inbox | BuiltinPerspective::Trash)
            )
        {
            groups = self.focus_groups(groups);
        }
        groups
    }

    fn project_groups(&self, id: &ProjectId) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let title = db
            .projects
            .get(id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project".to_string());
        let mut open_rows = Vec::new();
        let mut completed_rows = Vec::new();
        let mut root_completed = false;
        for row in db.project_outline_with(id, self.show_completed, self.keep_completed_subtasks) {
            if row.depth == 0 {
                root_completed = db
                    .tasks
                    .get(&row.task)
                    .is_some_and(|task| task.completed);
            }
            let mut task_row = query::row_for(db, &row.task, at);
            task_row.depth = row.depth;
            if self.show_completed && root_completed {
                completed_rows.push(task_row);
            } else {
                open_rows.push(task_row);
            }
        }
        let mut groups = vec![Group {
            projects: Vec::new(),
            key: format!("project:{id}"),
            title,
            position: 0,
            rows: open_rows,
        }];
        if self.show_completed && !completed_rows.is_empty() {
            groups.push(Group {
                projects: Vec::new(),
                key: format!("project-completed:{id}"),
                title: "Completed".to_string(),
                position: 1,
                rows: completed_rows,
            });
        }
        groups
    }

    fn filter_groups(&self, groups: Vec<Group>) -> Vec<Group> {
        let db = self.store.db();
        let needle = self.search.text().trim().to_lowercase();
        let mut out = Vec::new();
        for mut group in groups {
            // Fuzzy scoring subsumes plain substring matching ("lauch"
            // still finds "Launch"), and relevance puts the closest row
            // first without changing which group it belongs to.
            let mut scored: Vec<(i64, String, TaskRow)> = group
                .rows
                .into_iter()
                .filter_map(|row| {
                    let task = db.tasks.get(&row.id)?;
                    let score = flowlist_core::search::task_score_normalized(db, task, &needle);
                    (score > 0).then_some((score, task.title.to_lowercase(), row))
                })
                .collect();
            scored.sort_by(|(a_score, a_title, _), (b_score, b_title, _)| {
                b_score.cmp(a_score).then_with(|| a_title.cmp(b_title))
            });
            group.rows = scored.into_iter().map(|(_, _, row)| row).collect();
            if !group.rows.is_empty() {
                out.push(group);
            }
        }
        out
    }

    /// Matches from the whole library in one relevance-ordered list. Trash is
    /// never included; the Trash view's own search covers deleted tasks.
    fn library_groups(&self) -> Vec<Group> {
        let db = self.store.db();
        let needle = self.search.text().trim().to_lowercase();
        let at = now();
        let mut scored: Vec<(i64, String, TaskId)> = db
            .tasks
            .values()
            .filter(|task| !task.is_trashed())
            .filter_map(|task| {
                let score = flowlist_core::search::task_score_normalized(db, task, &needle);
                (score > 0).then_some((score, task.title.to_lowercase(), task.id.clone()))
            })
            .collect();
        scored.sort_by(|(a_score, a_title, _), (b_score, b_title, _)| {
            b_score.cmp(a_score).then_with(|| a_title.cmp(b_title))
        });
        let rows = scored
            .into_iter()
            .map(|(_, _, id)| query::row_for(db, &id, at))
            .collect();
        vec![Group {
            projects: Vec::new(),
            key: "search:library".to_string(),
            title: "Search results".to_string(),
            position: 0,
            rows,
        }]
    }

    fn visible_ids(&self) -> Vec<TaskId> {
        self.current_groups()
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.id))
            .collect()
    }

    fn open_view(&mut self, view: View, cx: &mut Context<Self>) {
        self.set_view(view);
        cx.notify();
    }

    fn open_quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.modal.is_some() {
            return;
        }
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.sidebar_menu = None;
        self.date_picker = None;
        self.quick_open = Some(QuickOpenState {
            query: text::TextBuffer::empty(),
            cursor: 0,
        });
        self.quick_open_scroll.set_offset(point(px(0.), px(0.)));
        self.focus = FocusArea::Content;
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn close_quick_open(&mut self, cx: &mut Context<Self>) {
        if self.quick_open.take().is_some() {
            cx.notify();
        }
    }

    fn quick_open_entries(&self) -> Vec<QuickOpenEntry> {
        let query = self
            .quick_open
            .as_ref()
            .map(|state| state.query.text().trim().to_lowercase())
            .unwrap_or_default();
        // The palette opens empty on purpose: listing every entity before the
        // user has typed reads as noise, not as a result set.
        if query.is_empty() {
            return Vec::new();
        }
        let db = self.store.db();
        let mut entries = Vec::new();
        for perspective in BuiltinPerspective::ALL {
            entries.push(QuickOpenEntry {
                label: perspective.label().to_string(),
                detail: "Perspective".to_string(),
                icon: Icon::Perspective,
                action: QuickOpenAction::Open(sidebar::SidebarTarget::Builtin(perspective)),
                rank: 0,
            });
        }
        for perspective in db.perspectives.values() {
            entries.push(QuickOpenEntry {
                label: perspective.name.clone(),
                detail: "Saved perspective".to_string(),
                icon: Icon::Perspective,
                action: QuickOpenAction::Open(sidebar::SidebarTarget::Custom(
                    perspective.id.clone(),
                )),
                rank: 1,
            });
        }
        if !self.focus_targets.is_empty() {
            entries.push(QuickOpenEntry {
                label: "Clear focus".to_string(),
                detail: "Focus".to_string(),
                icon: Icon::Close,
                action: QuickOpenAction::ClearFocus,
                rank: 0,
            });
        }
        for row in db.flatten_containers() {
            match row.container {
                ContainerId::Folder(id) => {
                    let path = db.folder_path(&id);
                    entries.push(QuickOpenEntry {
                        label: path.clone(),
                        detail: "Folder".to_string(),
                        icon: Icon::Folder,
                        action: QuickOpenAction::Open(sidebar::SidebarTarget::Folder(id.clone())),
                        rank: 2,
                    });
                    entries.push(QuickOpenEntry {
                        label: format!("Focus on {path}"),
                        detail: "Focus".to_string(),
                        icon: Icon::Folder,
                        action: QuickOpenAction::Focus(FocusTarget::Folder(id)),
                        rank: 6,
                    });
                }
                ContainerId::Project(id) => {
                    let path = db.project_path(&id);
                    entries.push(QuickOpenEntry {
                        label: path.clone(),
                        detail: "Project".to_string(),
                        icon: Icon::Project,
                        action: QuickOpenAction::Open(sidebar::SidebarTarget::Project(id.clone())),
                        rank: 3,
                    });
                    entries.push(QuickOpenEntry {
                        label: format!("Focus on {path}"),
                        detail: "Focus".to_string(),
                        icon: Icon::Project,
                        action: QuickOpenAction::Focus(FocusTarget::Project(id)),
                        rank: 6,
                    });
                }
            }
        }
        for row in db.flatten_tags() {
            entries.push(QuickOpenEntry {
                label: db.tag_path(&row.tag),
                detail: "Tag".to_string(),
                icon: Icon::Tag,
                action: QuickOpenAction::Open(sidebar::SidebarTarget::Tag(row.tag)),
                rank: 4,
            });
        }
        for priority in Priority::ALL {
            entries.push(QuickOpenEntry {
                label: format!("{} priority", priority.label()),
                detail: "Priority".to_string(),
                icon: Icon::Flagged,
                action: QuickOpenAction::Open(sidebar::SidebarTarget::Priority(priority)),
                rank: 5,
            });
        }

        let words: Vec<&str> = query.split_whitespace().collect();
        entries.retain(|entry| {
            let haystack = format!("{} {}", entry.label, entry.detail).to_lowercase();
            words.iter().all(|word| haystack.contains(word))
        });
        entries.sort_by(|left, right| {
            let left_lower = left.label.to_lowercase();
            let right_lower = right.label.to_lowercase();
            (
                left.rank,
                !left_lower.starts_with(&query),
                left_lower,
                left.detail.clone(),
            )
                .cmp(&(
                    right.rank,
                    !right_lower.starts_with(&query),
                    right_lower,
                    right.detail.clone(),
                ))
        });
        entries.truncate(12);
        entries
    }

    fn choose_quick_open(&mut self, cx: &mut Context<Self>) {
        let entries = self.quick_open_entries();
        let cursor = self
            .quick_open
            .as_ref()
            .map(|state| state.cursor)
            .unwrap_or(0);
        let Some(entry) = entries.get(cursor) else {
            self.quick_open = None;
            cx.notify();
            return;
        };
        let action = entry.action.clone();
        self.quick_open = None;
        self.focus = FocusArea::Content;
        match action {
            QuickOpenAction::Open(target) => self.sidebar_activate(target, cx),
            QuickOpenAction::Focus(target) => self.set_focus(target, cx),
            QuickOpenAction::ClearFocus => self.clear_focus(cx),
        }
    }

    fn handle_quick_open_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        let mods = stroke.modifiers;
        if mods.platform && stroke.key == "o" {
            self.close_quick_open(cx);
            return;
        }
        match stroke.key.as_str() {
            "escape" => {
                self.close_quick_open(cx);
                return;
            }
            "up" => {
                if let Some(state) = &mut self.quick_open {
                    state.cursor = state.cursor.saturating_sub(1);
                }
                self.reveal_quick_open_cursor();
                cx.notify();
                return;
            }
            "down" => {
                let count = self.quick_open_entries().len();
                if let Some(state) = &mut self.quick_open
                    && count > 0
                {
                    state.cursor = (state.cursor + 1).min(count - 1);
                }
                self.reveal_quick_open_cursor();
                cx.notify();
                return;
            }
            "enter" | "return" => {
                self.choose_quick_open(cx);
                return;
            }
            _ => {}
        }
        let paste = if mods.platform && stroke.key == "v" {
            cx.read_from_clipboard().and_then(|item| item.text())
        } else {
            None
        };
        if mods.platform && stroke.key == "a" {
            if let Some(state) = &mut self.quick_open {
                state.query.select_all();
            }
            cx.notify();
            return;
        }
        if mods.platform && (stroke.key == "c" || stroke.key == "x") {
            if let Some(state) = &mut self.quick_open {
                let text = state.query.selected_text();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                if stroke.key == "x" {
                    state.query.delete_selection();
                }
            }
            cx.notify();
            return;
        }
        if let Some(state) = &mut self.quick_open
            && let BufferKey::Changed =
                handle_buffer_key(&mut state.query, stroke, paste.as_deref())
        {
            state.cursor = 0;
            self.reveal_quick_open_cursor();
        }
        cx.notify();
    }

    /// Scroll the palette just enough to show the highlighted row. Called from
    /// key handling rather than render, so a mouse-wheel scroll is not undone
    /// on the next frame.
    fn reveal_quick_open_cursor(&self) {
        if let Some(state) = &self.quick_open
            && !self.quick_open_entries().is_empty()
        {
            self.quick_open_scroll.scroll_to_item(state.cursor);
        }
    }

    fn render_quick_open(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(state) = &self.quick_open else {
            return div().into_any_element();
        };
        let entries = self.quick_open_entries();
        let cursor = state.cursor.min(entries.len().saturating_sub(1));
        let query = state.query.clone();
        let typed = !state.query.text().trim().is_empty();
        // The palette hangs 110px from the top and never grows past the window,
        // so a short display scrolls its results instead of bleeding them over
        // the background.
        let viewport_height = f32::from(window.viewport_size().height);
        let panel_max_height = (viewport_height - 134.0).clamp(220.0, 420.0);
        let list_max_height = (panel_max_height - 96.0).max(96.0);
        let mut list = div()
            .id("quick-open-scroll")
            .flex()
            .flex_col()
            .gap_1()
            .max_h(px(list_max_height))
            .overflow_y_scroll()
            .track_scroll(&self.quick_open_scroll);
        // Keep the panel quiet until the first keystroke.
        if typed && entries.is_empty() {
            list = list.child(
                div()
                    .px_2()
                    .py_3()
                    .text_sm()
                    .text_color(theme.text_faint)
                    .child("No matches"),
            );
        } else if typed {
            for (index, entry) in entries.iter().enumerate() {
                let selected = index == cursor;
                let action = entry.action.clone();
                list = list.child(
                    div()
                        .id(sid(format!("quick-open:{}", entry.action.key())))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(if selected {
                            theme.selection
                        } else {
                            theme.clear
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .child(icons::icon(
                            entry.icon,
                            14.0,
                            if selected {
                                theme.accent
                            } else {
                                theme.text_faint
                            },
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(if selected {
                                            theme.accent
                                        } else {
                                            theme.text
                                        })
                                        .child(entry.label.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_faint)
                                        .child(entry.detail.clone()),
                                ),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.quick_open = None;
                            this.focus = FocusArea::Content;
                            match action.clone() {
                                QuickOpenAction::Open(target) => this.sidebar_activate(target, cx),
                                QuickOpenAction::Focus(target) => this.set_focus(target, cx),
                                QuickOpenAction::ClearFocus => this.clear_focus(cx),
                            }
                        })),
                );
            }
        }
        let panel = div()
            .w(px(560.))
            .max_h(px(panel_max_height))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .px_1()
                    .child(widgets::text_field(theme, &query, "Jump to…", true)),
            )
            .child(list)
            .child(
                div()
                    .px_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("↑↓ navigate · ↵ open · esc close"),
            );
        div()
            .id("quick-open-layer")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(theme.bg.opacity(0.42))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.quick_open = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .top(px(110.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(panel),
            )
            .into_any_element()
    }

    /// Switch views without notifying. Callers that already repaint use this
    /// from inside a command; `open_view` is the interactive entry point.
    fn set_view(&mut self, view: View) {
        if !self.focus_targets.is_empty()
            && !self
                .focus_targets
                .iter()
                .any(|target| target.allows_view(self.store.db(), &view))
        {
            self.focus_targets.clear();
            self.sidebar_selection.clear();
            self.focus_banner_anchor.set(None);
        }
        let key = view.key();
        if self.view.key() != key {
            // A new branch drops the forward trail, the same as a browser.
            self.view_history
                .truncate(self.view_history_index.saturating_add(1));
            if self.view_history.last() != Some(&key) {
                self.view_history.push(key);
                const LIMIT: usize = 50;
                if self.view_history.len() > LIMIT {
                    self.view_history.remove(0);
                }
            }
            self.view_history_index = self.view_history.len().saturating_sub(1);
        }
        self.apply_view(view);
    }

    /// Paint a view without touching the back/forward trail. History
    /// navigation uses this so stepping back does not push a new entry.
    fn apply_view(&mut self, view: View) {
        self.remember_view_options();
        self.view = view;
        self.restore_view_options();
        self.selected.clear();
        self.anchor = None;
        self.search.clear();
        self.editing = None;
        self.field_error = None;
        self.draft_task = None;
        self.focus = FocusArea::Content;
        self.save_session();
    }

    fn can_go_back(&self) -> bool {
        let db = self.store.db();
        self.view_history[..self.view_history_index]
            .iter()
            .rev()
            .any(|key| View::from_key(db, key).is_some())
    }

    fn can_go_forward(&self) -> bool {
        let db = self.store.db();
        self.view_history
            .iter()
            .skip(self.view_history_index + 1)
            .any(|key| View::from_key(db, key).is_some())
    }

    /// Step through views the window already visited. Entries whose project,
    /// folder or tag disappeared are skipped, so Undo and Delete never strand
    /// the arrows on a dead view.
    fn step_view_history(&mut self, delta: isize, cx: &mut Context<Self>) {
        let mut index = self.view_history_index as isize;
        loop {
            index += delta;
            if index < 0 || index as usize >= self.view_history.len() {
                return;
            }
            let key = self.view_history[index as usize].clone();
            if let Some(view) = View::from_key(self.store.db(), &key) {
                self.view_history_index = index as usize;
                self.apply_view(view);
                cx.notify();
                return;
            }
        }
    }

    fn view_title(&self) -> String {
        self.view.title(self.store.db())
    }

    fn view_subtitle(&self) -> String {
        if let View::Custom(id) = &self.view {
            if let Some(perspective) = self.store.db().perspectives.get(id) {
                let description = perspective.description.trim();
                if !description.is_empty() {
                    return description.to_string();
                }
            }
        }
        self.view.subtitle().to_string()
    }

    // ----------------------------------------------------------- selection

    fn selected_one(&self) -> Option<TaskId> {
        self.selected.first().cloned()
    }

    fn select_only(&mut self, id: TaskId) {
        if self.draft_task.as_ref() != Some(&id) {
            self.draft_task = None;
        }
        self.date_error = None;
        self.selected = vec![id.clone()];
        self.anchor = Some(id);
    }

    fn toggle_selection(&mut self, id: TaskId) {
        if let Some(index) = self.selected.iter().position(|candidate| candidate == &id) {
            self.selected.remove(index);
        } else {
            self.selected.push(id.clone());
            self.anchor = Some(id);
        }
    }

    fn select_range_to(&mut self, id: TaskId) {
        let ids = self.visible_ids();
        let Some(anchor) = self.anchor.clone() else {
            self.select_only(id);
            return;
        };
        let Some(start) = ids.iter().position(|candidate| candidate == &anchor) else {
            self.select_only(id);
            return;
        };
        let Some(end) = ids.iter().position(|candidate| candidate == &id) else {
            self.select_only(id);
            return;
        };
        let (low, high) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.selected = ids[low..=high].to_vec();
        self.save_session();
    }

    fn move_selection(&mut self, delta: isize, extend: bool) {
        let ids = self.visible_ids();
        if ids.is_empty() {
            self.selected.clear();
            return;
        }
        let current = self
            .selected
            .first()
            .and_then(|id| ids.iter().position(|candidate| candidate == id));
        let next = match current {
            Some(index) => {
                let target = index as isize + delta;
                target.clamp(0, ids.len() as isize - 1) as usize
            }
            None => {
                if delta < 0 {
                    ids.len() - 1
                } else {
                    0
                }
            }
        };
        if extend {
            let anchor = self.anchor.clone().unwrap_or_else(|| ids[next].clone());
            self.anchor = Some(anchor);
            self.selected = vec![ids[next].clone()];
            self.select_range_to(ids[next].clone());
        } else {
            self.select_only(ids[next].clone());
        }
        self.save_session();
    }

    // ------------------------------------------------------------ commands

    fn execute(&mut self, command: Command) -> Option<Outcome> {
        match self.store.execute(command) {
            Ok(outcome) => {
                self.command_error = None;
                self.toast = outcome
                    .message
                    .clone()
                    .or_else(|| Some(outcome.label.clone()));
                self.prune_selection();
                self.save_session();
                Some(outcome)
            }
            Err(error) => {
                let message = error.to_string();
                let message = message
                    .strip_prefix("Invalid operation: ")
                    .unwrap_or(&message)
                    .to_string();
                self.command_error = Some(message.clone());
                // A form clears this when it shows the message inline. Lists
                // and drag targets have no inline spot, so the toast remains
                // their fallback.
                self.toast = Some(message);
                None
            }
        }
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        match self.store.undo() {
            Ok(outcome) => {
                self.toast = Some(outcome.label);
                self.prune_selection();
                self.save_session();
            }
            Err(error) => self.toast = Some(error.to_string()),
        }
        cx.notify();
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        match self.store.redo() {
            Ok(outcome) => {
                self.toast = Some(outcome.label);
                self.prune_selection();
                self.save_session();
            }
            Err(error) => self.toast = Some(error.to_string()),
        }
        cx.notify();
    }

    fn complete_selected(&mut self, completed: bool, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let ids = self.selected.clone();
        self.execute(Command::CompleteTasks { ids, completed });
        if completed {
            self.selected.clear();
            self.anchor = None;
            self.editing = None;
            self.field_error = None;
            self.focus = FocusArea::Content;
        }
        self.save_session();
        cx.notify();
    }

    fn toggle_complete(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let completed = !self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.completed);
        self.execute(Command::CompleteTasks {
            ids: vec![id],
            completed,
        });
        if completed {
            self.selected.clear();
            self.anchor = None;
            self.editing = None;
            self.field_error = None;
            self.focus = FocusArea::Content;
        }
        self.save_session();
        cx.notify();
    }

    fn flag_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let db = self.store.db();
        let flag = !self
            .selected
            .iter()
            .all(|id| db.tasks.get(id).is_some_and(|task| task.flagged));
        self.execute(Command::SetTaskFlag {
            ids: self.selected.clone(),
            flagged: flag,
        });
        cx.notify();
    }

    fn toggle_flag(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let flagged = !self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.flagged);
        self.execute(Command::SetTaskFlag {
            ids: vec![id],
            flagged,
        });
        cx.notify();
    }

    fn set_task_step_mode(&mut self, id: TaskId, mode: TaskStepMode, cx: &mut Context<Self>) {
        let db = self.store.db();
        let Some(task) = db.tasks.get(&id) else {
            return;
        };
        let current = if task.steps_one_off {
            TaskStepMode::OneOff
        } else if task.sequential {
            TaskStepMode::Sequential
        } else {
            TaskStepMode::Parallel
        };
        if current == mode {
            return;
        }
        let children = task
            .child_order
            .iter()
            .filter(|child| db.tasks.get(child).is_some_and(|task| !task.is_trashed()))
            .count();
        self.inspector_sections
            .insert("task:subtasks".to_string(), false);
        // Converting to one-off steps is destructive: each child becomes a
        // checklist row with only its title and completion left. Ask first.
        if mode == TaskStepMode::OneOff && children > 0 {
            let modal = modals::Modal::hide_subtasks(self, id);
            self.open_modal(modal, cx);
            return;
        }
        self.apply_task_step_mode(id, mode, cx);
    }

    fn apply_task_step_mode(&mut self, id: TaskId, mode: TaskStepMode, cx: &mut Context<Self>) {
        self.inspector_sections
            .insert("task:subtasks".to_string(), false);
        self.execute(Command::SetTaskStepMode {
            id,
            sequential: mode == TaskStepMode::Sequential,
            steps_one_off: mode == TaskStepMode::OneOff,
        });
        cx.notify();
    }

    fn set_task_sequential(&mut self, sequential: bool, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            return;
        };
        self.execute(Command::UpdateTask {
            id,
            patch: TaskPatch {
                sequential: Some(sequential),
                ..Default::default()
            },
        });
        cx.notify();
    }

    /// Clicking the × on an attachment row asks first: removing a file is
    /// easy to mistake for opening it, and an accidental delete is not.
    fn remove_attachment(&mut self, path: String, cx: &mut Context<Self>) {
        let task = self.selected_one();
        let project = if task.is_none() {
            match &self.view {
                View::Project(id) => Some(id.clone()),
                _ => None,
            }
        } else {
            None
        };
        let target = match (task, project) {
            (Some(id), _) => modals::AttachmentTarget::Task(id),
            (None, Some(id)) => modals::AttachmentTarget::Project(id),
            _ => {
                self.toast = Some("Select a task or project first".to_string());
                cx.notify();
                return;
            }
        };
        self.open_modal(modals::Modal::remove_attachment(target, path), cx);
    }

    fn open_attachment(&mut self, path: String) {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let target = flowlist_core::model::attachment::attachment_url(&path).to_string();
        match std::process::Command::new(opener).arg(&target).spawn() {
            Ok(_) => self.toast = Some(format!("Opening {path}")),
            Err(error) => self.toast = Some(format!("Could not open {path}: {error}")),
        }
    }

    /// Ask macOS for real files (multi-select) and attach them to the selected
    /// task, or to the project when the project inspector is showing.
    fn pick_attachments(&mut self, cx: &mut Context<Self>) {
        let task = self.selected_one();
        let project = if task.is_none() {
            match &self.view {
                View::Project(id) => Some(id.clone()),
                _ => None,
            }
        } else {
            None
        };
        if task.is_none() && project.is_none() {
            self.toast = Some("Select a task or project first".to_string());
            cx.notify();
            return;
        }

        let picked = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let paths: Vec<String> = paths
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect();
            if paths.is_empty() {
                return;
            }
            this.update(cx, |this, cx| {
                this.add_attachments(task, project, paths, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Open the small link sheet for the selected task, or for the project
    /// whose inspector is showing. The sheet's single field takes an http(s)
    /// address and Enter attaches it.
    fn attach_link(&mut self, cx: &mut Context<Self>) {
        let task = self.selected_one();
        let project = if task.is_none() {
            match &self.view {
                View::Project(id) => Some(id.clone()),
                _ => None,
            }
        } else {
            None
        };
        let target = match (task, project) {
            (Some(id), _) => modals::AttachmentTarget::Task(id),
            (None, Some(id)) => modals::AttachmentTarget::Project(id),
            _ => {
                self.toast = Some("Select a task or project first".to_string());
                cx.notify();
                return;
            }
        };
        self.open_modal(modals::Modal::attach_link(target), cx);
    }

    fn add_attachments(
        &mut self,
        task: Option<TaskId>,
        project: Option<ProjectId>,
        paths: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(task) = task {
            let mut attachments = self
                .store
                .db()
                .tasks
                .get(&task)
                .map(|task| task.attachments.clone())
                .unwrap_or_default();
            for path in paths {
                if !attachments.contains(&path) {
                    attachments.push(path);
                }
            }
            self.execute(Command::UpdateTask {
                id: task,
                patch: TaskPatch {
                    attachments: Some(attachments),
                    ..Default::default()
                },
            });
        } else if let Some(project) = project {
            let mut attachments = self
                .store
                .db()
                .projects
                .get(&project)
                .map(|project| project.attachments.clone())
                .unwrap_or_default();
            for path in paths {
                if !attachments.contains(&path) {
                    attachments.push(path);
                }
            }
            self.execute(Command::UpdateProject {
                id: project,
                patch: ProjectPatch {
                    attachments: Some(attachments),
                    ..Default::default()
                },
            });
        }
        cx.notify();
    }

    /// One undoable step: the task disappears and a project with its
    /// subtasks appears in the same folder.
    fn convert_selected_to_project(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let folder = match &self.view {
            View::Folder(id) => Some(id.clone()),
            View::Project(project) => self
                .store
                .db()
                .projects
                .get(project)
                .and_then(|project| project.folder.clone()),
            _ => None,
        };
        if let Some(outcome) = self.execute(Command::ConvertTaskToProject { id, folder }) {
            if let Some(created) = outcome.created.first() {
                if let Ok(project) = self.store.db().resolve_project(created) {
                    self.set_view(View::Project(project));
                }
            }
        }
        cx.notify();
    }

    fn set_task_priority(
        &mut self,
        id: TaskId,
        priority: Option<Priority>,
        series: bool,
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::SetTaskPriority {
            ids: vec![id],
            priority,
            series,
        });
        cx.notify();
    }

    /// A repeating task asks whether the new priority belongs to this
    /// occurrence or the whole series. Ordinary tasks apply immediately.
    fn choose_task_priority(
        &mut self,
        id: TaskId,
        priority: Option<Priority>,
        cx: &mut Context<Self>,
    ) {
        let repeats = self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.repeat.is_some());
        if repeats {
            self.modal = Some(modals::Modal::priority_scope(self, id, priority));
            cx.notify();
        } else {
            self.set_task_priority(id, priority, false, cx);
        }
    }

    fn select_all_trash(&mut self, cx: &mut Context<Self>) {
        let ids = self.store.db().trashed_roots();
        if ids.is_empty() {
            self.toast = Some("Trash is already empty".to_string());
            cx.notify();
            return;
        }
        self.selected = ids;
        self.anchor = self.selected.first().cloned();
        self.focus = FocusArea::Content;
        self.save_session();
        cx.notify();
    }

    fn restore_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a trashed task first".to_string());
            cx.notify();
            return;
        }
        self.execute(Command::RestoreTasks {
            ids: self.selected.clone(),
        });
        self.selected.clear();
        self.anchor = None;
        cx.notify();
    }

    fn purge_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a trashed task first".to_string());
            cx.notify();
            return;
        }
        self.execute(Command::PurgeTasks {
            ids: self.selected.clone(),
        });
        self.selected.clear();
        self.anchor = None;
        self.save_session();
        cx.notify();
    }

    fn empty_trash(&mut self, cx: &mut Context<Self>) {
        let ids = self.store.db().trashed_roots();
        if ids.is_empty() {
            self.toast = Some("Trash is already empty".to_string());
            cx.notify();
            return;
        }
        self.execute(Command::PurgeTasks { ids });
        self.selected.clear();
        self.anchor = None;
        self.save_session();
        cx.notify();
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let ids = self.selected.clone();
        let count = ids.len();
        // Deleting from Trash is permanent; everywhere else the task moves to
        // Trash first.
        let in_trash = self.view == View::Builtin(BuiltinPerspective::Trash);
        // A repeating task has more than one occurrence, so ask before the
        // whole chain disappears. Trash keeps its existing permanent-delete
        // behaviour because the series was already broken there.
        let repeating_title = if in_trash {
            None
        } else {
            ids.iter().find_map(|id| {
                self.store
                    .db()
                    .tasks
                    .get(id)
                    .filter(|task| task.repeat.is_some())
                    .map(|task| task.title.clone())
            })
        };
        if let Some(title) = repeating_title {
            let modal = modals::Modal::delete_task(self, ids, title);
            self.open_modal(modal, cx);
            return;
        }
        let visible_before = self.visible_ids();
        let cursor = self
            .selected
            .first()
            .and_then(|id| visible_before.iter().position(|candidate| candidate == id))
            .unwrap_or(0);
        self.execute(if in_trash {
            Command::PurgeTasks { ids }
        } else {
            Command::DeleteTasks { ids }
        });
        self.selected.clear();
        self.anchor = None;
        // Keep the user where they were: select the task that took the
        // deleted row's place, or the last row if the tail was removed.
        let visible_after = self.visible_ids();
        if let Some(id) = visible_after
            .get(cursor.min(visible_after.len().saturating_sub(1)))
            .cloned()
        {
            self.select_only(id);
        }
        let verb = if in_trash {
            "Permanently deleted"
        } else {
            "Moved to Trash"
        };
        self.toast = Some(format!(
            "{verb} {} · ⌘Z to undo",
            plural(count, "task", "tasks")
        ));
        self.save_session();
        cx.notify();
    }

    /// Delete the entity under the sidebar cursor. Project rows have no
    /// delete affordance; they are closed from the project dashboard instead.
    fn delete_sidebar_target(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.sidebar_cursor.clone() else {
            self.toast = Some("Select a perspective, folder or tag first".to_string());
            cx.notify();
            return;
        };
        let (command, view) = match target {
            sidebar::SidebarTarget::Custom(id) => (
                Command::DeletePerspective { id: id.clone() },
                View::Custom(id),
            ),
            sidebar::SidebarTarget::Folder(id) => {
                let modal = modals::Modal::delete_folder(self, id);
                self.open_modal(modal, cx);
                return;
            }
            sidebar::SidebarTarget::Tag(id) => {
                let modal = modals::Modal::delete_tag(self, &id);
                self.open_modal(modal, cx);
                return;
            }
            sidebar::SidebarTarget::Project(_) => {
                self.toast = Some("Close a project from its dashboard".to_string());
                cx.notify();
                return;
            }
            sidebar::SidebarTarget::Builtin(_) | sidebar::SidebarTarget::Priority(_) => {
                self.toast = Some("This sidebar row cannot be deleted".to_string());
                cx.notify();
                return;
            }
        };
        self.execute(command);
        if self.view == view {
            self.view = View::Builtin(BuiltinPerspective::Inbox);
        }
        self.save_session();
        cx.notify();
    }

    fn indent_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(key) = db.order_key_for(&id) else {
            cx.notify();
            return;
        };
        let siblings = db.ordered_tasks(key).to_vec();
        let Some(index) = siblings.iter().position(|candidate| candidate == &id) else {
            cx.notify();
            return;
        };
        if index == 0 {
            self.toast = Some("Nothing above to nest under".to_string());
            cx.notify();
            return;
        }
        let parent = siblings[index - 1].clone();
        self.move_tasks_checked(
            vec![id],
            TaskPlacement::Parent {
                parent,
                position: Position::End,
            },
            cx,
        );
        cx.notify();
    }

    fn outdent_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(task) = db.tasks.get(&id) else {
            cx.notify();
            return;
        };
        let Some(parent_id) = task.parent.clone() else {
            self.toast = Some("Task is already at the top level".to_string());
            cx.notify();
            return;
        };
        let Some(parent) = db.tasks.get(&parent_id) else {
            cx.notify();
            return;
        };
        let placement = match &parent.parent {
            Some(grandparent) => TaskPlacement::Parent {
                parent: grandparent.clone(),
                position: Position::After(parent_id.clone()),
            },
            None => match &parent.project {
                Some(project) => TaskPlacement::Project {
                    project: project.clone(),
                    position: Position::After(parent_id.clone()),
                },
                None => TaskPlacement::Inbox(Position::After(parent_id.clone())),
            },
        };
        self.move_tasks_checked(vec![id], placement, cx);
    }

    fn move_selected(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(key) = db.order_key_for(&id) else {
            cx.notify();
            return;
        };
        let siblings = db.ordered_tasks(key.clone()).to_vec();
        let Some(index) = siblings.iter().position(|candidate| candidate == &id) else {
            cx.notify();
            return;
        };
        let position = if up {
            if index == 0 {
                cx.notify();
                return;
            }
            Position::Before(siblings[index - 1].clone())
        } else {
            if index + 1 >= siblings.len() {
                cx.notify();
                return;
            }
            Position::After(siblings[index + 1].clone())
        };
        let placement = match key {
            OrderKey::Inbox => TaskPlacement::Inbox(position),
            OrderKey::Project(project) => TaskPlacement::Project { project, position },
            OrderKey::Parent(parent) => TaskPlacement::Parent { parent, position },
        };
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement,
        });
        cx.notify();
    }

    // ------------------------------------------------------------- capture

    /// The system-wide capture shortcut can fire while another app is in
    /// front. Bring Flowlist forward, leave any perspective that has no
    /// capture bar, and put the caret in capture with the old text selected so
    /// typing starts a fresh task without losing what was there.
    /// Open the small floating capture panel. The in-window bar stays as the
    /// fallback when the platform cannot open a second window.
    pub(super) fn open_quick_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let theme = self.theme;
        // The in-app button belongs to this window, so the panel opens over
        // the window; the global shortcut keeps its screen-centered spot.
        let anchor = window.bounds();
        if let Err(error) = quick_capture::show(cx, self.store.paths().clone(), theme, Some(anchor))
        {
            self.toast = Some(format!("Could not open quick capture: {error}"));
            self.focus_capture(window, cx);
        }
    }

    /// The global chord toggles the panel: a second press puts it away.
    pub(super) fn toggle_quick_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let theme = self.theme;
        let anchor = window.bounds();
        if let Err(error) =
            quick_capture::toggle(cx, self.store.paths().clone(), theme, Some(anchor))
        {
            self.toast = Some(format!("Could not open quick capture: {error}"));
            self.focus_capture(window, cx);
        }
    }

    /// Select a just-captured task in the main window. `⌘⏎` in the capture
    /// panel calls this after the task has been filed.
    pub(super) fn reveal_task(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let project = self
            .store
            .db()
            .tasks
            .get(&id)
            .and_then(|task| task.project.clone());
        let view = project
            .map(View::Project)
            .unwrap_or(View::Builtin(BuiltinPerspective::Inbox));
        self.set_view(view);
        self.select_only(id);
        self.focus = FocusArea::Content;
        self.save_session();
        cx.notify();
    }

    fn focus_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.view.shows_capture() {
            self.open_view(View::Builtin(BuiltinPerspective::Inbox), cx);
        }
        self.editing = None;
        self.modal = None;
        self.checklist_editing = None;
        self.focus = FocusArea::Capture;
        self.capture.end(false);
        self.capture.select_all();
        // A Carbon hotkey fires while another app owns the menu bar, so
        // activate the app itself before asking the window to come forward.
        cx.activate(true);
        window.activate_window();
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn commit_capture(&mut self, cx: &mut Context<Self>) {
        let text = self.capture.take();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        }
        let at = now();
        match parse_quick(trimmed, at) {
            Ok(entry) => {
                let mut new_task = entry.into_new_task();
                self.apply_view_defaults(&mut new_task);
                self.execute(Command::CreateTask(new_task));
            }
            Err(error) => {
                self.toast = Some(format!("Could not understand that: {error}"));
            }
        }
        self.focus = FocusArea::Capture;
        cx.notify();
    }

    // ------------------------------------------------------------ inspector

    fn begin_checklist_edit(&mut self, item: ChecklistId, title: String, cx: &mut Context<Self>) {
        self.editing = None;
        self.checklist_editing = Some(item);
        self.checklist_buffer.set(title);
        self.focus = FocusArea::Checklist;
        cx.notify();
    }

    fn commit_checklist(&mut self, cx: &mut Context<Self>) {
        let Some(parent) = self.selected_one() else {
            self.checklist_editing = None;
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let title = self.checklist_buffer.text().trim().to_string();
        if title.is_empty() {
            self.focus = FocusArea::Content;
            self.checklist_editing = None;
            cx.notify();
            return;
        }
        if let Some(item) = self.checklist_editing.take() {
            self.execute(Command::UpdateChecklistItem {
                task: parent,
                item,
                title,
                completed: None,
            });
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
        } else if self
            .store
            .db()
            .tasks
            .get(&parent)
            .is_some_and(|task| task.steps_one_off)
        {
            self.execute(Command::AddChecklistItem {
                task: parent,
                title,
                position: None,
            });
            self.checklist_buffer.clear();
        } else {
            let mut new_task = NewTask::new(title);
            new_task.parent = Some(parent.clone());
            new_task.position = Position::End;
            self.execute(Command::CreateTask(new_task));
            self.expanded.insert(format!("task:{parent}"));
            self.checklist_buffer.clear();
            // Keep the field focused so several subtasks can be typed in a row.
            self.save_session();
        }
        cx.notify();
    }

    fn toggle_checklist_item(&mut self, task: TaskId, item: ChecklistId, cx: &mut Context<Self>) {
        let db = self.store.db();
        let Some(entity) = db
            .tasks
            .get(&task)
            .and_then(|task| task.checklist_item(&item))
        else {
            return;
        };
        let title = entity.title.clone();
        let completed = !entity.completed;
        self.execute(Command::UpdateChecklistItem {
            task,
            item,
            title,
            completed: Some(completed),
        });
        cx.notify();
    }

    fn move_checklist_item(
        &mut self,
        task: TaskId,
        item: ChecklistId,
        to: usize,
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::MoveChecklistItem { task, item, to });
        cx.notify();
    }

    fn delete_checklist_item(&mut self, task: TaskId, item: ChecklistId, cx: &mut Context<Self>) {
        if self.checklist_editing.as_ref() == Some(&item) {
            self.checklist_editing = None;
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
        }
        self.execute(Command::DeleteChecklistItem { task, item });
        cx.notify();
    }

    /// Every open project, as a `Folder / Project` path.
    fn project_paths(&self) -> Vec<String> {
        let db = self.store.db();
        db.flatten_containers()
            .into_iter()
            .filter_map(|row| match row.container {
                ContainerId::Project(id) => db
                    .projects
                    .get(&id)
                    .filter(|project| project.status.is_open())
                    .map(|_| db.project_path(&id)),
                ContainerId::Folder(_) => None,
            })
            .collect()
    }

    fn tag_paths(&self) -> Vec<String> {
        let db = self.store.db();
        db.flatten_tags()
            .into_iter()
            .map(|row| db.tag_path(&row.tag))
            .collect()
    }

    /// Prefix matches first, then substring matches. The query accepts either
    /// `Work / Launch` or the faster `Work/Launch`.
    fn suggest_paths(&self, corpus: Vec<String>, query: &str) -> Vec<String> {
        rank_suggestions(corpus, query)
    }

    fn project_suggestions(&self) -> Vec<String> {
        if self.editing != Some(EditField::Project) || self.focus != FocusArea::Field {
            return Vec::new();
        }
        let query = self.field.text().trim().to_string();
        // The dropdown stays closed until the user types. Listing every
        // project on open would take over the whole inspector once a library
        // has a few dozen projects.
        if query.is_empty() {
            return Vec::new();
        }
        self.suggest_paths(self.project_paths(), &query)
    }

    fn tag_suggestions(&self) -> Vec<String> {
        if self.focus != FocusArea::TagInput {
            return Vec::new();
        }
        self.suggest_paths(self.tag_paths(), self.tag_buffer.text())
    }

    fn choose_project_suggestion(&mut self, path: String, cx: &mut Context<Self>) {
        self.field.set(path);
        self.suggestion_index = 0;
        self.suggestion_engaged = false;
        self.commit_field(cx);
        // The suggestion was accepted, so this field session is over. A
        // first-time task now shows its inspector actions.
        self.draft_task = None;
    }

    fn choose_tag_suggestion(&mut self, path: String, cx: &mut Context<Self>) {
        self.tag_buffer.set(path);
        self.suggestion_index = 0;
        self.suggestion_engaged = false;
        self.add_tag_from_buffer(cx);
    }

    // ----------------------------------------------------------- date picker

    fn date_anchor(&self, field: DateField) -> Rc<Cell<Option<Bounds<gpui::Pixels>>>> {
        match field {
            DateField::Defer => self.defer_anchor.clone(),
            DateField::Planned => self.planned_anchor.clone(),
            DateField::Due => self.due_anchor.clone(),
            DateField::Reminder => self.reminder_anchor.clone(),
            DateField::RuleDate { .. } => self.due_anchor.clone(),
        }
    }

    /// The stamp currently stored in the selected task's date field.
    fn date_picker_value(&self, field: DateField) -> Option<Stamp> {
        let id = self.selected_one()?;
        let task = self.store.db().tasks.get(&id)?;
        match field {
            DateField::Defer => task.defer,
            DateField::Planned => task.planned,
            DateField::Due => task.due,
            DateField::Reminder => task.reminder_at,
            DateField::RuleDate { rule, second } => {
                let form = match &self.modal {
                    Some(modals::Modal::Perspective { form }) => form,
                    _ => return None,
                };
                let buffer =
                    form.rule(rule)
                        .map(|node| if second { &node.text2 } else { &node.text })?;
                flowlist_core::parse::parse_date(buffer.text().trim(), now())
                    .ok()
                    .map(Stamp::date_only)
            }
        }
    }

    /// Open the calendar under one field, starting on the stored date (or
    /// today) and closing every other overlay first.
    fn open_date_picker(&mut self, field: DateField, cx: &mut Context<Self>) {
        let stored = self.date_picker_value(field);
        let cursor = stored
            .map(|stamp| stamp.local_date())
            .unwrap_or_else(|| local_date(now()));
        self.date_picker = Some(field);
        self.date_picker_cursor = cursor;
        self.date_picker_month = first_of_month(cursor);
        self.date_picker_step = DatePickerStep::Calendar;
        self.date_picker_unit = ClockUnit::Hour;
        self.date_picker_boxes.borrow_mut().clear();
        // Keep a stored time, but start an all-day date all-day: the clock
        // controls are what turn it into a timed stamp.
        self.date_picker_time = stored
            .filter(|stamp| stamp.has_time)
            .map(|stamp| stamp.local_time());
        self.editing = None;
        self.field_error = None;
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.sidebar_menu = None;
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn close_date_picker(&mut self, cx: &mut Context<Self>) {
        if self.date_picker.take().is_some() {
            cx.notify();
        }
    }

    /// Flip the shown month, keeping the cursor on the same day where that
    /// day exists in the new month.
    fn shift_date_picker_month(&mut self, delta: i32) {
        self.date_picker_month = content::add_months(self.date_picker_month, delta);
        let (_, days, _) = content::month_grid(self.date_picker_month);
        let day = self.date_picker_cursor.day().min(days as u32).max(1);
        self.date_picker_cursor = self
            .date_picker_month
            .with_day(day)
            .unwrap_or(self.date_picker_month);
    }

    fn move_date_picker_cursor(&mut self, days: i64) {
        self.set_date_picker_cursor(self.date_picker_cursor + chrono::Duration::days(days));
    }

    fn set_date_picker_cursor(&mut self, date: chrono::NaiveDate) {
        self.date_picker_cursor = date;
        self.date_picker_month = first_of_month(date);
    }

    /// Pick a day from the calendar. All-day dates are written at once; a
    /// timed date opens the clock step, so the time is confirmed as part of
    /// the same choice instead of sitting cramped under the month.
    fn choose_date_picker_date(&mut self, date: chrono::NaiveDate, cx: &mut Context<Self>) {
        self.set_date_picker_cursor(date);
        if self.date_picker_time.is_some() {
            self.date_picker_step = DatePickerStep::Clock;
            self.date_picker_unit = ClockUnit::Hour;
            cx.notify();
        } else {
            self.apply_date_picker(date, cx);
        }
    }

    /// The time the clock step shows: the staged time, or the library default
    /// for this field when the date is still all-day.
    fn date_picker_clock_base(&self) -> Option<chrono::NaiveTime> {
        let field = self.date_picker?;
        let settings = &self.store.db().settings;
        Some(self.date_picker_time.unwrap_or_else(|| {
            picker_default_time(
                field,
                settings.default_due_time,
                settings.default_defer_time,
            )
        }))
    }

    /// Set the hour from the clock face. Choosing an hour moves the face on to
    /// minutes, like the Android time picker.
    fn set_date_picker_hour(&mut self, hour12: u32, cx: &mut Context<Self>) {
        let Some(base) = self.date_picker_clock_base() else {
            return;
        };
        let pm = base.hour() >= 12;
        if let Some(time) = base.with_hour(clock_hour_24(hour12, pm)) {
            self.date_picker_time = Some(time);
        }
        self.date_picker_unit = ClockUnit::Minute;
        cx.notify();
    }

    fn set_date_picker_minute(&mut self, minute: u32, cx: &mut Context<Self>) {
        let Some(base) = self.date_picker_clock_base() else {
            return;
        };
        if let Some(time) = base.with_minute(minute) {
            self.date_picker_time = Some(time);
        }
        cx.notify();
    }

    /// Switch AM/PM without changing the 12-hour reading.
    fn set_date_picker_half(&mut self, pm: bool, cx: &mut Context<Self>) {
        let Some(base) = self.date_picker_clock_base() else {
            return;
        };
        if let Some(time) = base.with_hour(clock_hour_24(clock_hour_12(base.hour()), pm)) {
            self.date_picker_time = Some(time);
        }
        cx.notify();
    }

    fn set_date_picker_unit(&mut self, unit: ClockUnit, cx: &mut Context<Self>) {
        self.date_picker_unit = unit;
        cx.notify();
    }

    /// Arrow keys on the clock face move one hour or one minute, depending on
    /// the active hand.
    fn step_date_picker_clock(&mut self, delta: i64, cx: &mut Context<Self>) {
        let Some(base) = self.date_picker_clock_base() else {
            return;
        };
        let minutes = match self.date_picker_unit {
            ClockUnit::Hour => delta.saturating_mul(60),
            ClockUnit::Minute => delta,
        };
        self.date_picker_time = Some(stepped_time(base, minutes));
        cx.notify();
    }

    fn date_picker_back_to_calendar(&mut self, cx: &mut Context<Self>) {
        if self.date_picker_step == DatePickerStep::Clock {
            self.date_picker_step = DatePickerStep::Calendar;
            cx.notify();
        }
    }

    /// The All day / At time choice. All-day drops the time and stays on the
    /// calendar; a time starts the clock face at the library default.
    fn set_date_picker_all_day(&mut self, all_day: bool, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
        if all_day {
            self.date_picker_time = None;
            self.date_picker_step = DatePickerStep::Calendar;
        } else {
            if self.date_picker_time.is_none() {
                let settings = &self.store.db().settings;
                self.date_picker_time = Some(picker_default_time(
                    field,
                    settings.default_due_time,
                    settings.default_defer_time,
                ));
            }
            self.date_picker_step = DatePickerStep::Clock;
            self.date_picker_unit = ClockUnit::Hour;
        }
        cx.notify();
    }

    /// Turn the staged time on (at the library default for this field) or off
    /// again, which makes the picked day all-day.
    fn toggle_date_picker_time(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
        self.date_picker_time = match self.date_picker_time {
            Some(_) => None,
            None => {
                let settings = &self.store.db().settings;
                Some(picker_default_time(
                    field,
                    settings.default_due_time,
                    settings.default_defer_time,
                ))
            }
        };
        cx.notify();
    }

    /// Step the staged time. An all-day date starts at its default, so the
    /// first press turns a time on instead of silently jumping to midnight.
    fn step_date_picker_time(&mut self, minutes: i64, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
        let base = self.date_picker_time.unwrap_or_else(|| {
            let settings = &self.store.db().settings;
            picker_default_time(
                field,
                settings.default_due_time,
                settings.default_defer_time,
            )
        });
        self.date_picker_time = Some(stepped_time(base, minutes));
        cx.notify();
    }

    /// Write the clicked day through the normal command layer, so the change
    /// is undoable. An existing time of day survives the date change.
    fn apply_date_picker(&mut self, date: chrono::NaiveDate, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
        let Some(id) = self.selected_one() else {
            self.date_picker = None;
            cx.notify();
            return;
        };
        if let DateField::RuleDate { rule, second } = field {
            self.date_picker = None;
            self.date_picker_time = None;
            self.date_picker_step = DatePickerStep::Calendar;
            self.date_picker_unit = ClockUnit::Hour;
            if let Some(modal) = self.modal.as_mut() {
                if let modals::Modal::Perspective { form } = modal {
                    if let Some(node) = form.rule_mut(rule) {
                        if second {
                            node.text2.set(date.format("%Y-%m-%d").to_string());
                        } else {
                            node.text.set(date.format("%Y-%m-%d").to_string());
                        }
                    }
                }
            }
            cx.notify();
            return;
        }
        let stamp = picker_stamp(date, self.date_picker_time);
        if let Some(message) = self.date_order_error(field, date) {
            self.date_picker = None;
            self.date_picker_time = None;
            self.date_error = Some((id.clone(), field, message.to_string()));
            self.toast = None;
            cx.notify();
            return;
        }
        self.date_picker = None;
        self.date_picker_time = None;
        self.date_picker_step = DatePickerStep::Calendar;
        self.date_picker_unit = ClockUnit::Hour;
        let mut patch = TaskPatch::default();
        match field {
            DateField::Defer => patch.defer = Some(Some(stamp)),
            DateField::Planned => patch.planned = Some(Some(stamp)),
            DateField::Due => patch.due = Some(Some(stamp)),
            DateField::Reminder => {
                patch.reminder_at = Some(Some(stamp));
                patch.reminder_match_due = Some(false);
            }
            DateField::RuleDate { .. } => unreachable!("handled before the task patch"),
        }
        let conflicts =
            flowlist_core::command::date_conflicts_for_task_patch(self.store.db(), &id, &patch);
        if let Some(conflict) = conflicts.first() {
            self.command_error = None;
            self.toast = None;
            self.date_error = Some((id, field, date_conflict_message(conflict)));
            cx.notify();
            return;
        }
        let _ = self.execute(Command::UpdateTask {
            id: id.clone(),
            patch,
        });
        if let Some(message) = self.command_error.take() {
            self.toast = None;
            self.date_error = Some((id, field, message));
        } else {
            self.date_error = None;
        }
        cx.notify();
    }

    /// A defer date after its due date would leave the task hidden until it
    /// is already overdue. Refuse the pick and explain beside the field.
    fn date_order_error(&self, field: DateField, date: chrono::NaiveDate) -> Option<&'static str> {
        let id = self.selected_one()?;
        let task = self.store.db().tasks.get(&id)?;
        match field {
            DateField::RuleDate { .. } => None,
            DateField::Defer if task.due.is_some_and(|due| date > due.local_date()) => {
                Some("The defer date can't be after the due date")
            }
            DateField::Due if task.defer.is_some_and(|defer| defer.local_date() > date) => {
                Some("The due date can't be before the defer date")
            }
            DateField::Reminder => {
                let stamp = picker_stamp(date, self.date_picker_time);
                (notifications::reminder_at(stamp) <= now())
                    .then_some("A reminder must be in the future")
            }
            _ => None,
        }
    }

    fn clear_date_picker(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
        if let DateField::RuleDate { rule, second } = field {
            self.date_picker = None;
            self.date_picker_time = None;
            self.date_picker_step = DatePickerStep::Calendar;
            self.date_picker_unit = ClockUnit::Hour;
            if let Some(modal) = self.modal.as_mut() {
                if let modals::Modal::Perspective { form } = modal {
                    if let Some(node) = form.rule_mut(rule) {
                        let today = local_date(now()).format("%Y-%m-%d").to_string();
                        match node.condition_ref() {
                            Some(flowlist_core::model::Condition::DateRange {
                                range: flowlist_core::model::DateRangeFilter::Between { .. },
                                ..
                            }) => {
                                if second {
                                    node.text2.clear();
                                } else {
                                    node.text.clear();
                                }
                            }
                            Some(flowlist_core::model::Condition::DateRange {
                                range: flowlist_core::model::DateRangeFilter::On { .. },
                                ..
                            }) => node.text.set(today),
                            _ => {
                                if second {
                                    node.text2.clear();
                                } else {
                                    node.text.clear();
                                }
                            }
                        }
                    }
                }
            }
            cx.notify();
            return;
        }
        let Some(id) = self.selected_one() else {
            self.date_picker = None;
            cx.notify();
            return;
        };
        self.date_picker = None;
        self.date_picker_time = None;
        self.date_picker_step = DatePickerStep::Calendar;
        self.date_picker_unit = ClockUnit::Hour;
        if self.date_picker_value(field).is_none() {
            cx.notify();
            return;
        }
        let mut patch = TaskPatch::default();
        match field {
            DateField::Defer => patch.defer = Some(None),
            DateField::Planned => patch.planned = Some(None),
            DateField::Due => patch.due = Some(None),
            DateField::Reminder => patch.reminder_at = Some(None),
            DateField::RuleDate { rule, second } => {
                if let Some(modal) = self.modal.as_mut() {
                    if let modals::Modal::Perspective { form } = modal {
                        if let Some(node) = form.rule_mut(rule) {
                            if second {
                                node.text2.clear();
                            } else {
                                node.text.clear();
                            }
                        }
                    }
                }
            }
        }
        self.execute(Command::UpdateTask { id, patch });
        cx.notify();
    }

    fn begin_edit(&mut self, field: EditField, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(task) = db.tasks.get(&id) else {
            cx.notify();
            return;
        };
        let value = match field {
            EditField::Title => task.title.clone(),
            EditField::Notes => task.notes.clone(),
            EditField::Project => task
                .project
                .as_ref()
                .map(|project| db.project_path(project))
                .unwrap_or_default(),
            EditField::Defer => task
                .defer
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Planned => task
                .planned
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Due => task
                .due
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Reminder => task
                .reminder_at
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Estimate => task.estimate_min.map(format_duration).unwrap_or_default(),
            EditField::Repeat => task
                .repeat
                .as_ref()
                .map(|rule| rule.every.max(1).to_string())
                .unwrap_or_default(),
        };
        self.field.set(value);
        if field == EditField::Project {
            // Opening the project picker selects the current path so typing
            // replaces it and immediately filters the dropdown.
            self.field.select_all();
        }
        self.field_error = None;
        self.suggestion_index = 0;
        self.suggestion_engaged = false;
        self.editing = Some(field);
        self.focus = FocusArea::Field;
        self.date_error = None;
        cx.notify();
    }

    /// Save the open field. Returns `false` when validation kept the editor
    /// open (an empty title, an unparseable date, or a project that needs
    /// confirmation), so callers can avoid advancing to the next field.
    fn commit_field(&mut self, cx: &mut Context<Self>) -> bool {
        if self.batch_editing.is_some() {
            return self.commit_batch_field(cx);
        }
        let Some(field) = self.editing else {
            self.focus = FocusArea::Content;
            cx.notify();
            return false;
        };
        let Some(id) = self.selected_one() else {
            self.editing = None;
            self.focus = FocusArea::Content;
            cx.notify();
            return false;
        };
        let raw = self.field.text().to_string();
        let value = raw.trim().to_string();
        self.date_error = None;
        let mut patch = TaskPatch::default();
        match field {
            EditField::Title => {
                if value.is_empty() {
                    self.field_error = Some("A task needs a title".to_string());
                    cx.notify();
                    return false;
                }
                patch.title = Some(value);
            }
            EditField::Notes => {
                patch.notes = Some(raw);
            }
            EditField::Project => {
                if value.is_empty() {
                    patch.project_path = Some(None);
                } else {
                    let known = {
                        let db = self.store.db();
                        db.find_project_by_path(&value).is_some()
                            || db.find_project_by_name(&value).is_some()
                    };
                    if !known {
                        // Ask before adding a project. A typo should not
                        // silently create a new entry in the sidebar.
                        self.editing = None;
                        self.field_error = None;
                        self.focus = FocusArea::Content;
                        self.open_modal(
                            modals::Modal::CreateProject {
                                task: id,
                                path: value,
                            },
                            cx,
                        );
                        return false;
                    }
                    patch.project_path = Some(Some(value));
                }
            }
            EditField::Defer | EditField::Planned | EditField::Due => {
                let stamp = if value.is_empty() {
                    None
                } else {
                    match parse_stamp(&value, now()) {
                        Ok(stamp) => Some(stamp),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return false;
                        }
                    }
                };
                match field {
                    EditField::Defer => patch.defer = Some(stamp),
                    EditField::Planned => patch.planned = Some(stamp),
                    _ => patch.due = Some(stamp),
                }
            }
            EditField::Reminder => {
                let stamp = if value.is_empty() {
                    None
                } else {
                    match parse_stamp(&value, now()) {
                        Ok(stamp) => Some(stamp),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return false;
                        }
                    }
                };
                if stamp
                    .as_ref()
                    .is_some_and(|stamp| notifications::reminder_at(*stamp) <= now())
                {
                    self.field_error = Some("A reminder must be in the future".to_string());
                    cx.notify();
                    return false;
                }
                patch.reminder_at = Some(stamp);
                if stamp.is_some() {
                    patch.reminder_match_due = Some(false);
                }
            }
            EditField::Estimate => {
                let estimate = if value.is_empty() {
                    None
                } else {
                    match parse_duration(&value) {
                        Ok(minutes) => Some(minutes),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return false;
                        }
                    }
                };
                patch.estimate_min = Some(estimate);
            }
            EditField::Repeat => {
                let every = match value.parse::<u32>() {
                    Ok(every) if (1..=999).contains(&every) => every,
                    _ => {
                        self.field_error = Some("Repeat every must be 1–999".to_string());
                        cx.notify();
                        return false;
                    }
                };
                let Some(mut rule) = self
                    .store
                    .db()
                    .tasks
                    .get(&id)
                    .and_then(|task| task.repeat.clone())
                else {
                    self.field_error = Some("Turn on Repeat first".to_string());
                    cx.notify();
                    return false;
                };
                rule.every = every;
                // Editing the interval returns to an interval schedule; the
                // weekday strip is the other mode.
                rule.weekdays = None;
                patch.repeat = Some(Some(rule));
            }
        }
        let conflicts =
            flowlist_core::command::date_conflicts_for_task_patch(self.store.db(), &id, &patch);
        if !conflicts.is_empty() {
            if field == EditField::Project {
                self.editing = None;
                self.field_error = None;
                self.focus = FocusArea::Content;
                self.open_modal(
                    modals::Modal::adjust_dates(
                        Command::UpdateTaskAdjusted { id, patch },
                        conflicts,
                    ),
                    cx,
                );
                return false;
            }
            if matches!(
                field,
                EditField::Defer | EditField::Planned | EditField::Due
            ) {
                let message = date_conflict_message(&conflicts[0]);
                if let Some(date_field) = field.date_field() {
                    self.date_error = Some((id.clone(), date_field, message));
                    self.field_error = None;
                } else {
                    self.field_error = Some(message);
                }
                cx.notify();
                return false;
            }
        }
        let _ = self.execute(Command::UpdateTask {
            id: id.clone(),
            patch,
        });
        if let Some(message) = self.command_error.take() {
            self.toast = None;
            if let Some(date_field) = field.date_field() {
                self.date_error = Some((id, date_field, message));
                self.field_error = None;
            } else {
                self.field_error = Some(message);
            }
            cx.notify();
            return false;
        }
        self.editing = None;
        self.field_error = None;
        self.date_error = None;
        self.focus = FocusArea::Content;
        cx.notify();
        true
    }

    fn add_tag_from_buffer(&mut self, cx: &mut Context<Self>) {
        let name = self.tag_buffer.take().trim().to_string();
        if name.is_empty() {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        }
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let existing = self.store.db().find_tag_path(&name);
        let tag = match existing {
            Some(tag) => tag,
            None => {
                let outcome = self.execute(Command::CreateTag {
                    name: name.clone(),
                    parent: None,
                    status: TagStatus::Available,
                    location: None,
                    position: Position::End,
                });
                match outcome.and_then(|outcome| outcome.created.first().cloned()) {
                    Some(created) => TagId::from_string(created),
                    None => {
                        self.focus = FocusArea::Content;
                        cx.notify();
                        return;
                    }
                }
            }
        };
        self.execute(Command::SetTaskTags {
            ids: vec![id],
            add: vec![tag],
            remove: Vec::new(),
        });
        self.focus = FocusArea::TagInput;
        cx.notify();
    }

    // ------------------------------------------------------------ mutations

    fn toggle_show_completed(&mut self, cx: &mut Context<Self>) {
        self.show_completed = !self.show_completed;
        self.remember_view_options();
        self.save_session();
        cx.notify();
    }

    fn choose_sort(&mut self, sort: Option<SortBy>, cx: &mut Context<Self>) {
        self.sort = sort;
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.remember_view_options();
        self.save_session();
        cx.notify();
    }

    fn choose_group(&mut self, group: Option<GroupBy>, cx: &mut Context<Self>) {
        self.group = group;
        self.group_menu_open = false;
        self.sort_menu_open = false;
        self.priority_menu_open = false;
        self.remember_view_options();
        self.save_session();
        cx.notify();
    }

    /// Set the global priority filter from the header menu. A priority view
    /// only makes sense while its filter is active, so it follows the choice.
    fn choose_priority_filter(&mut self, priority: Option<Priority>, cx: &mut Context<Self>) {
        self.priority_filter = priority;
        self.priority_menu_open = false;
        self.sort_menu_open = false;
        self.group_menu_open = false;
        if let View::Priority(_) = self.view {
            match priority {
                Some(priority) => self.set_view(View::Priority(priority)),
                None => self.set_view(View::Builtin(BuiltinPerspective::Inbox)),
            }
        }
        self.save_session();
        cx.notify();
    }

    fn toggle_only_available(&mut self, cx: &mut Context<Self>) {
        self.only_available = !self.only_available;
        self.remember_view_options();
        self.save_session();
        cx.notify();
    }

    /// Open, close or toggle the selected task's subtasks from the keyboard
    /// (`zo`, `zc`, `za`). A task without children keeps its state and says so.
    fn set_selected_subtasks(&mut self, open: Option<bool>, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let has_children = self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.has_children());
        if !has_children {
            self.toast = Some("This task has no subtasks".to_string());
            cx.notify();
            return;
        }
        let key = format!("task:{id}");
        let current = self.is_open(&key, false);
        let target = open.unwrap_or(!current);
        if target == current {
            return;
        }
        if target {
            self.collapsed_groups.remove(&key);
            self.expanded.insert(key);
        } else {
            self.expanded.remove(&key);
            self.collapsed_groups.insert(key);
        }
        cx.notify();
    }

    /// Flip a branch's disclosure. `default` must match how the caller paints
    /// the branch, otherwise the first click closes a branch that already
    /// looks closed and nothing moves.
    fn toggle_expanded(&mut self, key: String, default: bool, cx: &mut Context<Self>) {
        self.toggle_open(key, default);
        cx.notify();
    }

    /// Set a branch explicitly. Used by the task disclosure so its first click
    /// cannot depend on how the branch's default is interpreted elsewhere.
    fn set_branch_open(&mut self, key: String, open: bool) {
        if open {
            self.collapsed_groups.remove(&key);
            self.expanded.insert(key);
        } else {
            self.expanded.remove(&key);
            self.collapsed_groups.insert(key);
        }
    }

    fn toggle_group_collapsed(&mut self, key: String, cx: &mut Context<Self>) {
        let default = group_default_open(&key);
        self.toggle_open(key, default);
        cx.notify();
    }

    fn toggle_open(&mut self, key: String, default: bool) {
        if self.is_open(&key, default) {
            self.expanded.remove(&key);
            self.collapsed_groups.insert(key);
        } else {
            self.collapsed_groups.remove(&key);
            self.expanded.insert(key);
        }
    }

    fn is_open(&self, key: &str, default: bool) -> bool {
        if self.expanded.contains(key) {
            true
        } else if self.collapsed_groups.contains(key) {
            false
        } else {
            default
        }
    }

    /// Remember which sidebar row a dragged project or folder is over. The
    /// folder row, LIBRARY heading and the empty sidebar area paint themselves
    /// from this.
    fn set_drag_target(&mut self, target: Option<String>, cx: &mut Context<Self>) {
        if self.sidebar_drag_target != target {
            self.sidebar_drag_target = target;
            cx.notify();
        }
    }

    fn set_content_drop_target(&mut self, target: Option<String>, cx: &mut Context<Self>) {
        if self.content_drop_target != target {
            self.content_drop_target = target;
            cx.notify();
        }
    }

    /// A drop fires during mouse-up while the window is mid-frame, so a plain
    /// `notify` records the change without scheduling the redraw that shows it.
    /// Notify again from a background tick, once the frame has finished. This
    /// is what makes a drag look like it lands where it was dropped.
    fn repaint_after_drop(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1))
                .await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    /// Move a dragged project into a folder. Dropping it on the same folder is
    /// a no-op, so it adds no history entry.
    fn drop_on_folder(&mut self, folder: FolderId, payload: &DragProject, cx: &mut Context<Self>) {
        let already_there = self
            .store
            .db()
            .projects
            .get(&payload.id)
            .is_some_and(|project| project.folder.as_ref() == Some(&folder));
        if !already_there {
            self.execute(Command::MoveProject {
                id: payload.id.clone(),
                placement: ContainerPlacement {
                    parent: Some(folder),
                    position: ContainerPosition::End,
                },
            });
        }
        self.sidebar_drag_target = None;
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Move a dragged project back to the Library root. Dropping it where it
    /// already sits is a no-op, so it adds no history entry.
    fn drop_on_library(&mut self, payload: &DragProject, cx: &mut Context<Self>) {
        let at_root = self
            .store
            .db()
            .projects
            .get(&payload.id)
            .is_some_and(|project| project.folder.is_none());
        if !at_root {
            self.execute(Command::MoveProject {
                id: payload.id.clone(),
                placement: ContainerPlacement {
                    parent: None,
                    position: ContainerPosition::End,
                },
            });
        }
        self.sidebar_drag_target = None;
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Whether a folder can be dropped into `target`: never onto itself, and
    /// never inside its own subtree, since that would detach the tree.
    fn folder_can_hold(&self, dragged: &FolderId, target: &FolderId) -> bool {
        dragged != target && !self.store.db().folder_descendants(dragged).contains(target)
    }

    /// Nest a dragged folder inside another folder. Dropping it where it
    /// already sits is a no-op, so it adds no history entry.
    fn drop_folder_on_folder(
        &mut self,
        target: FolderId,
        payload: &DragFolder,
        cx: &mut Context<Self>,
    ) {
        if !self.folder_can_hold(&payload.id, &target) {
            self.sidebar_drag_target = None;
            cx.notify();
            return;
        }
        let already_there = self
            .store
            .db()
            .folders
            .get(&payload.id)
            .is_some_and(|folder| folder.parent.as_ref() == Some(&target));
        if !already_there {
            self.execute(Command::MoveFolder {
                id: payload.id.clone(),
                placement: ContainerPlacement {
                    parent: Some(target),
                    position: ContainerPosition::End,
                },
            });
        }
        self.sidebar_drag_target = None;
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Move a dragged folder back to the Library root.
    fn drop_folder_on_library(&mut self, payload: &DragFolder, cx: &mut Context<Self>) {
        let at_root = self
            .store
            .db()
            .folders
            .get(&payload.id)
            .is_some_and(|folder| folder.parent.is_none());
        if !at_root {
            self.execute(Command::MoveFolder {
                id: payload.id.clone(),
                placement: ContainerPlacement {
                    parent: None,
                    position: ContainerPosition::End,
                },
            });
        }
        self.sidebar_drag_target = None;
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Move tasks, but preflight the container's dates first. If the move
    /// would have to change stored dates, nothing moves until the user
    /// confirms the adjustment.
    fn move_tasks_checked(
        &mut self,
        ids: Vec<TaskId>,
        placement: TaskPlacement,
        cx: &mut Context<Self>,
    ) {
        let conflicts =
            flowlist_core::command::date_conflicts_for_move(self.store.db(), &ids, &placement);
        if conflicts.is_empty() {
            self.execute(Command::MoveTasks { ids, placement });
            cx.notify();
            self.repaint_after_drop(cx);
        } else {
            self.open_modal(
                modals::Modal::adjust_dates(
                    Command::MoveTasksAdjusted { ids, placement },
                    conflicts,
                ),
                cx,
            );
        }
    }

    fn drop_on_project(&mut self, project: ProjectId, payload: &DragTask, cx: &mut Context<Self>) {
        let ids = self.drag_ids(payload);
        if payload.tag {
            return;
        }
        self.move_tasks_checked(
            ids,
            TaskPlacement::Project {
                project,
                position: Position::End,
            },
            cx,
        );
        cx.notify();
        self.repaint_after_drop(cx);
    }

    fn drop_on_task(&mut self, target: TaskId, payload: &DragTask, cx: &mut Context<Self>) {
        if payload.tag || payload.id == target {
            return;
        }
        let db = self.store.db();
        if db.task_subtree(&payload.id).contains(&target) {
            self.toast = Some("A task cannot be nested inside itself".to_string());
            cx.notify();
            return;
        }
        let ids = self.drag_ids(payload);
        self.move_tasks_checked(
            ids,
            TaskPlacement::Parent {
                parent: target,
                position: Position::End,
            },
            cx,
        );
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Dropping a task in a gap between root rows moves it out of any parent
    /// and into the project or Inbox at that position.
    fn drop_on_root(&mut self, before: Option<TaskId>, payload: &DragTask, cx: &mut Context<Self>) {
        if payload.tag {
            return;
        }
        // A gap before a row files the task beside that row, in that row's
        // project. The end gap in a project or Inbox files at the end.
        let placement = match &before {
            Some(before) => match self
                .store
                .db()
                .tasks
                .get(before)
                .and_then(|task| task.project.clone())
            {
                Some(project) => TaskPlacement::Project {
                    project,
                    position: Position::Before(before.clone()),
                },
                None => TaskPlacement::Inbox(Position::Before(before.clone())),
            },
            None => match &self.view {
                View::Project(project) => TaskPlacement::Project {
                    project: project.clone(),
                    position: Position::End,
                },
                View::Builtin(BuiltinPerspective::Inbox) => TaskPlacement::Inbox(Position::End),
                _ => {
                    self.content_drop_target = None;
                    cx.notify();
                    return;
                }
            },
        };
        let ids = self.drag_ids(payload);
        self.move_tasks_checked(ids, placement, cx);
        self.content_drop_target = None;
        cx.notify();
        self.repaint_after_drop(cx);
    }

    /// Reorder or reparent a dragged task beside an inspector subtask row.
    fn drop_subtask_beside(
        &mut self,
        parent: TaskId,
        target: TaskId,
        payload: &DragTask,
        cx: &mut Context<Self>,
    ) {
        if payload.tag || payload.id == target {
            return;
        }
        let db = self.store.db();
        if db.task_subtree(&payload.id).contains(&target) {
            self.toast = Some("A task cannot be nested inside itself".to_string());
            cx.notify();
            return;
        }
        let ids = self.drag_ids(payload);
        // Dropping on a row above moves before it; dropping below moves after
        // it. Always using `After` made dragging upward a no-op.
        let order = db
            .tasks
            .get(&parent)
            .map(|task| task.child_order.clone())
            .unwrap_or_default();
        let from = order.iter().position(|id| id == &payload.id);
        let to = order.iter().position(|id| id == &target);
        let position = match (from, to) {
            (Some(from), Some(to)) if from > to => Position::Before(target),
            _ => Position::After(target),
        };
        self.move_tasks_checked(ids, TaskPlacement::Parent { parent, position }, cx);
    }

    fn trash_subtask(&mut self, id: TaskId, cx: &mut Context<Self>) {
        self.execute(Command::DeleteTasks { ids: vec![id] });
        self.toast = Some("Moved subtask to Trash · ⌘Z to undo".to_string());
        cx.notify();
    }

    fn drop_checklist_item(
        &mut self,
        task: TaskId,
        target: ChecklistId,
        payload: &drag::DragChecklist,
        cx: &mut Context<Self>,
    ) {
        if payload.item == target {
            return;
        }
        let Some(index) = self
            .store
            .db()
            .tasks
            .get(&task)
            .and_then(|task| task.checklist.iter().position(|item| item.id == target))
        else {
            return;
        };
        self.move_checklist_item(task, payload.item.clone(), index, cx);
    }

    /// Move a child task to the end of its project (or the Inbox), leaving
    /// its former parent behind.
    fn promote_subtask(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let project = self
            .store
            .db()
            .tasks
            .get(&id)
            .and_then(|task| task.project.clone());
        let placement = match project {
            Some(project) => TaskPlacement::Project {
                project,
                position: Position::End,
            },
            None => TaskPlacement::Inbox(Position::End),
        };
        self.move_tasks_checked(vec![id], placement, cx);
        self.toast = Some("Moved out from under its parent".to_string());
        self.save_session();
        cx.notify();
    }

    /// Dropping a task on a Forecast day rewrites its due date. A timed task
    /// keeps its clock time; an all-day date stays all-day.
    fn drop_on_forecast_day(
        &mut self,
        date: chrono::NaiveDate,
        payload: &DragTask,
        cx: &mut Context<Self>,
    ) {
        if payload.tag {
            return;
        }
        let ids = self.drag_ids(payload);
        let dues: Vec<(TaskId, Stamp)> = {
            let db = self.store.db();
            ids.into_iter()
                .map(|id| {
                    let due = db
                        .tasks
                        .get(&id)
                        .and_then(|task| task.due)
                        .filter(|stamp| stamp.has_time)
                        .map(|stamp| Stamp::on(date, stamp.local_time()))
                        .unwrap_or_else(|| Stamp::date_only(date));
                    (id, due)
                })
                .collect()
        };
        for (id, due) in dues {
            self.execute(Command::UpdateTask {
                id,
                patch: TaskPatch {
                    due: Some(Some(due)),
                    ..Default::default()
                },
            });
        }
        self.forecast_date = Some(date);
        self.forecast_past = false;
        self.forecast_future = false;
        self.forecast_month =
            chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
        self.toast = Some(format!("Moved to {}", date.format("%b %-d")));
        cx.notify();
        self.repaint_after_drop(cx);
    }

    fn drop_on_tag(&mut self, tag: TagId, payload: &DragTask, cx: &mut Context<Self>) {
        let ids = self.drag_ids(payload);
        self.execute(Command::SetTaskTags {
            ids,
            add: vec![tag],
            remove: Vec::new(),
        });
        cx.notify();
        self.repaint_after_drop(cx);
    }

    fn drag_ids(&self, payload: &DragTask) -> Vec<TaskId> {
        if self.selected.contains(&payload.id) {
            self.selected.clone()
        } else {
            vec![payload.id.clone()]
        }
    }

    // ------------------------------------------------------------ clipboard

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let db = self.store.db();
        let lines: Vec<String> = self
            .selected
            .iter()
            .filter_map(|id| db.tasks.get(id))
            .map(|task| {
                let project = task
                    .project
                    .as_ref()
                    .map(|project| db.project_path(project))
                    .unwrap_or_else(|| "Inbox".to_string());
                format!("{}\t{}", task.title, project)
            })
            .collect();
        if lines.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(lines.join("\n")));
        self.toast = Some(format!("Copied {}", plural(lines.len(), "task", "tasks")));
        cx.notify();
    }

    // -------------------------------------------------------------- modals

    fn open_modal(&mut self, modal: modals::Modal, cx: &mut Context<Self>) {
        self.pending_modal = None;
        self.modal_error = None;
        // Settings previews theme changes live, so remember what to put back
        // if the sheet is cancelled.
        if matches!(modal, modals::Modal::Settings { .. }) {
            self.theme_on_open = Some(self.theme);
            self.accent_on_open = Some((self.plain_accent_light, self.plain_accent_dark));
            self.traffic_lights_on_open = Some(self.traffic_lights_visible);
        } else {
            self.theme_on_open = None;
            self.accent_on_open = None;
            self.traffic_lights_on_open = None;
        }
        self.modal = Some(modal);
        self.modal_focus = 0;
        self.view_menu = None;
        self.view_menu_cursor = 0;
        self.view_boxes.borrow_mut().clear();
        self.settings_toggle_boxes.borrow_mut().clear();
        self.modal_scroll.set_offset(point(px(0.), px(0.)));
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.sidebar_menu = None;
        self.focus = FocusArea::Modal;
        // A sheet belongs to the window rather than a field session, so the
        // inspector stops editing and a first-time task stops being a draft.
        self.editing = None;
        self.field_error = None;
        self.draft_task = None;
        cx.notify();
    }

    fn entity_for_view(&self) -> Option<modals::EntityRef> {
        match &self.view {
            View::Project(id) => Some(modals::EntityRef::Project(id.clone())),
            View::Folder(id) => Some(modals::EntityRef::Folder(id.clone())),
            View::Tag(id) => Some(modals::EntityRef::Tag(id.clone())),
            View::Custom(id) => Some(modals::EntityRef::Perspective(id.clone())),
            View::Builtin(_) | View::Priority(_) => None,
        }
    }

    fn close_modal(&mut self, cx: &mut Context<Self>) {
        if let Some((light, dark)) = self.accent_on_open.take() {
            self.plain_accent_light = light;
            self.plain_accent_dark = dark;
        }
        if let Some(theme) = self.theme_on_open.take() {
            self.theme = theme;
        }
        if let Some(visible) = self.traffic_lights_on_open.take() {
            self.traffic_lights_visible = visible;
        }
        self.modal = None;
        self.modal_focus = 0;
        self.pending_modal = None;
        self.modal_error = None;
        self.date_picker = None;
        self.date_picker_time = None;
        self.date_picker_step = DatePickerStep::Calendar;
        self.view_menu = None;
        self.view_menu_cursor = 0;
        self.modal_scroll.set_offset(point(px(0.), px(0.)));
        self.focus = FocusArea::Content;
        self.save_session();
        cx.notify();
    }

    fn commit_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        let uninstall = matches!(modal, modals::Modal::Uninstall { .. });
        self.date_picker = None;
        self.date_picker_time = None;
        self.date_picker_step = DatePickerStep::Calendar;
        self.view_menu = None;
        self.view_menu_cursor = 0;
        self.command_error = None;
        let result = self
            .apply_modal(&modal)
            .and_then(|()| match self.command_error.take() {
                Some(error) => Err(error),
                None => Ok(()),
            });
        match result {
            Ok(()) => {
                self.modal = None;
                self.modal_error = None;
                self.modal_focus = 0;
                if let Some(modal) = self.pending_modal.take() {
                    self.open_modal(modal, cx);
                } else {
                    self.focus = FocusArea::Content;
                }
                self.sync_notifications(cx);
                // The previewed theme is now committed; drop the undo copy.
                self.theme_on_open = None;
                self.accent_on_open = None;
                self.traffic_lights_on_open = None;
                if uninstall {
                    cx.quit();
                }
            }
            Err(message) => {
                self.modal = Some(modal);
                self.modal_error = Some(message);
                self.toast = None;
                self.focus = FocusArea::Modal;
            }
        }
        cx.notify();
    }

    fn apply_modal(&mut self, modal: &modals::Modal) -> std::result::Result<(), String> {
        match modal {
            modals::Modal::Rename { entity, name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A name cannot be empty".to_string());
                }
                match entity {
                    modals::EntityRef::Project(id) => {
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch: ProjectPatch {
                                name: Some(name),
                                ..Default::default()
                            },
                        });
                    }
                    modals::EntityRef::Folder(id) => {
                        self.execute(Command::RenameFolder {
                            id: id.clone(),
                            name,
                        });
                    }
                    modals::EntityRef::Tag(id) => {
                        self.execute(Command::UpdateTag {
                            id: id.clone(),
                            patch: TagPatch {
                                name: Some(name),
                                ..Default::default()
                            },
                        });
                    }
                    modals::EntityRef::Perspective(id) => {
                        let Some(mut perspective) = self.store.db().perspectives.get(id).cloned()
                        else {
                            return Err("That perspective no longer exists".to_string());
                        };
                        perspective.name = name;
                        self.execute(Command::SavePerspective { perspective });
                    }
                }
            }
            modals::Modal::NewProject {
                editing,
                name,
                notes,
                review_days,
                defer,
                planned,
                due,
                kind,
                icon,
                fullscreen_reminder,
                ..
            } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A project needs a name".to_string());
                }
                let review_interval_days = if review_days.text().trim().is_empty() {
                    None
                } else {
                    Some(
                        review_days
                            .text()
                            .trim()
                            .parse::<u32>()
                            .map_err(|_| "Review days must be a whole number".to_string())?
                            .max(1),
                    )
                };
                let parse_date = |buffer: &text::TextBuffer,
                                  label: &str|
                 -> std::result::Result<
                    Option<flowlist_core::time::Stamp>,
                    String,
                > {
                    let text = buffer.text().trim();
                    if text.is_empty() {
                        Ok(None)
                    } else {
                        flowlist_core::parse::parse_stamp(text, now())
                            .map(Some)
                            .map_err(|error| format!("{label} date: {error}"))
                    }
                };
                let defer = parse_date(defer, "Defer")?;
                let planned = parse_date(planned, "Planned")?;
                let due = parse_date(due, "Due")?;
                if let Some(id) = editing {
                    let patch = ProjectPatch {
                        name: Some(name),
                        notes: Some(notes.text().to_string()),
                        review_interval_days,
                        defer: Some(defer),
                        planned: Some(planned),
                        due: Some(due),
                        kind: Some(*kind),
                        icon: Some(icon.clone()),
                        fullscreen_reminder: Some(*fullscreen_reminder),
                        ..Default::default()
                    };
                    let conflicts = flowlist_core::command::date_conflicts_for_project_patch(
                        self.store.db(),
                        id,
                        &patch,
                    );
                    if conflicts.is_empty() {
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch,
                        });
                    } else {
                        self.pending_modal = Some(modals::Modal::adjust_dates(
                            Command::UpdateProjectAdjusted {
                                id: id.clone(),
                                patch,
                            },
                            conflicts,
                        ));
                    }
                    return Ok(());
                }
                let db = self.store.db();
                let folder = match &self.view {
                    View::Folder(id) => Some(id.clone()),
                    View::Project(id) => db
                        .projects
                        .get(id)
                        .and_then(|project| project.folder.clone()),
                    _ => None,
                };
                let mut new_project = NewProject::new(name);
                new_project.notes = notes.text().to_string();
                new_project.kind = *kind;
                new_project.icon = icon.clone();
                new_project.review_interval_days = review_interval_days;
                new_project.defer = defer;
                new_project.planned = planned;
                new_project.due = due;
                new_project.sequential = *kind == ProjectKind::Sequential;
                new_project.fullscreen_reminder = *fullscreen_reminder;
                new_project.folder = folder;
                new_project.position = ContainerPosition::End;
                if let Some(outcome) = self.execute(Command::CreateProject(new_project)) {
                    if let Some(created) = outcome.created.first() {
                        if let Ok(id) = self.store.db().resolve_project(created) {
                            self.view = View::Project(id);
                            self.selected.clear();
                            self.anchor = None;
                        }
                    }
                }
            }
            modals::Modal::CreateProject { task, path } => {
                self.execute(Command::UpdateTask {
                    id: task.clone(),
                    patch: TaskPatch {
                        project_path: Some(Some(path.clone())),
                        ..Default::default()
                    },
                });
            }
            modals::Modal::CustomNotification { target, at } => {
                let text = at.text().trim().to_string();
                if text.is_empty() {
                    return Err("Enter a date and time for the notification".to_string());
                }
                let mut stamp = parse_stamp(&text, now())
                    .map_err(|error| format!("Invalid notification time: {error}"))?;
                if !stamp.has_time {
                    let time = self
                        .store
                        .db()
                        .settings
                        .default_custom_notification_time
                        .unwrap_or_else(|| {
                            chrono::NaiveTime::from_hms_opt(9, 0, 0)
                                .unwrap_or(chrono::NaiveTime::MIN)
                        });
                    stamp = Stamp::on(stamp.local_date(), time);
                }
                match target {
                    NotificationTarget::Task(id) => {
                        let mut custom = self
                            .store
                            .db()
                            .tasks
                            .get(id)
                            .map(|task| task.custom_notifications.clone())
                            .unwrap_or_default();
                        if !custom.iter().any(|existing| existing.at == stamp.at) {
                            custom.push(stamp);
                        }
                        custom.sort_by_key(|stamp| stamp.at);
                        self.execute(Command::UpdateTask {
                            id: id.clone(),
                            patch: TaskPatch {
                                custom_notifications: Some(custom),
                                ..Default::default()
                            },
                        });
                    }
                    NotificationTarget::Project(id) => {
                        let mut custom = self
                            .store
                            .db()
                            .projects
                            .get(id)
                            .map(|project| project.custom_notifications.clone())
                            .unwrap_or_default();
                        if !custom.iter().any(|existing| existing.at == stamp.at) {
                            custom.push(stamp);
                        }
                        custom.sort_by_key(|stamp| stamp.at);
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch: ProjectPatch {
                                custom_notifications: Some(custom),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
            modals::Modal::RelativeNotification {
                target,
                event,
                offset,
            } => {
                let offset_text = offset.text().trim();
                let spec = if offset_text.is_empty() {
                    "0"
                } else {
                    offset_text
                };
                let parsed = TaskAlert::parse_list(&format!("{}:{spec}", event.slug()))
                    .map_err(|error| format!("Invalid alert timing: {error}"))?;
                let Some(alert) = parsed.into_iter().next() else {
                    return Err("Enter a timing such as -15m".to_string());
                };
                match target {
                    NotificationTarget::Task(id) => {
                        let mut alerts = self
                            .store
                            .db()
                            .tasks
                            .get(id)
                            .map(|task| task.alerts.clone())
                            .unwrap_or_default();
                        if !alerts.contains(&alert) {
                            alerts.push(alert);
                        }
                        self.execute(Command::UpdateTask {
                            id: id.clone(),
                            patch: TaskPatch {
                                alerts: Some(alerts),
                                ..Default::default()
                            },
                        });
                    }
                    NotificationTarget::Project(id) => {
                        let mut alerts = self
                            .store
                            .db()
                            .projects
                            .get(id)
                            .map(|project| project.alerts.clone())
                            .unwrap_or_default();
                        if !alerts.contains(&alert) {
                            alerts.push(alert);
                        }
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch: ProjectPatch {
                                alerts: Some(alerts),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
            modals::Modal::DeleteProject { id, .. } => {
                self.execute(Command::DeleteProject { id: id.clone() });
                if self.view == View::Project(id.clone()) {
                    self.view = View::Builtin(BuiltinPerspective::Inbox);
                    self.selected.clear();
                    self.anchor = None;
                }
                self.save_session();
            }
            modals::Modal::DeleteTag { id, .. } => {
                self.execute(Command::DeleteTag { id: id.clone() });
                if self.view == View::Tag(id.clone()) {
                    self.view = View::Builtin(BuiltinPerspective::Inbox);
                    self.selected.clear();
                    self.anchor = None;
                }
                self.save_session();
            }
            modals::Modal::DeleteContainers { projects, folders } => {
                self.execute(Command::DeleteContainers {
                    projects: projects.clone(),
                    folders: folders.clone(),
                });
                self.sidebar_selection.clear();
                if !view_exists(self.store.db(), &self.view) {
                    self.view = View::Builtin(BuiltinPerspective::Inbox);
                    self.selected.clear();
                    self.anchor = None;
                }
                self.save_session();
            }
            modals::Modal::DeleteFolder { id, .. } => {
                self.execute(Command::DeleteFolder { id: id.clone() });
                if self.view == View::Folder(id.clone()) {
                    self.view = View::Builtin(BuiltinPerspective::Inbox);
                    self.selected.clear();
                    self.anchor = None;
                }
                self.save_session();
            }
            modals::Modal::DeleteTask { ids, whole, .. } => {
                let saved = self.execute(if *whole {
                    Command::DeleteTaskSeries { ids: ids.clone() }
                } else {
                    Command::DeleteTasks { ids: ids.clone() }
                });
                if saved.is_none() {
                    return Ok(());
                }
                self.selected.clear();
                self.anchor = None;
                self.toast = Some(if *whole {
                    "Moved the repeating series to Trash · ⌘Z to undo".to_string()
                } else {
                    format!("Moved {} occurrence(s) to Trash · ⌘Z to undo", ids.len())
                });
            }
            modals::Modal::HideSubtasks { id, .. } => {
                self.execute(Command::SetTaskStepMode {
                    id: id.clone(),
                    sequential: false,
                    steps_one_off: true,
                });
            }
            modals::Modal::PriorityScope {
                id, priority, all, ..
            } => {
                let saved = self.execute(Command::SetTaskPriority {
                    ids: vec![id.clone()],
                    priority: *priority,
                    series: *all,
                });
                if saved.is_some() {
                    self.toast = Some(if *all {
                        "Priority updated for the whole series".to_string()
                    } else {
                        "Priority updated for this occurrence".to_string()
                    });
                }
            }
            modals::Modal::NewFolder {
                editing,
                name,
                fullscreen_reminder,
            } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A folder needs a name".to_string());
                }
                if let Some(id) = editing {
                    self.execute(Command::RenameFolder {
                        id: id.clone(),
                        name,
                    });
                    return Ok(());
                }
                let parent = match &self.view {
                    View::Folder(id) => Some(id.clone()),
                    _ => None,
                };
                self.execute(Command::CreateFolder {
                    name,
                    parent,
                    position: ContainerPosition::End,
                    fullscreen_reminder: *fullscreen_reminder,
                });
            }
            modals::Modal::AttachLink {
                target,
                url,
                name,
                replace,
            } => {
                let Some(link) = flowlist_core::model::attachment::normalize_web_link(url.text())
                else {
                    return Err("Links start with http:// or https://".to_string());
                };
                let link = flowlist_core::model::attachment::link_with_name(&link, name.text());
                let apply = |attachments: &mut Vec<String>| -> std::result::Result<(), String> {
                    if let Some(original) = replace {
                        if let Some(index) = attachments.iter().position(|item| item == original) {
                            attachments[index] = link.clone();
                            return Ok(());
                        }
                    }
                    if attachments.iter().any(|item| {
                        flowlist_core::model::attachment::attachment_url(item)
                            == flowlist_core::model::attachment::attachment_url(&link)
                    }) {
                        return Err("That link is already attached".to_string());
                    }
                    attachments.push(link.clone());
                    Ok(())
                };
                match target {
                    modals::AttachmentTarget::Task(id) => {
                        let Some(mut attachments) = self
                            .store
                            .db()
                            .tasks
                            .get(id)
                            .map(|task| task.attachments.clone())
                        else {
                            return Err("That task no longer exists".to_string());
                        };
                        apply(&mut attachments)?;
                        self.execute(Command::UpdateTask {
                            id: id.clone(),
                            patch: TaskPatch {
                                attachments: Some(attachments),
                                ..Default::default()
                            },
                        });
                    }
                    modals::AttachmentTarget::Project(id) => {
                        let Some(mut attachments) = self
                            .store
                            .db()
                            .projects
                            .get(id)
                            .map(|project| project.attachments.clone())
                        else {
                            return Err("That project no longer exists".to_string());
                        };
                        apply(&mut attachments)?;
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch: ProjectPatch {
                                attachments: Some(attachments),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
            modals::Modal::NewTag { name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A tag needs a name".to_string());
                }
                let parent = match &self.view {
                    View::Tag(id) => Some(id.clone()),
                    _ => None,
                };
                self.execute(Command::CreateTag {
                    name,
                    parent,
                    status: TagStatus::Available,
                    location: None,
                    position: Position::End,
                });
            }
            modals::Modal::RemoveAttachment { target, path, .. } => match target {
                modals::AttachmentTarget::Task(id) => {
                    let Some(mut attachments) = self
                        .store
                        .db()
                        .tasks
                        .get(id)
                        .map(|task| task.attachments.clone())
                    else {
                        return Err("That task no longer exists".to_string());
                    };
                    attachments.retain(|candidate| candidate != path);
                    self.execute(Command::UpdateTask {
                        id: id.clone(),
                        patch: TaskPatch {
                            attachments: Some(attachments),
                            ..Default::default()
                        },
                    });
                }
                modals::AttachmentTarget::Project(id) => {
                    let Some(mut attachments) = self
                        .store
                        .db()
                        .projects
                        .get(id)
                        .map(|project| project.attachments.clone())
                    else {
                        return Err("That project no longer exists".to_string());
                    };
                    attachments.retain(|candidate| candidate != path);
                    self.execute(Command::UpdateProject {
                        id: id.clone(),
                        patch: ProjectPatch {
                            attachments: Some(attachments),
                            ..Default::default()
                        },
                    });
                }
            },
            modals::Modal::Uninstall { purge } => {
                let root = self.store.paths().root.clone();
                let plan = crate::uninstall::plan(*purge, &root);
                crate::uninstall::schedule(&plan)?;
            }
            modals::Modal::IconPicker { .. } => {}
            modals::Modal::AdjustDates { command, .. } => {
                self.execute(command.clone());
            }
            modals::Modal::ReviewInterval {
                id,
                days,
                next_review,
                ..
            } => {
                let value = days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Review days must be a number".to_string())?;
                if !(1..=365).contains(&value) {
                    return Err("Review days must be between 1 and 365".to_string());
                }
                let next = flowlist_core::parse::parse_date(next_review.text().trim(), now())
                    .map_err(|error| format!("Next review: {error}"))?;
                self.execute(Command::UpdateProject {
                    id: id.clone(),
                    patch: ProjectPatch {
                        review_interval_days: Some(value),
                        next_review: Some(next),
                        ..Default::default()
                    },
                });
            }
            modals::Modal::Perspective { form } => {
                if form.name.text().trim().is_empty() {
                    return Err("A perspective needs a name".to_string());
                }
                let perspective = form.to_perspective();
                let id = perspective.id.clone();
                if let Some(outcome) = self.execute(Command::SavePerspective { perspective }) {
                    let saved = outcome
                        .affected
                        .first()
                        .map(flowlist_core::ids::PerspectiveId::from_string)
                        .unwrap_or(id);
                    if self.store.db().perspectives.contains_key(&saved) {
                        self.set_view(View::Custom(saved));
                    }
                }
            }
            modals::Modal::Import { text, merge } => {
                let data = text.text().to_string();
                if data.trim().is_empty() {
                    return Err("Paste a library or TaskPaper text first".to_string());
                }
                let db = if data.trim_start().starts_with('{') {
                    export::from_json(&data).map_err(|error| error.to_string())?
                } else {
                    taskpaper::import(&data).map_err(|error| error.to_string())?
                };
                let mode = if *merge {
                    flowlist_core::command::ImportMode::Merge
                } else {
                    flowlist_core::command::ImportMode::Replace
                };
                self.execute(Command::Import { db, mode });
            }
            modals::Modal::Settings {
                review_days,
                completed_days,
                changed_days,
                due_soon_days,
                radius,
                trash_days,
                vim_mode,
                notifications,
                calendar_events,
                calendar_hidden,
                forecast_due_only,
                fullscreen_reminders,
                pause_media_on_reminder,
                reminder_background,
                reminder_match_theme,
                reminder_wallpaper_light,
                reminder_wallpaper_dark,
                reminder_wallpaper_separate,
                reminder_use_wallpaper,
                reminder_wallpaper_opacity,
                fullscreen_snooze_minutes,
                wallpaper_light,
                wallpaper_dark,
                wallpaper_separate,
                use_wallpaper,
                wallpaper_opacity,
                keep_completed_subtasks,
                forecast_due_counts,
                forecast_tag,
                forecast_flagged,
                default_defer_notification,
                default_planned_notification,
                default_latest_start_notification,
                default_due_notification,
                custom_notification_time,
                focus_banner,
                toolbar_labels,
                hide_empty_views,
                follow_system,
                theme,
                appearance,
                sidebar,
                accent_light,
                accent_dark,
                traffic_lights,
                ..
            } => {
                let review = review_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Review days must be a number".to_string())?;
                let completed = completed_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Completed days must be a number".to_string())?;
                let changed = changed_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Changed days must be a number".to_string())?;
                let due_soon = due_soon_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Due soon days must be a number".to_string())?;
                let radius = radius
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Radius must be a number".to_string())?;
                let trash = trash_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Trash retention must be a number".to_string())?;
                let tag_path = forecast_tag.text().trim().to_string();
                let custom_time_text = custom_notification_time.text().trim().to_string();
                let custom_time = if custom_time_text.is_empty() {
                    None
                } else {
                    Some(
                        chrono::NaiveTime::parse_from_str(&custom_time_text, "%H:%M")
                            .map_err(|_| "Custom notification time must be HH:MM".to_string())?,
                    )
                };
                let forecast_tag = if tag_path.is_empty() {
                    None
                } else {
                    Some(
                        self.store
                            .db()
                            .find_tag_path(&tag_path)
                            .ok_or_else(|| format!("Tag not found: {tag_path}"))?,
                    )
                };
                self.execute(Command::UpdateSettings(SettingsPatch {
                    default_review_days: Some(review),
                    show_completed_days: Some(completed),
                    changed_days: Some(changed),
                    due_soon_days: Some(due_soon),
                    nearby_radius_km: Some(radius),
                    trash_retention_days: Some(trash),
                    forecast_due_counts: Some(*forecast_due_counts),
                    forecast_tag: Some(forecast_tag),
                    forecast_flagged: Some(*forecast_flagged),
                    default_defer_notification: Some(*default_defer_notification),
                    default_planned_notification: Some(*default_planned_notification),
                    default_latest_start_notification: Some(*default_latest_start_notification),
                    default_due_notification: Some(*default_due_notification),
                    default_custom_notification_time: Some(custom_time),
                    sidebar: Some(sidebar.clone()),
                    ..Default::default()
                }));
                if let Err(error) = self.store.sweep_trash() {
                    self.toast = Some(format!("Could not empty expired trash: {error}"));
                }
                self.vim_mode = *vim_mode;
                self.due_notifications = *notifications;
                self.forecast_due_only = *forecast_due_only;
                self.calendar_hidden = calendar_hidden.clone();
                if self.calendar_events_enabled != *calendar_events {
                    self.calendar_events_enabled = *calendar_events;
                    self.calendar_last_refresh = None;
                    if self.calendar_events_enabled {
                        calendar::request_permission();
                    } else {
                        self.calendar_events.clear();
                        self.calendar_calendars.clear();
                    }
                } else if self.calendar_events_enabled {
                    self.calendar_last_refresh = None;
                }
                self.save_session();
                self.fullscreen_reminders = *fullscreen_reminders;
                self.pause_media_on_reminder = *pause_media_on_reminder;
                self.reminder_background = *reminder_background;
                self.reminder_match_theme = *reminder_match_theme;
                self.reminder_wallpaper_light = reminder_wallpaper_light.clone();
                self.reminder_wallpaper_dark = reminder_wallpaper_dark.clone();
                self.reminder_wallpaper_separate = *reminder_wallpaper_separate;
                self.reminder_use_wallpaper = *reminder_use_wallpaper;
                self.reminder_wallpaper_opacity = *reminder_wallpaper_opacity;
                self.reminder_snooze_minutes = (*fullscreen_snooze_minutes).clamp(1, 1440);
                self.wallpaper_light = wallpaper_light.clone();
                self.wallpaper_dark = wallpaper_dark.clone();
                self.wallpaper_separate = *wallpaper_separate;
                self.use_wallpaper = *use_wallpaper;
                self.wallpaper_opacity = *wallpaper_opacity;
                self.keep_completed_subtasks = *keep_completed_subtasks;
                self.show_focus_banner = *focus_banner;
                if self.show_focus_banner {
                    self.focus_banner_until = None;
                }
                self.show_toolbar_labels = *toolbar_labels;
                self.hide_empty_views = *hide_empty_views;
                self.notification_requested = false;
                if self.due_notifications && !notifications::permission().allowed() {
                    notifications::request_permission();
                    self.notification_requested = true;
                }
                notifications::refresh_permission();
                self.notified_revision = None;
                self.follow_system = *follow_system;
                self.traffic_lights_visible = *traffic_lights;
                self.plain_accent_light = *accent_light;
                self.plain_accent_dark = *accent_dark;
                self.theme = self.build_theme(*theme, *appearance);
                self.save_session();
            }
            modals::Modal::Location { lat, lon } => {
                let lat = lat
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Latitude must be a number".to_string())?;
                let lon = lon
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Longitude must be a number".to_string())?;
                self.execute(Command::UpdateSettings(SettingsPatch {
                    current_location: Some(Some(flowlist_core::model::Geo { lat, lon })),
                    ..Default::default()
                }));
            }
            modals::Modal::TagLocation { tag, lat, lon } => {
                let lat = lat
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Latitude must be a number".to_string())?;
                let lon = lon
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Longitude must be a number".to_string())?;
                if !(-90.0..=90.0).contains(&lat) {
                    return Err("Latitude must be between -90 and 90".to_string());
                }
                if !(-180.0..=180.0).contains(&lon) {
                    return Err("Longitude must be between -180 and 180".to_string());
                }
                self.execute(Command::UpdateTag {
                    id: tag.clone(),
                    patch: TagPatch {
                        location: Some(Some(flowlist_core::model::Geo { lat, lon })),
                        ..Default::default()
                    },
                });
            }
            modals::Modal::Help => {}
        }
        Ok(())
    }

    // --------------------------------------------------------------- keys

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = event.keystroke.clone();
        if keystroke.key == "f12"
            && let Some(path) = test_shot_path()
        {
            capture_window_png(window, &path);
            return;
        }

        let stroke = &event.keystroke;
        let mods = stroke.modifiers;

        // GPUI does not install an application menu, so macOS never gets a
        // Quit item to bind ⌘Q to. Handle the standard chord here instead.
        if mods.platform && !mods.shift && stroke.key == "q" {
            cx.quit();
            return;
        }

        if self.quick_open.is_some() {
            self.handle_quick_open_key(event, cx);
            return;
        }
        if mods.platform && stroke.key == "o" && self.modal.is_none() {
            if mods.shift {
                self.open_modal(
                    modals::Modal::Import {
                        text: text::TextBuffer::empty(),
                        merge: true,
                    },
                    cx,
                );
            } else {
                self.open_quick_open(window, cx);
            }
            return;
        }

        // The Carbon shortcut normally opens the floating panel before this
        // handler runs. Keeping the same chord here means the panel still opens
        // when global registration is unavailable.
        if mods.control && mods.alt && stroke.key == "space" {
            self.toggle_quick_capture(window, cx);
            return;
        }

        // A preview owns the keyboard while it is open: nothing should type
        // into the list behind it.
        if self.preview.is_some() {
            match stroke.key.as_str() {
                "escape" => self.close_preview(cx),
                "o" if !mods.platform && !mods.control && !mods.alt => {
                    self.open_preview_external(cx);
                }
                "right" | "down" | "pagedown" | "j" => self.step_preview_page(1, cx),
                "left" | "up" | "pageup" | "k" => self.step_preview_page(-1, cx),
                _ => {}
            }
            window.focus(&self.focus_handle);
            cx.notify();
            return;
        }

        if self.focus == FocusArea::Modal {
            if mods.platform {
                // Settings sections are one keystroke away. The digits do not
                // collide with the sheet's fields because a field would have
                // to be an ⌘-chord to see them first.
                let section = match stroke.key.as_str() {
                    "1" => Some(modals::SettingsSection::Appearance),
                    "2" => Some(modals::SettingsSection::Sidebar),
                    "3" => Some(modals::SettingsSection::Workflow),
                    "4" => Some(modals::SettingsSection::Reminders),
                    "9" => Some(modals::SettingsSection::Calendar),
                    "5" => Some(modals::SettingsSection::Nearby),
                    "6" => Some(modals::SettingsSection::Trash),
                    "7" => Some(modals::SettingsSection::Mobile),
                    "8" => Some(modals::SettingsSection::About),
                    _ => None,
                };
                if let (
                    Some(section),
                    Some(modals::Modal::Settings {
                        section: current, ..
                    }),
                ) = (section, &mut self.modal)
                {
                    *current = section;
                    self.modal_focus = section.fields().first().copied().unwrap_or(0);
                    self.settings_toggle_boxes.borrow_mut().clear();
                    cx.notify();
                    return;
                }
                if stroke.key == "w" {
                    self.close_modal(cx);
                    return;
                }
                if stroke.key == "t" {
                    if let Some(modals::Modal::Settings {
                        theme, appearance, ..
                    }) = &mut self.modal
                    {
                        if mods.shift {
                            *theme = theme.next();
                        } else if mods.alt {
                            *theme = theme.previous();
                        } else {
                            *appearance = appearance.toggled();
                        }
                    }
                    let preview = match &self.modal {
                        Some(modals::Modal::Settings {
                            theme, appearance, ..
                        }) => Some((*theme, *appearance)),
                        _ => None,
                    };
                    if let Some((kind, mode)) = preview {
                        // The shortcut previews just like the swatches do.
                        self.theme = self.preview_theme(kind, mode);
                    }
                    cx.notify();
                    return;
                }
            }
            // Plain's accent presets are one digit away while Appearance is
            // open; no field is visible there, so the digits are free.
            if !mods.platform
                && !mods.alt
                && !mods.control
                && !mods.shift
                && self.active_buffer_mut().is_none()
                && matches!(
                    self.modal,
                    Some(modals::Modal::Settings {
                        theme: theme::ThemeKind::Plain,
                        section: modals::SettingsSection::Appearance,
                        ..
                    })
                )
            {
                if let Ok(index) = stroke.key.parse::<usize>() {
                    if let Some(preset) = theme::PLAIN_ACCENTS.get(index.wrapping_sub(1)) {
                        let (light, dark) = (preset.light, preset.dark);
                        self.set_plain_accents(Some(light), Some(dark), cx);
                        cx.notify();
                        return;
                    }
                }
            }
            // `w` flips the window buttons while Appearance is open. No field
            // is visible in that section, so the letter cannot collide with
            // typing; it mirrors the digits that pick accent presets.
            if !mods.platform
                && !mods.alt
                && !mods.control
                && !mods.shift
                && stroke.key == "w"
                && self.active_buffer_mut().is_none()
                && matches!(
                    self.modal,
                    Some(modals::Modal::Settings {
                        section: modals::SettingsSection::Appearance,
                        ..
                    })
                )
            {
                if let Some(modals::Modal::Settings { traffic_lights, .. }) = &mut self.modal {
                    *traffic_lights = !*traffic_lights;
                    self.traffic_lights_visible = *traffic_lights;
                }
                cx.notify();
                return;
            }
            if let Some(field) = self.view_menu {
                let choices = self.view_choices(field);
                let count = choices.len();
                match stroke.key.as_str() {
                    "escape" => {
                        self.view_menu = None;
                        cx.notify();
                    }
                    "down" | "j" => {
                        if count > 0 {
                            self.view_menu_cursor = (self.view_menu_cursor + 1) % count;
                        }
                        cx.notify();
                    }
                    "up" | "k" => {
                        if count > 0 {
                            self.view_menu_cursor = (self.view_menu_cursor + count - 1) % count;
                        }
                        cx.notify();
                    }
                    "home" => {
                        self.view_menu_cursor = 0;
                        cx.notify();
                    }
                    "end" => {
                        self.view_menu_cursor = count.saturating_sub(1);
                        cx.notify();
                    }
                    "enter" | "return" => {
                        if let Some(choice) = choices.get(self.view_menu_cursor).cloned() {
                            self.choose_view_choice(choice, cx);
                        }
                    }
                    _ => {}
                }
                return;
            }
            let paste = if mods.platform && stroke.key == "v" {
                cx.read_from_clipboard().and_then(|item| item.text())
            } else {
                None
            };
            if mods.platform && stroke.key == "a" {
                if let Some(buffer) = self.active_buffer_mut() {
                    buffer.select_all();
                }
                cx.notify();
                return;
            }
            if mods.platform && stroke.key == "c" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                return;
            }
            if mods.platform && stroke.key == "x" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    buffer.delete_selection();
                }
                cx.notify();
                return;
            }
            // The series-delete sheet is a two-choice switch rather than a
            // text field, so Tab and the arrow keys flip the choice.
            if matches!(self.modal, Some(modals::Modal::DeleteTask { .. })) {
                match stroke.key.as_str() {
                    "tab" | "left" | "right" | "up" | "down" => {
                        if let Some(modals::Modal::DeleteTask { whole, .. }) = &mut self.modal {
                            *whole = !*whole;
                        }
                        cx.notify();
                        return;
                    }
                    "escape" => {
                        self.close_modal(cx);
                        return;
                    }
                    "enter" | "return" => {
                        self.commit_modal(cx);
                        return;
                    }
                    _ => {}
                }
            }
            // The one-off conversion warning is a plain confirm/cancel sheet.
            if matches!(self.modal, Some(modals::Modal::HideSubtasks { .. })) {
                match stroke.key.as_str() {
                    "escape" => {
                        self.close_modal(cx);
                        return;
                    }
                    "enter" | "return" => {
                        self.commit_modal(cx);
                        return;
                    }
                    _ => {}
                }
            }
            // The priority-scope sheet is a two-choice switch too.
            if matches!(self.modal, Some(modals::Modal::PriorityScope { .. })) {
                match stroke.key.as_str() {
                    "tab" | "left" | "right" | "up" | "down" => {
                        if let Some(modals::Modal::PriorityScope { all, .. }) = &mut self.modal {
                            *all = !*all;
                        }
                        cx.notify();
                        return;
                    }
                    "escape" => {
                        self.close_modal(cx);
                        return;
                    }
                    "enter" | "return" => {
                        self.commit_modal(cx);
                        return;
                    }
                    _ => {}
                }
            }
            // The uninstall sheet is also a two-choice switch.
            if matches!(self.modal, Some(modals::Modal::Uninstall { .. })) {
                match stroke.key.as_str() {
                    "tab" | "left" | "right" | "up" | "down" => {
                        if let Some(modals::Modal::Uninstall { purge }) = &mut self.modal {
                            *purge = !*purge;
                        }
                        cx.notify();
                        return;
                    }
                    "escape" => {
                        self.close_modal(cx);
                        return;
                    }
                    "enter" | "return" => {
                        self.commit_modal(cx);
                        return;
                    }
                    _ => {}
                }
            }
            if let Some((target, search_index)) = self.icon_picker_target() {
                match stroke.key.as_str() {
                    "escape" => {
                        self.close_icon_picker(cx);
                        return;
                    }
                    "enter" | "return" if !mods.platform => {
                        if let Some(spec) = self.first_icon_match(search_index) {
                            self.choose_icon(target, spec, cx);
                            return;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(buffer) = self.active_buffer_mut() {
                match handle_buffer_key(buffer, stroke, paste.as_deref()) {
                    BufferKey::Changed => {
                        cx.notify();
                        return;
                    }
                    BufferKey::Submit => {
                        self.commit_modal(cx);
                        return;
                    }
                    BufferKey::Cancel => {
                        self.close_modal(cx);
                        return;
                    }
                    BufferKey::Tab => {
                        // Settings only tabs through the fields in the section
                        // on screen; the other defaults stay hidden.
                        let indices: Vec<usize> = match &self.modal {
                            Some(modals::Modal::Settings { section, .. }) => {
                                section.fields().to_vec()
                            }
                            Some(modal) => (0..modal.field_count())
                                .filter(|index| modal.field(*index).is_some())
                                .collect(),
                            None => Vec::new(),
                        };
                        if !indices.is_empty() {
                            let at = indices
                                .iter()
                                .position(|index| *index == self.modal_focus)
                                .unwrap_or(0);
                            let next = if mods.shift {
                                (at + indices.len() - 1) % indices.len()
                            } else {
                                (at + 1) % indices.len()
                            };
                            self.modal_focus = indices[next];
                        }
                        cx.notify();
                        return;
                    }
                    BufferKey::Ignored => {}
                }
            }
            // Modals without text fields still need Escape and Return.
            if stroke.key == "escape" {
                self.close_modal(cx);
                return;
            }
            if stroke.key == "enter" || stroke.key == "return" {
                self.commit_modal(cx);
                return;
            }
            return;
        }

        // The date popover owns the keyboard while it is open. The calendar
        // moves by day, the clock face by hour or minute, and every key is
        // swallowed so typing cannot edit the list behind the popover.
        if self.date_picker.is_some() && !mods.platform {
            match self.date_picker_step {
                DatePickerStep::Calendar => match stroke.key.as_str() {
                    "escape" => self.close_date_picker(cx),
                    "left" => self.move_date_picker_cursor(-1),
                    "right" => self.move_date_picker_cursor(1),
                    "up" => self.move_date_picker_cursor(-7),
                    "down" => self.move_date_picker_cursor(7),
                    "pageup" => self.shift_date_picker_month(-1),
                    "pagedown" => self.shift_date_picker_month(1),
                    "t" => self.choose_date_picker_date(local_date(now()), cx),
                    "backspace" | "delete" => self.clear_date_picker(cx),
                    "a" => self.toggle_date_picker_time(cx),
                    "[" => self.step_date_picker_time(-15, cx),
                    "]" => self.step_date_picker_time(15, cx),
                    "{" => self.step_date_picker_time(-60, cx),
                    "}" => self.step_date_picker_time(60, cx),
                    "enter" | "return" => {
                        let date = self.date_picker_cursor;
                        self.choose_date_picker_date(date, cx);
                    }
                    _ => {}
                },
                DatePickerStep::Clock => match stroke.key.as_str() {
                    "escape" => self.date_picker_back_to_calendar(cx),
                    "left" | "down" => self.step_date_picker_clock(-1, cx),
                    "right" | "up" => self.step_date_picker_clock(1, cx),
                    "pageup" => self.step_date_picker_clock(5, cx),
                    "pagedown" => self.step_date_picker_clock(-5, cx),
                    "tab" => {
                        let unit = match self.date_picker_unit {
                            ClockUnit::Hour => ClockUnit::Minute,
                            ClockUnit::Minute => ClockUnit::Hour,
                        };
                        self.set_date_picker_unit(unit, cx);
                    }
                    "h" => self.set_date_picker_unit(ClockUnit::Hour, cx),
                    "m" => self.set_date_picker_unit(ClockUnit::Minute, cx),
                    "a" => self.set_date_picker_all_day(true, cx),
                    "[" => self.step_date_picker_time(-15, cx),
                    "]" => self.step_date_picker_time(15, cx),
                    "{" => self.step_date_picker_time(-60, cx),
                    "}" => self.step_date_picker_time(60, cx),
                    "backspace" | "delete" => self.clear_date_picker(cx),
                    "enter" | "return" => {
                        let date = self.date_picker_cursor;
                        self.apply_date_picker(date, cx);
                    }
                    _ => {}
                },
            }
            cx.notify();
            return;
        }

        if matches!(
            self.focus,
            FocusArea::Capture
                | FocusArea::Search
                | FocusArea::Field
                | FocusArea::TagInput
                | FocusArea::Checklist
        ) {
            let paste = if mods.platform && stroke.key == "v" {
                cx.read_from_clipboard().and_then(|item| item.text())
            } else {
                None
            };
            if mods.platform && stroke.key == "a" {
                if let Some(buffer) = self.active_buffer_mut() {
                    buffer.select_all();
                }
                cx.notify();
                return;
            }
            if mods.platform && stroke.key == "c" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                return;
            }
            if mods.platform && stroke.key == "x" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    buffer.delete_selection();
                }
                cx.notify();
                return;
            }
            // Project and tag fields offer path completions. When a list is
            // open, Up/Down pick a row and Tab accepts it; otherwise the keys
            // keep their normal text-editing meaning.
            let suggestions =
                if self.focus == FocusArea::Field && self.editing == Some(EditField::Project) {
                    self.project_suggestions()
                } else if self.focus == FocusArea::TagInput {
                    self.tag_suggestions()
                } else {
                    Vec::new()
                };
            if !suggestions.is_empty() {
                match stroke.key.as_str() {
                    "down" => {
                        self.suggestion_index = if self.suggestion_engaged {
                            (self.suggestion_index + 1) % suggestions.len()
                        } else {
                            0
                        };
                        self.suggestion_engaged = true;
                        cx.notify();
                        return;
                    }
                    "up" => {
                        self.suggestion_index = if self.suggestion_engaged {
                            (self.suggestion_index + suggestions.len() - 1) % suggestions.len()
                        } else {
                            suggestions.len() - 1
                        };
                        self.suggestion_engaged = true;
                        cx.notify();
                        return;
                    }
                    "tab"
                        if !mods.shift
                            && (self.focus == FocusArea::TagInput || self.suggestion_engaged) =>
                    {
                        let pick =
                            suggestions[self.suggestion_index.min(suggestions.len() - 1)].clone();
                        if self.focus == FocusArea::TagInput {
                            self.choose_tag_suggestion(pick, cx);
                        } else {
                            self.choose_project_suggestion(pick, cx);
                        }
                        return;
                    }
                    "enter" | "return"
                        if self.focus == FocusArea::Field && self.suggestion_engaged =>
                    {
                        let pick =
                            suggestions[self.suggestion_index.min(suggestions.len() - 1)].clone();
                        self.choose_project_suggestion(pick, cx);
                        return;
                    }
                    _ => {}
                }
            }
            if let Some(buffer) = self.active_buffer_mut() {
                match handle_buffer_key(buffer, stroke, paste.as_deref()) {
                    BufferKey::Changed => {
                        // Typing filters the project list again; Tab returns
                        // to moving between fields until an arrow is pressed.
                        if self.focus == FocusArea::Field
                            && self.editing == Some(EditField::Project)
                        {
                            self.suggestion_engaged = false;
                        }
                        cx.notify();
                        return;
                    }
                    BufferKey::Submit => {
                        self.submit_focus(cx);
                        return;
                    }
                    BufferKey::Cancel => {
                        self.cancel_focus(cx);
                        return;
                    }
                    BufferKey::Tab => {
                        self.tab_focus(mods.shift, true, cx);
                        return;
                    }
                    BufferKey::Ignored => {}
                }
            }
        }

        if self.vim_mode && self.handle_vim_key(stroke, cx) {
            window.focus(&self.focus_handle);
            return;
        }

        if mods.platform {
            match stroke.key.as_str() {
                "z" => {
                    if mods.shift {
                        self.redo(cx);
                    } else {
                        self.undo(cx);
                    }
                }
                "[" => self.step_view_history(-1, cx),
                "]" => self.step_view_history(1, cx),
                "n" => {
                    if mods.shift {
                        self.open_modal(modals::Modal::new_project(self), cx);
                    } else if mods.alt {
                        self.open_modal(modals::Modal::new_folder(self), cx);
                    } else if mods.control {
                        self.open_modal(modals::Modal::new_tag(self), cx);
                    } else if self.view.shows_capture() {
                        self.focus = FocusArea::Capture;
                        self.capture.clear();
                    } else {
                        self.focus = FocusArea::Search;
                        self.search.select_all();
                    }
                }
                "p" if mods.shift => {
                    self.open_modal(modals::Modal::new_perspective(self), cx);
                }
                "f" => {
                    // ⌘⇧F flips search between this view and the whole
                    // library; the scope chip in the content header mirrors it.
                    if mods.shift && self.view.searches_tasks() {
                        self.search_library = !self.search_library;
                        self.toast = Some(
                            if self.search_library {
                                "Searching the whole library"
                            } else {
                                "Searching this view"
                            }
                            .to_string(),
                        );
                    }
                    self.focus = FocusArea::Search;
                    self.search.select_all();
                }
                "i" => {
                    self.show_inspector = !self.show_inspector;
                    self.save_session();
                }
                "t" => {
                    if mods.shift || mods.alt {
                        // Step the family but rebuild through the saved Plain
                        // accent, so returning to Plain keeps its colour.
                        let kind = if mods.shift {
                            self.theme.kind().next()
                        } else {
                            self.theme.kind().previous()
                        };
                        self.theme = self.build_theme(kind, self.theme.appearance());
                        self.toast = Some(format!(
                            "{} · {}",
                            self.theme.kind().label(),
                            self.theme.appearance().label()
                        ));
                        self.save_session();
                    } else {
                        self.toggle_appearance();
                    }
                }
                "d" => self.flag_selected(cx),
                "a" if matches!(self.view, View::Builtin(BuiltinPerspective::Trash)) => {
                    self.select_all_trash(cx)
                }
                "backspace" | "delete" => {
                    if self.focus == FocusArea::Sidebar {
                        self.delete_sidebar_target(cx);
                    } else {
                        self.delete_selected(cx);
                    }
                }
                "c" => self.copy_selection(cx),
                "s" => {
                    self.save_session();
                    self.toast = Some("Session saved".to_string());
                }
                "l" => {
                    self.show_inspector = true;
                    self.checklist_editing = None;
                    self.checklist_buffer.clear();
                    self.focus = FocusArea::Checklist;
                    self.save_session();
                }
                "r" => {
                    if let Some(entity) = self.entity_for_view() {
                        let modal = modals::Modal::rename(self, entity);
                        self.open_modal(modal, cx);
                    } else {
                        self.toast = Some(
                            "Open a project, folder, tag or perspective to rename it".to_string(),
                        );
                    }
                }
                "," => self.open_modal(modals::Modal::settings(self), cx),
                key if matches!(
                    key,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
                ) =>
                {
                    if let Some(perspective) = perspective_for_digit(key) {
                        self.open_view(View::Builtin(perspective), cx);
                    }
                }
                "o" => self.open_modal(
                    modals::Modal::Import {
                        text: text::TextBuffer::empty(),
                        merge: true,
                    },
                    cx,
                ),
                "e" => {
                    if mods.shift {
                        let text = taskpaper::export(self.store.db());
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        self.toast = Some("TaskPaper export copied to clipboard".to_string());
                    }
                }
                "?" | "/" => self.open_modal(modals::Modal::Help, cx),
                "up" => self.move_selected(true, cx),
                "down" => self.move_selected(false, cx),
                _ => {}
            }
            cx.notify();
            return;
        }

        if self.focus == FocusArea::Sidebar {
            match stroke.key.as_str() {
                "up" => self.sidebar_move(-1, cx),
                "down" => self.sidebar_move(1, cx),
                "enter" => self.sidebar_open_current(cx),
                "escape" => {
                    self.focus = FocusArea::Content;
                    cx.notify();
                }
                _ => {}
            }
            window.focus(&self.focus_handle);
            return;
        }

        if self.focus == FocusArea::ChecklistList {
            match stroke.key.as_str() {
                "up" => self.checklist_move_cursor(-1, cx),
                "down" => self.checklist_move_cursor(1, cx),
                "space" => self.checklist_toggle(cx),
                "enter" => self.checklist_begin_edit(cx),
                "a" => {
                    self.checklist_editing = None;
                    self.checklist_buffer.clear();
                    self.focus = FocusArea::Checklist;
                    cx.notify();
                }
                "backspace" | "delete" => self.checklist_delete(cx),
                "escape" => {
                    self.checklist_cursor = None;
                    self.focus = FocusArea::Content;
                    cx.notify();
                }
                _ => {}
            }
            window.focus(&self.focus_handle);
            return;
        }

        match stroke.key.as_str() {
            "up" => self.move_selection(-1, mods.shift),
            "down" => self.move_selection(1, mods.shift),
            "home" => self.move_selection(-1000, mods.shift),
            "end" => self.move_selection(1000, mods.shift),
            "space" => self.complete_selected(true, cx),
            "enter" => {
                if let Some(id) = self.selected_one() {
                    let completed = self
                        .store
                        .db()
                        .tasks
                        .get(&id)
                        .is_some_and(|task| task.completed);
                    if completed {
                        self.toggle_complete(id, cx);
                    } else {
                        self.begin_edit(EditField::Title, cx);
                    }
                }
            }
            "escape" => {
                if self.sort_menu_open
                    || self.group_menu_open
                    || self.priority_menu_open
                    || self.notification_menu.is_some()
                    || self.repeat_unit_menu.is_some()
                    || self.sidebar_menu.is_some()
                {
                    self.sort_menu_open = false;
                    self.group_menu_open = false;
                    self.priority_menu_open = false;
                    self.notification_menu = None;
                    self.repeat_unit_menu = None;
                    self.sidebar_menu = None;
                    cx.notify();
                    return;
                }
                self.selected.clear();
                self.anchor = None;
                self.search.clear();
                self.focus = FocusArea::Content;
                self.save_session();
                cx.notify();
                return;
            }
            "tab" => {
                if mods.shift {
                    self.outdent_selected(cx);
                } else {
                    self.indent_selected(cx);
                }
            }
            "backspace" | "delete" => self.delete_selected(cx),
            "n" => {
                if self.view.shows_capture() {
                    self.focus = FocusArea::Capture;
                    self.capture.clear();
                } else {
                    self.focus = FocusArea::Search;
                    self.search.select_all();
                }
                cx.notify();
                return;
            }
            "/" => {
                self.focus = FocusArea::Search;
                self.search.select_all();
                cx.notify();
                return;
            }
            "!" => self.flag_selected(cx),
            "x" => {
                let completed = self.selected.iter().all(|id| {
                    self.store
                        .db()
                        .tasks
                        .get(id)
                        .is_some_and(|task| task.completed)
                });
                self.complete_selected(!completed, cx);
            }
            "i" => {
                self.show_inspector = !self.show_inspector;
                self.save_session();
                cx.notify();
                return;
            }
            "e" => self.begin_edit(EditField::Title, cx),
            "r" if matches!(self.view, View::Builtin(BuiltinPerspective::Trash)) => {
                self.restore_selected(cx)
            }
            "[" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_step_date(-1);
            }
            "]" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_step_date(1);
            }
            "{" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_shift_month(-1);
            }
            "}" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_shift_month(1);
            }
            "\\" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_date = None;
                self.forecast_past = false;
                self.forecast_future = false;
                self.selected.clear();
                self.anchor = None;
                self.save_session();
            }
            "-" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.nudge_forecast_height(-24.0, cx);
            }
            "=" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.nudge_forecast_height(24.0, cx);
            }
            "t" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.forecast_today();
            }
            key if matches!(
                key,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
            ) =>
            {
                if let Some(perspective) = perspective_for_digit(key) {
                    self.open_view(View::Builtin(perspective), cx);
                }
            }
            _ => {}
        }
        window.focus(&self.focus_handle);
        cx.notify();
    }

    // ----------------------------------------------------------------- vim

    fn handle_vim_key(&mut self, stroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        if !self.vim_mode {
            return false;
        }
        match self.vim.feed(&stroke.key, stroke.modifiers) {
            vim::VimInput::Ignored => false,
            vim::VimInput::Pending => {
                cx.notify();
                true
            }
            vim::VimInput::Action(action) => {
                self.perform_vim(action, cx);
                cx.notify();
                true
            }
        }
    }

    fn perform_vim(&mut self, action: vim::VimAction, cx: &mut Context<Self>) {
        use vim::VimAction as A;
        match action {
            A::Cancel => {
                // Escape closes an open dropdown first. Vim handling runs
                // before the plain key map, so without this the menus would
                // stay open while the selection cleared underneath them.
                self.vim.reset();
                if self.sort_menu_open
                    || self.group_menu_open
                    || self.priority_menu_open
                    || self.notification_menu.is_some()
                    || self.repeat_unit_menu.is_some()
                    || self.sidebar_menu.is_some()
                {
                    self.sort_menu_open = false;
                    self.group_menu_open = false;
                    self.priority_menu_open = false;
                    self.notification_menu = None;
                    self.repeat_unit_menu = None;
                    self.sidebar_menu = None;
                    return;
                }
                self.selected.clear();
                self.anchor = None;
                self.search.clear();
                self.checklist_cursor = None;
                self.focus = FocusArea::Content;
                self.save_session();
            }
            A::Help => self.open_modal(modals::Modal::Help, cx),
            A::Search => {
                self.focus = FocusArea::Search;
                self.search.select_all();
            }
            A::NextMatch => self.search_step(true, cx),
            A::PrevMatch => self.search_step(false, cx),
            A::Theme => self.toggle_appearance(),
            A::Inspector => {
                self.show_inspector = !self.show_inspector;
                if !self.show_inspector && self.focus == FocusArea::ChecklistList {
                    self.focus = FocusArea::Content;
                }
                self.save_session();
            }
            A::FocusSidebar => {
                self.sidebar_cursor = self
                    .sidebar_cursor
                    .clone()
                    .or_else(|| self.target_for_view(&self.view));
                self.focus = FocusArea::Sidebar;
                self.save_session();
            }
            A::FocusContent => {
                self.checklist_cursor = None;
                self.focus = FocusArea::Content;
            }
            A::FocusChecklist => self.focus_checklist_pane(cx),
            A::Move(delta) => match self.focus {
                FocusArea::Sidebar => self.sidebar_move(delta, cx),
                FocusArea::ChecklistList => self.checklist_move_cursor(delta, cx),
                _ => {
                    let extend = false;
                    self.move_selection(delta, extend);
                    cx.notify();
                }
            },
            A::HalfPage(delta) => match self.focus {
                FocusArea::Sidebar => self.sidebar_move(delta * 10, cx),
                FocusArea::ChecklistList => self.checklist_move_cursor(delta * 10, cx),
                _ => {
                    self.move_selection(delta * 10, false);
                    cx.notify();
                }
            },
            A::ReorderDown => match self.focus {
                FocusArea::ChecklistList => self.checklist_reorder(true, cx),
                _ => self.move_selected(false, cx),
            },
            A::ReorderUp => match self.focus {
                FocusArea::ChecklistList => self.checklist_reorder(false, cx),
                _ => self.move_selected(true, cx),
            },
            A::Top => self.vim_jump(0, true, cx),
            A::Bottom => self.vim_jump(usize::MAX, false, cx),
            A::Goto(line) => self.vim_jump(line.saturating_sub(1), true, cx),
            A::Delete => match self.focus {
                FocusArea::ChecklistList => self.checklist_delete(cx),
                FocusArea::Sidebar => self.delete_sidebar_target(cx),
                _ => self.delete_selected(cx),
            },
            A::Yank => self.copy_selection(cx),
            A::Paste => self.paste_clipboard(cx),
            A::Undo => self.undo(cx),
            A::Redo => self.redo(cx),
            A::Rename => match self.focus {
                FocusArea::Sidebar => self.rename_sidebar_target(cx),
                FocusArea::ChecklistList => self.checklist_begin_edit(cx),
                _ => self.begin_edit(EditField::Title, cx),
            },
            A::Edit => self.begin_edit(EditField::Title, cx),
            A::Location => self.edit_location(cx),
            A::NewBelow => self.create_adjacent_task(false, cx),
            A::NewAbove => self.create_adjacent_task(true, cx),
            A::Complete => match self.focus {
                FocusArea::ChecklistList => self.checklist_toggle(cx),
                _ => self.complete_selected(true, cx),
            },
            A::Flag => self.flag_selected(cx),
            A::Indent => match self.focus {
                FocusArea::Sidebar => {
                    if !self.sidebar_expand(true, cx) {
                        self.sidebar_open_current(cx);
                    }
                }
                _ => self.indent_selected(cx),
            },
            A::Outdent => match self.focus {
                FocusArea::Sidebar => {
                    if !self.sidebar_expand(false, cx) {
                        self.sidebar_parent_move(cx);
                    }
                }
                _ => self.outdent_selected(cx),
            },
            A::ToggleSubtasks => self.set_selected_subtasks(None, cx),
            A::ExpandSubtasks => self.set_selected_subtasks(Some(true), cx),
            A::CollapseSubtasks => self.set_selected_subtasks(Some(false), cx),
            A::Open => match self.focus {
                FocusArea::Sidebar => self.sidebar_open_current(cx),
                FocusArea::ChecklistList => self.checklist_begin_edit(cx),
                _ => self.open_selected(cx),
            },
        }
    }

    fn vim_jump(&mut self, index: usize, from_start: bool, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Sidebar => {
                let len = self.sidebar_targets().len();
                if len == 0 {
                    return;
                }
                let index = if index == usize::MAX {
                    len - 1
                } else {
                    index.min(len - 1)
                };
                self.sidebar_select(index, cx);
            }
            FocusArea::ChecklistList => {
                let len = self.checklist_ids().len();
                if len == 0 {
                    return;
                }
                let index = if index == usize::MAX {
                    len - 1
                } else {
                    index.min(len - 1)
                };
                self.checklist_cursor = Some(index);
                cx.notify();
            }
            _ => {
                let ids = self.visible_ids();
                if ids.is_empty() {
                    return;
                }
                let index = if index == usize::MAX {
                    ids.len() - 1
                } else if from_start {
                    index.min(ids.len() - 1)
                } else {
                    index.min(ids.len() - 1)
                };
                self.select_only(ids[index].clone());
                self.save_session();
                cx.notify();
            }
        }
    }

    fn search_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.search.is_empty() {
            self.toast = Some("Type / to search first".to_string());
            cx.notify();
            return;
        }
        if self.focus == FocusArea::Sidebar {
            self.focus = FocusArea::Content;
        }
        self.move_selection(if forward { 1 } else { -1 }, false);
        cx.notify();
    }

    /// `⇧L` edits coordinates: a tag's spot in the sidebar or current view,
    /// otherwise the library location used by Nearby.
    fn edit_location(&mut self, cx: &mut Context<Self>) {
        if self.focus == FocusArea::Sidebar {
            match self.sidebar_cursor.clone() {
                Some(sidebar::SidebarTarget::Tag(id)) => {
                    let modal = modals::Modal::tag_location(self, id);
                    self.open_modal(modal, cx);
                    return;
                }
                Some(_) => {
                    self.toast = Some("Locations belong to tags".to_string());
                    cx.notify();
                    return;
                }
                None => {}
            }
        }
        if let View::Tag(id) = self.view.clone() {
            let modal = modals::Modal::tag_location(self, id);
            self.open_modal(modal, cx);
            return;
        }
        let modal = modals::Modal::location(self);
        self.open_modal(modal, cx);
    }

    fn open_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            return;
        };
        let completed = self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.completed);
        if completed {
            self.toggle_complete(id, cx);
        } else {
            self.begin_edit(EditField::Title, cx);
        }
    }

    fn focus_checklist_pane(&mut self, cx: &mut Context<Self>) {
        if self.selected_one().is_none() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        self.show_inspector = true;
        let len = self.checklist_ids().len();
        self.checklist_cursor = if len == 0 {
            None
        } else {
            Some(self.checklist_cursor.unwrap_or(0).min(len - 1))
        };
        self.focus = FocusArea::ChecklistList;
        self.save_session();
        cx.notify();
    }

    fn checklist_ids(&self) -> Vec<ChecklistId> {
        let Some(id) = self.selected_one() else {
            return Vec::new();
        };
        self.store
            .db()
            .tasks
            .get(&id)
            .map(|task| task.checklist.iter().map(|item| item.id.clone()).collect())
            .unwrap_or_default()
    }

    fn checklist_current(&self) -> Option<(TaskId, ChecklistId, String)> {
        let task = self.selected_one()?;
        let index = self.checklist_cursor?;
        let item = self
            .store
            .db()
            .tasks
            .get(&task)?
            .checklist
            .get(index)?
            .clone();
        Some((task, item.id, item.title))
    }

    fn checklist_move_cursor(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.checklist_ids().len();
        if len == 0 {
            self.checklist_cursor = None;
        } else {
            let current = self.checklist_cursor.unwrap_or(0) as isize;
            self.checklist_cursor = Some((current + delta).clamp(0, len as isize - 1) as usize);
        }
        cx.notify();
    }

    fn checklist_toggle(&mut self, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        self.toggle_checklist_item(task, item, cx);
    }

    fn checklist_begin_edit(&mut self, cx: &mut Context<Self>) {
        let Some((_, item, title)) = self.checklist_current() else {
            return;
        };
        self.begin_checklist_edit(item, title, cx);
    }

    fn checklist_delete(&mut self, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        self.delete_checklist_item(task, item, cx);
        let len = self.checklist_ids().len();
        self.checklist_cursor = if len == 0 {
            None
        } else {
            Some(self.checklist_cursor.unwrap_or(0).min(len - 1))
        };
        cx.notify();
    }

    fn checklist_reorder(&mut self, down: bool, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        let Some(index) = self.checklist_cursor else {
            return;
        };
        let target = if down {
            index + 1
        } else {
            index.saturating_sub(1)
        };
        if target == index || target >= self.checklist_ids().len() {
            return;
        }
        self.move_checklist_item(task, item, target, cx);
        self.checklist_cursor = Some(target);
        cx.notify();
    }

    /// Make a new task belong to the view it was created from: a project view
    /// files into that project, a tag view adds the tag, the Flagged view
    /// flags it, a priority view sets that priority, and Forecast gives it
    /// today's due date.
    fn apply_view_defaults(&self, new_task: &mut NewTask) {
        match &self.view {
            View::Project(id) => {
                new_task.project = Some(id.clone());
                new_task.project_path = None;
            }
            View::Tag(id) => {
                if !new_task.tags.contains(id) {
                    new_task.tags.push(id.clone());
                }
            }
            View::Priority(priority) => {
                if new_task.priority.is_none() {
                    new_task.priority = Some(*priority);
                }
            }
            View::Builtin(BuiltinPerspective::Flagged) => {
                new_task.flagged = true;
            }
            View::Builtin(BuiltinPerspective::Forecast) => {
                if new_task.due.is_none() {
                    new_task.due = Some(flowlist_core::time::Stamp::date_only(local_date(now())));
                }
            }
            _ => {}
        }
    }

    /// Add a subtask under the selected task and put the cursor in its title,
    /// so creating one never depends on knowing the Tab shortcut.
    fn add_subtask_to_selected(&mut self, cx: &mut Context<Self>) {
        let Some(parent) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let mut new_task = NewTask::new("New task");
        new_task.parent = Some(parent.clone());
        new_task.position = Position::End;
        if let Some(outcome) = self.execute(Command::CreateTask(new_task)) {
            self.expanded.insert(format!("task:{parent}"));
            if let Some(created) = outcome.created.first() {
                if let Ok(id) = self.store.db().resolve_task(created) {
                    self.select_only(id.clone());
                    self.draft_task = Some(id);
                    self.begin_edit(EditField::Title, cx);
                    self.field.select_all();
                }
            }
        }
        self.save_session();
        cx.notify();
    }

    fn create_adjacent_task(&mut self, above: bool, cx: &mut Context<Self>) {
        let selected = self.selected_one();
        let mut new_task = NewTask::new("New task");
        self.apply_view_defaults(&mut new_task);
        if let Some(id) = &selected {
            if let Some(task) = self.store.db().tasks.get(id) {
                new_task.parent = task.parent.clone();
                if new_task.project.is_none() {
                    new_task.project = task.project.clone();
                }
                new_task.position = if above {
                    Position::Before(id.clone())
                } else {
                    Position::After(id.clone())
                };
            }
        } else {
            new_task.position = if above {
                Position::Start
            } else {
                Position::End
            };
        }
        if let Some(outcome) = self.execute(Command::CreateTask(new_task)) {
            if let Some(created) = outcome.created.first() {
                if let Ok(id) = self.store.db().resolve_task(created) {
                    self.select_only(id.clone());
                    self.draft_task = Some(id);
                    self.begin_edit(EditField::Title, cx);
                    self.field.select_all();
                }
            }
        }
        self.save_session();
        cx.notify();
    }

    fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.toast = Some("Clipboard is empty".to_string());
            cx.notify();
            return;
        };
        let lines: Vec<String> = text
            .lines()
            .map(|line| line.split('\t').next().unwrap_or(line).trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            self.toast = Some("Clipboard has no task text".to_string());
            cx.notify();
            return;
        }
        let mut created = 0;
        let mut failed = 0;
        for line in lines {
            match parse_quick(&line, now()) {
                Ok(entry) => {
                    let mut new_task = entry.into_new_task();
                    self.apply_view_defaults(&mut new_task);
                    self.execute(Command::CreateTask(new_task));
                    created += 1;
                }
                Err(_) => failed += 1,
            }
        }
        self.toast = Some(if failed == 0 {
            format!("Pasted {}", plural(created, "task", "tasks"))
        } else {
            format!(
                "Pasted {}, skipped {}",
                plural(created, "task", "tasks"),
                plural(failed, "line", "lines")
            )
        });
        self.save_session();
        cx.notify();
    }

    fn rename_sidebar_target(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.sidebar_cursor.clone() else {
            return;
        };
        let entity = match target {
            sidebar::SidebarTarget::Project(id) => modals::EntityRef::Project(id),
            sidebar::SidebarTarget::Folder(id) => modals::EntityRef::Folder(id),
            sidebar::SidebarTarget::Tag(id) => modals::EntityRef::Tag(id),
            // A saved perspective's pencil opens the full editor, which also
            // carries its name. ⌘R still gives the plain rename sheet.
            sidebar::SidebarTarget::Custom(id) => {
                if let Some(modal) = modals::Modal::perspective(self, &id) {
                    self.open_modal(modal, cx);
                }
                return;
            }
            sidebar::SidebarTarget::Builtin(_) | sidebar::SidebarTarget::Priority(_) => {
                self.toast = Some("This sidebar row cannot be renamed".to_string());
                cx.notify();
                return;
            }
        };
        let modal = modals::Modal::rename(self, entity);
        self.open_modal(modal, cx);
    }

    fn sidebar_open_current(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.sidebar_cursor.clone() else {
            return;
        };
        match target {
            sidebar::SidebarTarget::Folder(id) => self.toggle_folder_row(id, cx),
            target => self.sidebar_activate(target, cx),
        }
    }

    /// Move the caret in a sheet field. `extend` keeps the old anchor so a
    /// drag paints a selection; a plain click collapses it.
    fn set_modal_cursor(
        &mut self,
        index: usize,
        char_index: usize,
        extend: bool,
        click_count: usize,
        cx: &mut Context<Self>,
    ) {
        self.modal_focus = index;
        self.focus = FocusArea::Modal;
        if let Some(modal) = self.modal.as_mut()
            && let Some(buffer) = modal.field_mut(index)
        {
            if extend {
                buffer.set_cursor(char_index, true);
            } else if click_count >= 3 {
                buffer.select_all();
            } else if click_count == 2 {
                buffer.select_word_at(char_index);
            } else {
                buffer.set_cursor(char_index, false);
            }
        }
        cx.notify();
    }

    fn active_buffer_mut(&mut self) -> Option<&mut text::TextBuffer> {
        match self.focus {
            FocusArea::Capture => Some(&mut self.capture),
            FocusArea::Search => Some(&mut self.search),
            FocusArea::Field => Some(&mut self.field),
            FocusArea::TagInput => Some(&mut self.tag_buffer),
            FocusArea::Checklist => Some(&mut self.checklist_buffer),
            FocusArea::Modal => {
                let index = self.modal_focus;
                self.modal.as_mut().and_then(|modal| modal.field_mut(index))
            }
            FocusArea::Content => None,
            FocusArea::Sidebar | FocusArea::ChecklistList => None,
        }
    }

    fn submit_focus(&mut self, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Capture => self.commit_capture(cx),
            FocusArea::Search => {
                self.focus = FocusArea::Content;
                cx.notify();
            }
            FocusArea::Field => self.tab_focus(false, false, cx),
            FocusArea::TagInput => self.add_tag_from_buffer(cx),
            FocusArea::Checklist => self.commit_checklist(cx),
            FocusArea::Modal => self.commit_modal(cx),
            FocusArea::Content | FocusArea::Sidebar | FocusArea::ChecklistList => {}
        }
    }

    fn cancel_focus(&mut self, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Capture => self.capture.clear(),
            FocusArea::Search => self.search.clear(),
            FocusArea::Field => {
                self.editing = None;
                self.batch_editing = None;
                self.field_error = None;
                self.draft_task = None;
            }
            FocusArea::TagInput => self.tag_buffer.clear(),
            FocusArea::Checklist => {
                self.checklist_editing = None;
                self.checklist_buffer.clear();
            }
            FocusArea::Modal => {
                self.close_modal(cx);
                return;
            }
            FocusArea::Content => {}
            FocusArea::Sidebar => {
                self.sidebar_cursor = None;
            }
            FocusArea::ChecklistList => {
                self.checklist_cursor = None;
            }
        }
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn tab_focus(&mut self, backwards: bool, wrap: bool, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Field if self.batch_editing.is_some() => {
                let current = self.batch_editing.unwrap_or_default();
                let index = BatchField::ALL
                    .iter()
                    .position(|field| *field == current)
                    .unwrap_or(0);
                if !self.commit_batch_field(cx) {
                    return;
                }
                let next = if backwards {
                    (index + BatchField::ALL.len() - 1) % BatchField::ALL.len()
                } else {
                    (index + 1) % BatchField::ALL.len()
                };
                self.begin_batch_edit(BatchField::ALL[next], cx);
            }
            FocusArea::Field => {
                // Save first; only move on when validation accepted the value.
                // A failed field (empty title, bad date, project confirm)
                // keeps the editor open on the same field so the error stays
                // visible instead of being replaced by the next field.
                let current = self.editing;
                let next = current.map(|field| {
                    if backwards {
                        field.previous()
                    } else {
                        field.next()
                    }
                });
                if !self.commit_field(cx) {
                    return;
                }
                let Some(next) = next else {
                    return;
                };
                // Enter on the last field finishes editing. Tab keeps cycling
                // through the fields, so the keyboard never traps the focus.
                if !wrap && !backwards && current == Some(EditField::Repeat) {
                    // The first-time task stops being a draft only when the
                    // whole field session ends, not when its title is saved,
                    // so Delete/Complete stay out of the way while the user
                    // is still filling it in.
                    self.draft_task = None;
                    return;
                }
                self.begin_edit(next, cx);
            }
            FocusArea::TagInput => {
                self.focus = FocusArea::Content;
                cx.notify();
            }
            FocusArea::Capture => {
                self.commit_capture(cx);
                self.focus = FocusArea::Content;
                cx.notify();
            }
            _ => {}
        }
    }
}

pub(super) fn handle_buffer_key(
    buffer: &mut text::TextBuffer,
    stroke: &Keystroke,
    paste: Option<&str>,
) -> BufferKey {
    let mods = stroke.modifiers;
    if let Some(text) = paste {
        buffer.insert_str(text);
        return BufferKey::Changed;
    }
    if mods.platform || mods.control {
        return BufferKey::Ignored;
    }
    if mods.alt {
        match stroke.key.as_str() {
            "left" => {
                buffer.move_word_left(mods.shift);
                return BufferKey::Changed;
            }
            "right" => {
                buffer.move_word_right(mods.shift);
                return BufferKey::Changed;
            }
            _ => {}
        }
    }
    match stroke.key.as_str() {
        "left" => {
            buffer.move_left(mods.shift);
            BufferKey::Changed
        }
        "right" => {
            buffer.move_right(mods.shift);
            BufferKey::Changed
        }
        "up" => {
            buffer.home(mods.shift);
            BufferKey::Changed
        }
        "down" => {
            buffer.end(mods.shift);
            BufferKey::Changed
        }
        "home" => {
            buffer.home(mods.shift);
            BufferKey::Changed
        }
        "end" => {
            buffer.end(mods.shift);
            BufferKey::Changed
        }
        "backspace" => {
            buffer.backspace();
            BufferKey::Changed
        }
        "delete" => {
            buffer.delete_forward();
            BufferKey::Changed
        }
        "enter" | "return" => {
            if mods.shift {
                buffer.newline();
                BufferKey::Changed
            } else {
                BufferKey::Submit
            }
        }
        "escape" => BufferKey::Cancel,
        "tab" => BufferKey::Tab,
        _ => match stroke.key_char.as_deref() {
            Some(chars) if !chars.is_empty() => {
                buffer.insert_str(chars);
                BufferKey::Changed
            }
            _ => BufferKey::Ignored,
        },
    }
}

fn sid(value: impl Into<gpui::SharedString>) -> gpui::SharedString {
    value.into()
}

fn store_warning(store: &Store) -> Option<String> {
    store.warning().map(|warning| warning.to_string())
}

fn view_exists(db: &Db, view: &View) -> bool {
    match view {
        View::Builtin(_) => true,
        View::Project(id) => db.projects.contains_key(id),
        View::Folder(id) => db.folders.contains_key(id),
        View::Tag(id) => db.tags.contains_key(id),
        View::Custom(id) => db.perspectives.contains_key(id),
        View::Priority(_) => true,
    }
}

fn project_in_folder(db: &Db, project: &ProjectId, folder: &FolderId) -> bool {
    if db.project_is_in_folder(project, folder, true) {
        return true;
    }
    db.projects
        .get(project)
        .and_then(|candidate| candidate.folder.as_ref())
        == Some(folder)
}

/// Digits are stable shortcuts, independent of the sidebar's display order:
/// 1–9 walk the original perspectives and 0 is the newer Changed list.
fn perspective_for_digit(key: &str) -> Option<BuiltinPerspective> {
    match key {
        "1" => Some(BuiltinPerspective::Inbox),
        "2" => Some(BuiltinPerspective::Projects),
        "3" => Some(BuiltinPerspective::Forecast),
        "4" => Some(BuiltinPerspective::Flagged),
        "5" => Some(BuiltinPerspective::Review),
        "6" => Some(BuiltinPerspective::Tags),
        "7" => Some(BuiltinPerspective::Completed),
        "8" => Some(BuiltinPerspective::Nearby),
        "9" => Some(BuiltinPerspective::Trash),
        "0" => Some(BuiltinPerspective::Changed),
        _ => None,
    }
}

/// Rank completion candidates for a project or tag field: prefix matches come
/// before substring matches, an exact path is dropped, and at most six rows are
/// offered. Spaces around `/` are optional so `Work/Launch` finds `Work / Launch`.
fn rank_suggestions(corpus: Vec<String>, query: &str) -> Vec<String> {
    let query = query.trim().to_lowercase().replace(" / ", "/");
    if query.is_empty() {
        return Vec::new();
    }
    let mut prefix = Vec::new();
    let mut contains = Vec::new();
    for path in corpus {
        let normalized = path.to_lowercase().replace(" / ", "/");
        if normalized == query {
            continue;
        }
        if normalized.starts_with(&query) {
            prefix.push(path);
        } else if normalized.contains(&query) {
            contains.push(path);
        }
    }
    prefix.extend(contains);
    prefix.truncate(6);
    prefix
}

fn sort_slug(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Manual => "manual",
        SortBy::Due => "due",
        SortBy::Defer => "defer",
        SortBy::Planned => "planned",
        SortBy::Name => "name",
        SortBy::Priority => "priority",
        SortBy::Flagged => "flagged",
        SortBy::Estimate => "estimate",
        SortBy::Created => "created",
        SortBy::CompletedAt => "completed",
        SortBy::DueFlagged => "due_flagged",
        SortBy::Tag => "tag",
        SortBy::AddedDate => "added",
        SortBy::ChangedDate => "changed",
        SortBy::DroppedDate => "dropped",
        SortBy::NextReview => "review",
    }
}

fn sort_from_slug(value: &str) -> Option<SortBy> {
    match value {
        "manual" => Some(SortBy::Manual),
        "due" => Some(SortBy::Due),
        "defer" => Some(SortBy::Defer),
        "planned" => Some(SortBy::Planned),
        "name" => Some(SortBy::Name),
        "priority" => Some(SortBy::Priority),
        "flagged" => Some(SortBy::Flagged),
        "estimate" => Some(SortBy::Estimate),
        "created" => Some(SortBy::Created),
        "completed" => Some(SortBy::CompletedAt),
        "due_flagged" => Some(SortBy::DueFlagged),
        "tag" => Some(SortBy::Tag),
        "added" => Some(SortBy::AddedDate),
        "changed" => Some(SortBy::ChangedDate),
        "dropped" => Some(SortBy::DroppedDate),
        "review" => Some(SortBy::NextReview),
        _ => None,
    }
}

fn sort_label(sort: Option<SortBy>) -> &'static str {
    match sort {
        None => "Default",
        Some(SortBy::Manual) => "Manual",
        Some(SortBy::Due) => "Due",
        Some(SortBy::Defer) => "Defer",
        Some(SortBy::Planned) => "Planned",
        Some(SortBy::Name) => "Name",
        Some(SortBy::Priority) => "Priority",
        Some(SortBy::Flagged) => "Flagged",
        Some(SortBy::Estimate) => "Estimate",
        Some(SortBy::Created) => "Created",
        Some(SortBy::CompletedAt) => "Completed",
        Some(SortBy::DueFlagged) => "Due & Flagged",
        Some(SortBy::Tag) => "Tag",
        Some(SortBy::AddedDate) => "Added",
        Some(SortBy::ChangedDate) => "Changed",
        Some(SortBy::DroppedDate) => "Dropped",
        Some(SortBy::NextReview) => "Next Review",
    }
}

/// Most groups start open. Forecast's far horizon starts folded so a task
/// months away does not push the coming week down the list.
fn group_default_open(key: &str) -> bool {
    key != "forecast:next-month"
}

fn group_slug(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "none",
        GroupBy::Project => "project",
        GroupBy::Folder => "folder",
        GroupBy::Tag => "tag",
        GroupBy::DueDate => "due",
        GroupBy::DeferDate => "defer",
        GroupBy::PlannedDate => "planned",
        GroupBy::CompletedDate => "completed",
        GroupBy::DroppedDate => "dropped",
        GroupBy::AddedDate => "added",
        GroupBy::ChangedDate => "changed",
        GroupBy::NextReview => "review",
        GroupBy::TagsCombined => "tags",
        GroupBy::Flagged => "flagged",
        GroupBy::Estimate => "estimate",
    }
}

fn group_from_slug(value: &str) -> Option<GroupBy> {
    match value {
        "none" => Some(GroupBy::None),
        "project" => Some(GroupBy::Project),
        "folder" => Some(GroupBy::Folder),
        "tag" => Some(GroupBy::Tag),
        "due" => Some(GroupBy::DueDate),
        "defer" => Some(GroupBy::DeferDate),
        "planned" => Some(GroupBy::PlannedDate),
        "completed" => Some(GroupBy::CompletedDate),
        "dropped" => Some(GroupBy::DroppedDate),
        "added" => Some(GroupBy::AddedDate),
        "changed" => Some(GroupBy::ChangedDate),
        "review" => Some(GroupBy::NextReview),
        "tags" => Some(GroupBy::TagsCombined),
        "flagged" => Some(GroupBy::Flagged),
        "estimate" => Some(GroupBy::Estimate),
        _ => None,
    }
}

fn group_label(group: Option<GroupBy>) -> &'static str {
    match group {
        None => "Default",
        Some(GroupBy::None) => "None",
        Some(GroupBy::Project) => "Project",
        Some(GroupBy::Folder) => "Folder",
        Some(GroupBy::Tag) => "Tag",
        Some(GroupBy::DueDate) => "Due",
        Some(GroupBy::DeferDate) => "Defer",
        Some(GroupBy::PlannedDate) => "Planned",
        Some(GroupBy::CompletedDate) => "Completed",
        Some(GroupBy::DroppedDate) => "Dropped",
        Some(GroupBy::AddedDate) => "Added",
        Some(GroupBy::ChangedDate) => "Changed",
        Some(GroupBy::NextReview) => "Next Review",
        Some(GroupBy::TagsCombined) => "Tags (combined)",
        Some(GroupBy::Flagged) => "Flagged",
        Some(GroupBy::Estimate) => "Estimate",
    }
}

/// A Priority view always filters to its own priority, whatever the last
/// header filter was. Other views use the filter saved with that view.
fn effective_priority_filter(view: &View, saved: Option<Priority>) -> Option<Priority> {
    match view {
        View::Priority(priority) => Some(*priority),
        _ => saved,
    }
}

// ---------------------------------------------------------------- render

impl FlowlistApp {
    fn perspective_count(&self, perspective: BuiltinPerspective) -> usize {
        let db = self.store.db();
        let at = now();
        match perspective {
            BuiltinPerspective::Inbox => db
                .inbox_unscheduled_outline(false)
                .into_iter()
                .filter(|row| self.focus_allows_task(&row.task))
                .count(),
            BuiltinPerspective::Today => query::today(db, true, at)
                .into_iter()
                .flat_map(|group| group.rows)
                .filter(|row| self.focus_allows_task(&row.id))
                .count(),
            BuiltinPerspective::Projects => db
                .projects
                .values()
                .filter(|project| {
                    project.status.is_open() && self.focus_allows_project(&project.id)
                })
                .count(),
            BuiltinPerspective::Forecast => {
                let due_counts = db.settings.forecast_due_counts;
                if due_counts {
                    let today = local_date(at);
                    db.tasks
                        .values()
                        .filter(|task| {
                            !task.completed
                                && !task.is_trashed()
                                && self.focus_allows_task(&task.id)
                                && task.due.is_some_and(|stamp| stamp.local_date() <= today)
                        })
                        .count()
                } else {
                    let data = query::forecast::forecast(db, at, self.forecast_due_only);
                    data.all_ids()
                        .into_iter()
                        .filter(|id| self.focus_allows_task(id))
                        .collect::<BTreeSet<_>>()
                        .len()
                }
            }
            BuiltinPerspective::Flagged => db
                .tasks
                .values()
                .filter(|task| {
                    !task.completed
                        && !task.is_trashed()
                        && task.flagged
                        && self.focus_allows_task(&task.id)
                })
                .count(),
            BuiltinPerspective::Review => query::review_projects(db, at)
                .into_iter()
                .filter(|project| self.focus_allows_project(project))
                .count(),
            BuiltinPerspective::Tags => {
                let mut tags = BTreeSet::new();
                for task in db.tasks.values() {
                    if task.completed || task.is_trashed() || !self.focus_allows_task(&task.id) {
                        continue;
                    }
                    tags.extend(task.tags.iter().cloned());
                }
                tags.len()
            }
            BuiltinPerspective::Completed => {
                let days = db.settings.show_completed_days.max(1) as i64;
                let cutoff = at - chrono::Duration::days(days);
                db.tasks
                    .values()
                    .filter(|task| {
                        task.completed
                            && !task.is_trashed()
                            && task
                                .completed_at
                                .is_some_and(|completed| completed >= cutoff)
                            && self.focus_allows_task(&task.id)
                    })
                    .count()
            }
            BuiltinPerspective::Archive => query::archived_projects(db)
                .into_iter()
                .filter(|project| self.focus_allows_project(project))
                .count(),
            BuiltinPerspective::Nearby => query::nearby(db, at)
                .into_iter()
                .filter(|hit| self.focus_allows_task(&hit.task))
                .count(),
            BuiltinPerspective::Changed => {
                let days = db.settings.changed_days.max(1) as i64;
                let cutoff = at - chrono::Duration::days(days);
                db.tasks
                    .values()
                    .filter(|task| {
                        !task.completed
                            && !task.is_trashed()
                            && task.modified_at >= cutoff
                            && self.focus_allows_task(&task.id)
                    })
                    .count()
            }
            BuiltinPerspective::Attachments => db
                .tasks
                .values()
                .filter(|task| {
                    !task.completed
                        && !task.is_trashed()
                        && !task.attachments.is_empty()
                        && self.focus_allows_task(&task.id)
                })
                .count(),
            BuiltinPerspective::Fullscreen => db
                .tasks
                .values()
                .filter(|task| {
                    !task.completed
                        && !task.is_trashed()
                        && db.wants_fullscreen_reminder(&task.id)
                        && db.reminder_stamp(&task.id).is_some()
                        && self.focus_allows_task(&task.id)
                })
                .count(),
            BuiltinPerspective::Trash => db.trash_count(),
        }
    }

    /// Write the state the GUI smoke test waits on. The file is only touched
    /// when `FLOWLIST_TEST_STATE` names a path, and only when something
    /// observable changed.
    fn sync_test_state(&mut self) {
        let Some(path) = self.test_state_path.clone() else {
            return;
        };
        // The tick/count each sidebar project shows, keyed by name so the GUI
        // smoke test can assert the badge without reading pixels.
        let project_badges = {
            let db = self.store.db();
            let mut projects: Vec<_> = db.projects.values().collect();
            projects.sort_by_cached_key(|project| project.name.to_lowercase());
            projects
                .iter()
                .map(|project| {
                    let (_, total) = db.project_progress(&project.id);
                    let open = db.project_open_count(&project.id);
                    let badge =
                        match sidebar::project_badge(open, total, project.effective_kind()) {
                            sidebar::ProjectBadge::Count(count) => count.to_string(),
                            sidebar::ProjectBadge::Stalled => "stalled".to_string(),
                        };
                    (project.name.clone(), badge)
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let anchor = |bounds: Option<Bounds<gpui::Pixels>>| {
            bounds.map(|bounds| {
                [
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ]
            })
        };
        let accent = theme::hsla_hex(self.theme.accent);
        let field_selection = {
            let (start, end) = self.field.selection();
            [start, end]
        };
        let mut state = serde_json::json!({
            "view": self.view.key(),
            "focus_targets": self
                .focus_targets
                .iter()
                .map(FocusTarget::key)
                .collect::<Vec<_>>(),
            "sidebar_selection": self
                .sidebar_selection
                .iter()
                .map(FocusTarget::key)
                .collect::<Vec<_>>(),
            "last_focus_targets": self
                .last_focus_targets
                .iter()
                .map(FocusTarget::key)
                .collect::<Vec<_>>(),
            "focus_banner_enabled": self.show_focus_banner,
            "focus_banner_transient": self.focus_banner_until.is_some(),
            "toolbar_labels": self.show_toolbar_labels,
            "header_height": if self.show_toolbar_labels { 66.0 } else { 46.0 },
            "modal": self.modal.as_ref().map(|modal| modal.title()),
            "modal_message": self.modal_error.clone(),
            "date_error": self.date_error.as_ref().map(|(_, field, message)| {
                serde_json::json!({ "field": field.slug(), "message": message })
            }),
            "modal_name": self
                .modal
                .as_ref()
                .and_then(|modal| modal.field(0))
                .map(|buffer| buffer.text().to_string()),
            "modal_cursor": self
                .modal
                .as_ref()
                .and_then(|modal| modal.field(0))
                .map(|buffer| buffer.cursor()),
            "modal_selection": self
                .modal
                .as_ref()
                .and_then(|modal| modal.field(0))
                .map(|buffer| {
                    let (start, end) = buffer.selection();
                    [start, end]
                }),
            "modal_field": anchor(self.modal_field_anchor.get()),
            "modal_close": self.modal.as_ref().is_some_and(|modal| modal.shows_close()),
            "tip": self.hovered_tip.as_ref().map(|tip| tip.to_string()),
            "tip_muted": self.suppressed_tip.as_ref().map(|tip| tip.to_string()),
            "focus": format!("{:?}", self.focus),
            "field": self.editing.map(|field| format!("{field:?}")),
            "selected": self.selected.len(),
            "selected_ids": self.selected.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "draft": self.draft_task.is_some(),
            "suggestions": self.project_suggestions().len(),
            "window": self.window_size,
            "theme": self.theme.kind().slug(),
            "appearance": self.theme.appearance().slug(),
            "blur": self.blur_mode.map(|appearance| appearance.slug()),
            "priority_filter": self.priority_filter.map(|priority| priority.slug()),
            "date_picker": self.date_picker.map(DateField::slug),
            "date_picker_cursor": self.date_picker_cursor.to_string(),
            "date_picker_month": self.date_picker_month.format("%Y-%m").to_string(),
            "only_available": self.only_available,
            "follow_system": self.follow_system,
            "sort": self.sort.map(sort_slug),
            "group": self.group.map(group_slug),
            "sort_menu": self.sort_menu_open,
            "group_menu": self.group_menu_open,
            "priority_menu": self.priority_menu_open,
            "sort_anchor": self.sort_anchor.get().is_some(),
            "group_anchor": self.group_anchor.get().is_some(),
            "priority_anchor": self.priority_anchor.get().is_some(),
            "due_anchor": anchor(self.due_anchor.get()),
            "defer_anchor": anchor(self.defer_anchor.get()),
            "planned_anchor": anchor(self.planned_anchor.get()),
            "reminder_anchor": anchor(self.reminder_anchor.get()),
            "title_space": self.title_space.get(),
            "title_visible": self.title_visible.get(),
            "sidebar_menu": self.sidebar_menu.as_ref().map(|menu| menu.key()),
            "forecast_date": self.forecast_date.map(|date| date.to_string()),
            "forecast_past": self.forecast_past,
            "forecast_future": self.forecast_future,
            "trashed": self.store.db().trash_count(),
            "traffic_lights": match self.traffic_lights_applied {
                Some(true) => "visible",
                Some(false) => "hidden",
                None => "unknown",
            },
            "modal_theme": match &self.modal {
                Some(modals::Modal::Settings { theme, appearance, .. }) => {
                    Some(format!("{}/{}", theme.slug(), appearance.slug()))
                }
                _ => None,
            },
        });
        state["delete_series"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::DeleteTask { whole, .. }) => Some(*whole),
            _ => None,
        });
        // Inserted after the macro: the full field list is already at the
        // json! recursion limit, so new debug fields go in one at a time.
        state["project_badges"] = serde_json::json!(project_badges);
        state["toast"] = serde_json::json!(self.toast);
        state["update"] = serde_json::json!(match &self.update {
            update::UpdateState::Idle => "idle",
            update::UpdateState::Checking => "checking",
            update::UpdateState::UpToDate { .. } => "up-to-date",
            update::UpdateState::Available { .. } => "available",
            update::UpdateState::Installing => "installing",
            update::UpdateState::Ready { .. } => "ready",
            update::UpdateState::Failed(_) => "failed",
        });
        state["cli"] = match crate::cli_install::status() {
            crate::cli_install::Status::Installed { path, on_path } => serde_json::json!({
                "installed": true,
                "path": path,
                "on_path": on_path,
            }),
            crate::cli_install::Status::Missing => serde_json::json!({
                "installed": false,
                "path": null,
                "on_path": false,
            }),
        };
        state["app_icon"] = serde_json::json!(self.app_icon.slug());
        state["notifications"] = serde_json::json!({
            "enabled": self.due_notifications,
            "permission": self.notification_permission.label(),
            "scheduled": self.notification_scheduled,
            "requested": self.notification_requested,
            "error": notifications::last_error(),
            "supported": notifications::supported(),
        });
        state["fullscreen_reminders"] = serde_json::json!(self.fullscreen_reminders);
        state["pause_media_on_reminder"] = serde_json::json!(self.pause_media_on_reminder);
        state["reminder_open"] = serde_json::json!(self.reminder_open);
        state["inspector_scroll"] = serde_json::json!({
            "max": f32::from(self.inspector_scroll.max_offset().height),
            "offset": f32::from(self.inspector_scroll.offset().y),
            "viewport": f32::from(self.inspector_scroll.bounds().size.height),
        });
        state["last_reminder_backdrop"] = serde_json::json!(self.last_reminder_backdrop);
        state["reminder_seen"] = serde_json::json!(self.reminder_seen.len());
        state["preview"] = serde_json::json!(self.preview.as_ref().map(|preview| {
            serde_json::json!({
                "path": preview.path,
                "name": preview.name,
                "kind": match preview.kind {
                    preview::PreviewKind::Image => "image",
                    preview::PreviewKind::Pdf { .. } => "pdf",
                },
                "page": match preview.kind {
                    preview::PreviewKind::Pdf { at, .. } => Some(at + 1),
                    preview::PreviewKind::Image => None,
                },
            })
        }));
        state["modal_fields"] = serde_json::json!(match &self.modal {
            Some(modal) => (0..modal.field_count())
                .filter(|index| modal.field(*index).is_some())
                .map(|index| {
                    serde_json::json!({
                        "label": modal.field_label(index),
                        "text": modal.field(index).map(|buffer| buffer.text().to_string()),
                    })
                })
                .collect::<Vec<_>>(),
            None => Vec::new(),
        });
        state["archive_projects"] = serde_json::json!(
            query::archived_projects(self.store.db())
                .into_iter()
                .filter_map(|id| self.store.db().projects.get(&id).map(
                    |project| serde_json::json!({
                        "id": id.to_string(),
                        "name": project.name,
                        "status": project.status.slug(),
                    })
                ))
                .collect::<Vec<_>>()
        );
        state["archive_rows"] = serde_json::json!(self.archive_boxes.borrow().clone());
        state["archive_groups"] = serde_json::json!(
            self.archive_sections()
                .into_iter()
                .map(|(key, title, projects)| serde_json::json!({
                    "key": key,
                    "title": title,
                    "projects": projects.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>()
        );
        state["date_picker_open"] = serde_json::json!(self.date_picker.map(|field| match field {
            DateField::Defer => "defer",
            DateField::Planned => "planned",
            DateField::Due => "due",
            DateField::Reminder => "reminder",
            DateField::RuleDate { .. } => "rule-date",
        }));
        state["date_picker_time"] = serde_json::json!(
            self.date_picker_time
                .map(|time| time.format("%H:%M").to_string())
        );
        state["date_picker_step"] = serde_json::json!(match self.date_picker_step {
            DatePickerStep::Calendar => "calendar",
            DatePickerStep::Clock => "clock",
        });
        state["date_picker_unit"] = serde_json::json!(match self.date_picker_unit {
            ClockUnit::Hour => "hour",
            ClockUnit::Minute => "minute",
        });
        state["date_picker_boxes"] = serde_json::json!(self.date_picker_boxes.borrow().clone());
        state["accent"] = serde_json::json!(accent);
        state["settings_section"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Settings { section, .. }) => Some(section.slug()),
            _ => None,
        });
        state["theme_boxes"] = serde_json::json!(self.theme_boxes.borrow().clone());
        state["view_subtitle"] = serde_json::json!(self.view_subtitle());
        state["icon_picker"] = serde_json::json!({
            "open": self.icon_picker_target().is_some(),
            "first": self
                .icon_picker_target()
                .and_then(|(_, index)| self.first_icon_match(index))
                .map(|spec| spec.stored()),
        });
        state["view_menu"] = serde_json::json!(self.view_menu.map(|field| field.key()));
        state["view_boxes"] = serde_json::json!(self.view_boxes.borrow().clone());
        state["view_form"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Perspective { form }) => Some(serde_json::json!({
                "name": form.name.text(),
                "description": form.description.text(),
                "tab": match form.tab {
                    perspective::PerspectiveTab::Simple => "simple",
                    perspective::PerspectiveTab::Comprehensive => "comprehensive",
                },
                "icon": form.icon.as_ref().map(|icon| icon.stored()),
                "icon_open": form.icon_open,
                "pinned": form.pinned,
                "show_completed": form.show_completed,
                "group": form.group_label(),
                "sort": form.sort_label(),
                "filter": serde_json::to_value(form.filter()).ok(),
                "editing": form.editing,
                "add_menu": form.add_menu,
                "row_menu": form.row_menu,
                "rows": form
                    .rows(self.store.db())
                    .into_iter()
                    .map(|(id, parent, label, enabled, group)| serde_json::json!({
                        "id": id,
                        "parent": parent,
                        "label": label,
                        "enabled": enabled,
                        "group": group,
                    }))
                    .collect::<Vec<_>>(),
                "matches": query::run(
                    self.store.db(),
                    &form.filter(),
                    form.sort,
                    form.group,
                    form.show_completed,
                    now(),
                )
                .iter()
                .map(|group| group.rows.len() + group.projects.len())
                .sum::<usize>(),
            })),
            _ => None,
        });
        state["field_cursor"] = serde_json::json!(self.field.cursor());
        state["field_selection"] = serde_json::json!(field_selection);
        state["field_text"] = serde_json::json!(self.field.text());
        state["checklist_cursor"] = serde_json::json!(self.checklist_buffer.cursor());
        state["checklist_selection"] = {
            let (start, end) = self.checklist_buffer.selection();
            serde_json::json!([start, end])
        };
        state["checklist_text"] = serde_json::json!(self.checklist_buffer.text());
        state["subtask_add"] = serde_json::json!(anchor(self.checklist_add_anchor.get()));
        state["task_steps"] = serde_json::json!(self.selected_one().and_then(|id| {
            let db = self.store.db();
            let task = db.tasks.get(&id)?;
            Some(serde_json::json!({
                "sequential": task.sequential,
                "one_off": task.steps_one_off,
                "checklist": task
                    .checklist
                    .iter()
                    .map(|item| serde_json::json!({
                        "title": item.title,
                        "completed": item.completed,
                    }))
                    .collect::<Vec<_>>(),
                "children": task
                    .child_order
                    .iter()
                    .filter_map(|child_id| db.tasks.get(child_id))
                    .map(|child| serde_json::json!({
                        "id": child.id,
                        "title": child.title,
                        "completed": child.completed,
                        "blocked_by": flowlist_core::query::availability::sequence_blocker(db, &child.id)
                            .and_then(|id| db.tasks.get(&id))
                            .map(|task| task.title.clone()),
                    }))
                    .collect::<Vec<_>>(),
            }))
        }));
        state["field_editor"] = serde_json::json!(anchor(self.field_editor_anchor.get()));
        state["focus_banner"] = serde_json::json!(anchor(self.focus_banner_anchor.get()));
        state["focus_eye"] = serde_json::json!(anchor(self.focus_eye_anchor.get()));
        state["hide_past"] = serde_json::json!(self.hide_past);
        state["hide_past_anchor"] = serde_json::json!(anchor(self.hide_past_anchor.get()));
        state["forecast_calendar_open"] = serde_json::json!(self.forecast_calendar_open);
        state["forecast_planned"] = serde_json::json!(self.forecast_planned);
        state["forecast_count"] =
            serde_json::json!(self.perspective_count(BuiltinPerspective::Forecast));
        state["forecast_calendar_expanded"] = serde_json::json!(self.forecast_calendar_expanded);
        state["forecast_week"] = serde_json::json!(self.forecast_week.to_string());
        state["forecast_strip"] = serde_json::json!(self.forecast_strip_boxes.borrow().clone());
        state["forecast_strip_next"] =
            serde_json::json!(anchor(self.forecast_strip_next_anchor.get()));
        state["forecast_strip_expand"] =
            serde_json::json!(anchor(self.forecast_strip_expand_anchor.get()));
        state["forecast_calendar_collapse"] =
            serde_json::json!(anchor(self.forecast_calendar_collapse_anchor.get()));
        state["forecast_days"] = serde_json::json!(self.forecast_day_boxes.borrow().clone());
        state["forecast_chips"] = serde_json::json!(self.forecast_chip_boxes.borrow().clone());
        state["forecast_day"] = serde_json::json!(self.forecast_date.map(|date| date.to_string()));
        state["forecast_past"] = serde_json::json!(self.forecast_past);
        state["forecast_future"] = serde_json::json!(self.forecast_future);
        state["empty_note"] = serde_json::json!(self.empty_note.borrow().clone());
        state["collapsed"] =
            serde_json::json!(self.collapsed_groups.iter().cloned().collect::<Vec<_>>());
        state["can_go_back"] = serde_json::json!(self.can_go_back());
        state["can_go_forward"] = serde_json::json!(self.can_go_forward());
        state["history"] = serde_json::json!(self.view_history);
        state["history_index"] = serde_json::json!(self.view_history_index);
        state["history_rows"] = serde_json::json!(self.history_boxes.borrow().clone());
        state["settings_toggle_rows"] =
            serde_json::json!(self.settings_toggle_boxes.borrow().clone());
        state["sidebar_sections"] = serde_json::json!(self.sidebar_section_boxes.borrow().clone());
        state["sidebar_counts"] = serde_json::json!(self.sidebar_count_boxes.borrow().clone());
        state["sidebar_count_values"] =
            serde_json::json!(self.sidebar_count_values.borrow().clone());
        state["quick_open"] = serde_json::json!(self.quick_open.as_ref().map(|state| {
            serde_json::json!({
                "query": state.query.text(),
                "cursor": state.cursor,
                "results": self.quick_open_entries().len(),
            })
        }));
        state["sidebar_visible"] = serde_json::json!(
            self.sidebar_targets()
                .iter()
                .map(|target| target.key())
                .collect::<Vec<_>>()
        );
        state["expanded"] = serde_json::json!(self.expanded.iter().cloned().collect::<Vec<_>>());
        state["sidebar_settings"] = serde_json::json!(self.store.db().settings.sidebar);
        state["forecast_calendar"] = serde_json::json!(anchor(self.forecast_calendar_anchor.get()));
        state["forecast_toggle_anchor"] =
            serde_json::json!(anchor(self.forecast_toggle_anchor.get()));
        state["forecast_grip_anchor"] = serde_json::json!(anchor(self.forecast_grip_anchor.get()));
        state["forecast_calendar_height"] = serde_json::json!(self.forecast_calendar_height);
        state["modal_project_icon"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::NewProject { icon, .. }) => Some(icon.stored()),
            _ => None,
        });
        state["wallpaper_light"] = serde_json::json!(
            self.wallpaper_light
                .as_ref()
                .map(|path| path.display().to_string())
        );
        state["wallpaper_dark"] = serde_json::json!(
            self.wallpaper_dark
                .as_ref()
                .map(|path| path.display().to_string())
        );
        state["use_wallpaper"] = serde_json::json!(self.use_wallpaper);
        state["wallpaper_active"] = serde_json::json!(self.active_wallpaper().is_some());
        state["modal_wallpaper"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Settings {
                wallpaper_light,
                wallpaper_dark,
                wallpaper_separate,
                use_wallpaper,
                wallpaper_note,
                ..
            }) => {
                let active = if *wallpaper_separate {
                    match self.theme.appearance() {
                        Appearance::Light => wallpaper_light,
                        Appearance::Dark => wallpaper_dark,
                    }
                } else {
                    wallpaper_light
                };
                Some(serde_json::json!({
                    "path": active.as_ref().map(|path| path.display().to_string()),
                    "light": wallpaper_light.as_ref().map(|path| path.display().to_string()),
                    "dark": wallpaper_dark.as_ref().map(|path| path.display().to_string()),
                    "separate": wallpaper_separate,
                    "use": use_wallpaper,
                    "note": wallpaper_note,
                }))
            }
            _ => None,
        });
        state["reminder_wallpaper_light"] = serde_json::json!(
            self.reminder_wallpaper_light
                .as_ref()
                .map(|path| path.display().to_string())
        );
        state["reminder_wallpaper_dark"] = serde_json::json!(
            self.reminder_wallpaper_dark
                .as_ref()
                .map(|path| path.display().to_string())
        );
        state["reminder_wallpaper_separate"] = serde_json::json!(self.reminder_wallpaper_separate);
        state["modal_reminder_wallpaper"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Settings {
                reminder_wallpaper_light,
                reminder_wallpaper_dark,
                reminder_wallpaper_separate,
                reminder_use_wallpaper,
                reminder_wallpaper_note,
                ..
            }) => {
                let active = if *reminder_wallpaper_separate {
                    match self.theme.appearance() {
                        Appearance::Light => reminder_wallpaper_light,
                        Appearance::Dark => reminder_wallpaper_dark,
                    }
                } else {
                    reminder_wallpaper_light
                };
                Some(serde_json::json!({
                    "path": active.as_ref().map(|path| path.display().to_string()),
                    "light": reminder_wallpaper_light.as_ref().map(|path| path.display().to_string()),
                    "dark": reminder_wallpaper_dark.as_ref().map(|path| path.display().to_string()),
                    "separate": reminder_wallpaper_separate,
                    "use": reminder_use_wallpaper,
                    "note": reminder_wallpaper_note,
                }))
            }
            _ => None,
        });
        state["modal_sidebar"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Settings { sidebar, .. }) => Some(sidebar),
            _ => None,
        });
        state["settings_size"] = serde_json::json!(self.settings_size);
        state["perspective_size"] = serde_json::json!(self.perspective_size);
        state["rule_menu_scroll"] = serde_json::json!(f32::from(-self.rule_menu_scroll.offset().y));
        state["settings_scroll"] = serde_json::json!(f32::from(-self.modal_scroll.offset().y));
        state["settings_sidebar_group"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::Settings { sidebar_group, .. }) => {
                (*sidebar_group).map(|group| group.slug())
            }
            _ => None,
        });
        state["sidebar_rows"] = serde_json::json!(self.sidebar_row_boxes.borrow().clone());
        state["sidebar_menu_boxes"] = serde_json::json!(self.sidebar_menu_boxes.borrow().clone());
        state["sidebar_drag_target"] = serde_json::json!(self.sidebar_drag_target);
        state["content_drop_target"] = serde_json::json!(self.content_drop_target);
        state["sidebar_names"] = serde_json::json!(self.sidebar_name_boxes.borrow().clone());
        state["content_rows"] = serde_json::json!(
            self.content_rows
                .borrow()
                .iter()
                .map(|(id, depth, title, has_children)| serde_json::json!({
                    "id": id,
                    "depth": depth,
                    "title": title,
                    "has_children": has_children,
                }))
                .collect::<Vec<_>>()
        );
        state["content_groups"] = serde_json::json!(self.content_group_boxes.borrow().clone());
        state["content_row_boxes"] = serde_json::json!(self.content_row_boxes.borrow().clone());
        state["content_projects"] = serde_json::json!(self.content_project_boxes.borrow().clone());
        state["batch_boxes"] = serde_json::json!(self.batch_boxes.borrow().clone());
        state["notification_boxes"] = serde_json::json!(self.notification_boxes.borrow().clone());
        state["calendar_event_boxes"] =
            serde_json::json!(self.calendar_event_boxes.borrow().clone());
        state["calendar_chip_boxes"] = serde_json::json!(self.calendar_chip_boxes.borrow().clone());
        state["calendar_events"] = serde_json::json!(self.calendar_events);
        state["calendar_events_enabled"] = serde_json::json!(self.calendar_events_enabled);
        state["calendar_permission"] = serde_json::json!(self.calendar_permission.label());
        state["calendar_calendars"] = serde_json::json!(self.calendar_calendars);
        state["calendar_hidden"] = serde_json::json!(self.calendar_hidden);
        state["forecast_due_only"] = serde_json::json!(self.forecast_due_only);
        state["inspector_section_boxes"] =
            serde_json::json!(self.inspector_section_boxes.borrow().clone());
        state["inspector_sections"] = serde_json::json!(self.inspector_sections);
        state["inspector_tab"] = serde_json::json!(self.inspector_tab.slug());
        state["notification_menu"] =
            serde_json::json!(self.notification_menu.as_ref().map(|target| match target {
                NotificationTarget::Task(id) => format!("task:{id}"),
                NotificationTarget::Project(id) => format!("project:{id}"),
            }));
        state["repeat_unit_menu"] =
            serde_json::json!(self.repeat_unit_menu.as_ref().map(|id| id.to_string()));
        state["batch"] = serde_json::json!(if self.selected.len() > 1 {
            Some(serde_json::json!({
                "count": self.selected.len(),
                "editing": self.batch_editing.map(|field| field.slug()),
                "priority": self.batch_draft.priority.map(|priority| priority.map(|p| p.slug())),
                "flagged": self.batch_draft.flagged,
                "error": self.batch_draft.error,
            }))
        } else {
            None
        });
        state["task_disclosures"] = serde_json::json!(self.task_disclosures.borrow().clone());
        state["inspector_subtasks"] = serde_json::json!(self.inspector_boxes.borrow().clone());
        let text = state.to_string();
        if self.test_state_last.as_deref() == Some(text.as_str()) {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, &text);
        self.test_state_last = Some(text);
    }

    /// When the user asked Flowlist to follow macOS, the palette family stays
    /// put and only light/dark is taken from the window each frame.
    fn sync_system_appearance(&mut self, window: &Window) {
        if !self.follow_system {
            return;
        }
        let appearance = match window.appearance() {
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => {
                theme::Appearance::Dark
            }
            _ => theme::Appearance::Light,
        };
        if self.theme.appearance() != appearance {
            self.theme = self.build_theme(self.theme.kind(), appearance);
        }
    }

    fn maybe_sync(&mut self) {
        if self.last_sync.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_sync = Instant::now();
        if self.store.reload_if_changed().unwrap_or(false) {
            self.prune_selection();
        }
    }

    /// Put a small title under a header icon when the toolbar-label setting is
    /// on. Search deliberately never goes through this.
    fn with_header_label(&self, content: impl IntoElement, label: &'static str) -> AnyElement {
        if !self.show_toolbar_labels {
            return content.into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .child(content)
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(self.theme.text_faint)
                    .child(label),
            )
            .into_any_element()
    }

    /// A title-bar arrow that steps through the views the window visited.
    /// It stops the header's window-drag handler so a click never starts a
    /// window move, and it measures itself for the GUI probe.
    fn history_button(
        &self,
        key: &'static str,
        icon: Icon,
        enabled: bool,
        tip: &'static str,
        forward: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let boxes = self.history_boxes.clone();
        let box_key = key.to_string();
        div()
            .id(key)
            .opacity(if enabled { 1.0 } else { 0.35 })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .flow_tip(self, cx, theme, tip)
            .on_click(cx.listener(move |this, _, _, cx| {
                if enabled {
                    this.step_view_history(if forward { 1 } else { -1 }, cx);
                }
            }))
            .child(
                self.with_header_label(
                    div()
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(bounds) = bounds.first() {
                                boxes.borrow_mut().insert(
                                    box_key.clone(),
                                    [
                                        f32::from(bounds.origin.x),
                                        f32::from(bounds.origin.y),
                                        f32::from(bounds.size.width),
                                        f32::from(bounds.size.height),
                                    ],
                                );
                            }
                        })
                        .child(widgets::icon_button(theme, icon, false)),
                    if forward { "Forward" } else { "Back" },
                ),
            )
    }

    /// The eye beside the back/forward arrows. It ends focus, or restores the
    /// last focus when none is active. It glows while focused.
    fn focus_eye_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let active = !self.focus_targets.is_empty();
        let can_restore = !self.last_focus_targets.is_empty();
        let enabled = active || can_restore;
        let tip = if active {
            "End focus"
        } else if can_restore {
            "Restore previous focus"
        } else {
            "Focus a project or folder first — then this eye restores it"
        };
        let anchor = self.focus_eye_anchor.clone();
        div()
            .id("focus-eye")
            .ml(px(if self.show_toolbar_labels { 10. } else { 0. }))
            // Reserve the wider "Unfocus" slot even while the label says
            // "Focus", so switching focus never shifts the eye icon.
            .w(px(if self.show_toolbar_labels { 50. } else { 28. }))
            .flex()
            .justify_center()
            .opacity(if enabled { 1.0 } else { 0.35 })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .flow_tip(self, cx, theme, tip)
            .on_click(cx.listener(move |this, _, _, cx| {
                if enabled {
                    this.toggle_focus_eye(cx);
                } else {
                    this.toast = Some(
                        "Focus a project or folder first — right-click it or use “focus on” in Quick Open."
                            .to_string(),
                    );
                    cx.notify();
                }
            }))
            .child(
                self.with_header_label(
                    div()
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(bounds) = bounds.first() {
                                anchor.set(Some(*bounds));
                            }
                        })
                        .child(widgets::icon_button(theme, Icon::Eye, active)),
                    if active { "Unfocus" } else { "Focus" },
                ),
            )
            .into_any_element()
    }

    /// The full-width Focus banner below the title bar.
    fn render_focus_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let (background, text, border, hover) = focus_banner_palette(self.theme);
        let labels: Vec<String> = self
            .focus_targets
            .iter()
            .map(|target| target.label(self.store.db()))
            .collect();
        let label = format!("Focus: {}", labels.join(", "));
        let anchor = self.focus_banner_anchor.clone();
        let banner = div()
            .id("focus-banner")
            .w_full()
            .h(px(30.))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(border)
            .bg(background)
            .text_color(text)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text)
                    .child(label),
            )
            .child(
                div()
                    .id("focus-clear")
                    .size(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|style| style.bg(hover))
                    .child(icons::icon(Icon::Close, 11.0, text))
                    .on_click(cx.listener(|this, _, _, cx| this.clear_focus(cx))),
            );
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    anchor.set(Some(*bounds));
                }
            })
            .child(banner)
            .into_any_element()
    }

    fn render_header(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let window_width = self.window_size.map_or(1254.0, |(width, _)| width);
        let (search_left, search_width) = search_slot(window_width);
        let can_back = self.can_go_back();
        let can_forward = self.can_go_forward();
        let header_height = if self.show_toolbar_labels { 66. } else { 46. };
        div()
            .flex()
            .items_center()
            .justify_between()
            .relative()
            .h(px(header_height))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.glass)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_1()
                    // The native window buttons sit at x = 18..72 when they
                    // are shown, so the name and history have to start after
                    // them rather than underneath.
                    .pl(px(if self.traffic_lights_visible {
                        88.
                    } else {
                        16.
                    }))
                    .child(
                        div()
                            .cursor(CursorStyle::OpenHand)
                            .pb(px(if self.show_toolbar_labels { 18. } else { 0. }))
                            .on_mouse_down(
                                MouseButton::Left,
                                |event: &gpui::MouseDownEvent, window, _| {
                                    if event.click_count >= 2 {
                                        window.zoom_window();
                                    } else {
                                        window.start_window_move();
                                    }
                                },
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Flowlist"),
                            ),
                    )
                    .child(
                        div()
                            .ml_2()
                            .flex()
                            .items_center()
                            .gap(px(if self.show_toolbar_labels { 7. } else { 2. }))
                            .child(self.history_button(
                                "history-back",
                                Icon::ArrowLeft,
                                can_back,
                                "Back (⌘[)",
                                false,
                                cx,
                            ))
                            .child(self.history_button(
                                "history-forward",
                                Icon::ArrowRight,
                                can_forward,
                                "Forward (⌘])",
                                true,
                                cx,
                            )),
                    )
                    .child(self.focus_eye_button(cx)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(search_left))
                    .top_0()
                    .bottom_0()
                    .w(px(search_width))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id("search")
                            .relative()
                            .w_full()
                            .child(widgets::text_field_with_trailer(
                                theme,
                                &self.search,
                                if self.search_library {
                                    "Search library"
                                } else {
                                    "Search tasks"
                                },
                                self.focus == FocusArea::Search,
                                if self.search.is_empty() { 0.0 } else { 26.0 },
                            ))
                            .when(!self.search.is_empty(), |this| {
                                this.child(
                                    div()
                                        .id("search-clear")
                                        .absolute()
                                        .right(px(2.))
                                        .top(px(2.))
                                        .size(px(24.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .bg(if self.focus == FocusArea::Search {
                                            theme.raised
                                        } else {
                                            theme.field
                                        })
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .flow_tip(self, cx, theme, "Clear search")
                                        .child(icons::icon(Icon::Close, 12.0, theme.text_faint))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            cx.stop_propagation();
                                            this.search.clear();
                                            this.focus = FocusArea::Search;
                                            cx.notify();
                                        })),
                                )
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.focus = FocusArea::Search;
                                this.search.select_all();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(if self.show_toolbar_labels { 14. } else { 4. }))
                    .pr_3()
                    .child(
                        div()
                            .id("add-task")
                            .mr_1()
                            .flow_tip(self, cx, theme, "Add a task (⌃⌥Space)")
                            .child(if self.show_toolbar_labels {
                                self.with_header_label(
                                    widgets::icon_button(theme, Icon::Plus, false),
                                    "Add Task",
                                )
                            } else {
                                widgets::button_with_icon(theme, Icon::Plus, "Add task", true)
                                    .into_any_element()
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_quick_capture(window, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("undo")
                            .opacity(if self.store.can_undo() { 1.0 } else { 0.4 })
                            .flow_tip(self, cx, theme, "Undo (⌘Z)")
                            .child(self.with_header_label(
                                widgets::icon_button(theme, Icon::Undo, false),
                                "Undo",
                            ))
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    )
                    .child(
                        div()
                            .id("redo")
                            .opacity(if self.store.can_redo() { 1.0 } else { 0.4 })
                            .flow_tip(self, cx, theme, "Redo (⇧⌘Z)")
                            .child(self.with_header_label(
                                widgets::icon_button(theme, Icon::Redo, false),
                                "Redo",
                            ))
                            .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
                    )
                    .child(
                        div()
                            .id("help")
                            .flow_tip(self, cx, theme, "Keyboard and mouse help (?)")
                            .child(self.with_header_label(
                                widgets::icon_button(theme, Icon::Help, false),
                                "Help",
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_modal(modals::Modal::Help, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("settings")
                            .flow_tip(self, cx, theme, "Settings (⌘,)")
                            .child(self.with_header_label(
                                widgets::icon_button(theme, Icon::Settings, false),
                                "Settings",
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_modal(modals::Modal::settings(this), cx)
                            })),
                    )
                    .child({
                        let dark = self.theme.appearance() == theme::Appearance::Dark;
                        div()
                            .id("theme-mode")
                            .flow_tip(
                                self,
                                cx,
                                theme,
                                if dark {
                                    "Switch to light mode (⌘T)"
                                } else {
                                    "Switch to dark mode (⌘T)"
                                },
                            )
                            .child(self.with_header_label(
                                widgets::icon_button(
                                    theme,
                                    if dark { Icon::Sun } else { Icon::Moon },
                                    false,
                                ),
                                if dark { "Light" } else { "Dark" },
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_appearance();
                                cx.notify();
                            }))
                    })
                    .child(
                        div()
                            .id("inspector")
                            .flow_tip(self, cx, theme, "Toggle inspector (i)")
                            .child(self.with_header_label(
                                widgets::icon_button(theme, Icon::Info, self.show_inspector),
                                "Inspect",
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_inspector = !this.show_inspector;
                                this.save_session();
                                cx.notify();
                            })),
                    ),
            )
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let at = now();
        let open = db
            .tasks
            .values()
            .filter(|task| !task.completed && !task.is_trashed())
            .count();
        let available = db
            .tasks
            .values()
            .filter(|task| {
                !task.completed
                    && !task.is_trashed()
                    && query::availability::availability(db, &task.id, at).is_available()
            })
            .count();
        let selected = self.selected.len();
        let undo = self
            .store
            .undo_label()
            .unwrap_or_else(|| "No undo".to_string());
        let revision = self.store.revision();
        let root = flowlist_core::paths::display_path(&self.store.paths().root);
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.))
            .px_4()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.glass)
            .text_xs()
            .text_color(theme.text_faint)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(format!("{open} open"))
                    .child(format!("{available} available"))
                    .when(selected > 0, |this| {
                        this.child(
                            div()
                                .text_color(theme.accent)
                                .child(format!("{selected} selected")),
                        )
                    })
                    .child(
                        div()
                            .id("status-undo")
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text_dim))
                            .child(undo)
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .when(self.vim_mode, |this| {
                        this.child(
                            div()
                                .text_color(if self.vim.is_pending() {
                                    theme.accent
                                } else {
                                    theme.text_faint
                                })
                                .child(if self.vim.is_pending() {
                                    "vim …"
                                } else {
                                    "vim"
                                }),
                        )
                    })
                    // The revision and full library path stay available on
                    // hover instead of putting engineering detail in every
                    // frame of the status bar.
                    .child(
                        div()
                            .id("status-library")
                            .max_w(px(360.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .flow_tip(self, cx, theme, format!("{root} · revision {revision}"))
                            .child(root),
                    ),
            )
    }

    /// Whether a click muted this tooltip until the pointer leaves its button.
    pub(super) fn tip_suppressed(&self, label: &SharedString) -> bool {
        self.suppressed_tip.as_ref() == Some(label)
    }

    /// Track which button owns the pointer so a click can mute just that
    /// tooltip, and clear the mute when the pointer moves off the button.
    pub(super) fn note_tip_hover(
        &mut self,
        label: SharedString,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            if self.suppressed_tip.as_ref() != Some(&label) {
                self.suppressed_tip = None;
            }
            self.hovered_tip = Some(label);
        } else {
            if self.hovered_tip.as_ref() == Some(&label) {
                self.hovered_tip = None;
            }
            if self.suppressed_tip.as_ref() == Some(&label) {
                self.suppressed_tip = None;
                cx.notify();
            }
        }
    }

    /// A click quiets any tooltip on the button under the pointer. The mute
    /// lasts until the pointer leaves, so it resets naturally for next time.
    fn suppress_hover_tip(&mut self) {
        if let Some(label) = self.hovered_tip.clone() {
            self.suppressed_tip = Some(label);
        }
    }

    fn render_toast(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let message = self.toast.clone().unwrap_or_default();
        let fresh = self
            .toast_shown
            .as_ref()
            .is_none_or(|(shown, _)| shown != &message);
        if fresh {
            self.toast_shown = Some((message.clone(), Instant::now()));
            let expected = message.clone();
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(2600))
                    .await;
                this.update(cx, |this, cx| {
                    if this.toast.as_deref() == Some(expected.as_str()) {
                        this.toast = None;
                        this.toast_shown = None;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }
        let clear = cx.listener(|this, _, _, cx| {
            this.toast = None;
            this.toast_shown = None;
            cx.notify();
        });
        // Sit over the middle pane, not the window: a toast under the
        // inspector or the sidebar points at the wrong place. Unlike search,
        // the toast follows the visible pane, so closing the inspector
        // re-centres it instead of leaving it left of middle.
        let (band_left, band_width) = pane_band(
            self.window_size.map_or(1254.0, |(width, _)| width),
            self.show_inspector,
        );
        div()
            .absolute()
            .bottom(px(38.))
            .left(px(band_left))
            .w(px(band_width))
            .flex()
            .justify_center()
            .child(fade_in(
                "toast-fade",
                div()
                    .id("toast")
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .text_sm()
                    .text_color(theme.text)
                    .cursor_pointer()
                    .child(message)
                    .on_click(clear),
            ))
    }

    fn render_empty(&self, title: &str, detail: &str) -> impl IntoElement {
        let theme = self.theme;
        // Recorded for the GUI probe, which cannot read text off the screen.
        *self.empty_note.borrow_mut() = Some(title.to_string());
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .gap_3()
            .py_12()
            .child(
                // A soft halo keeps an empty view from reading as a missing
                // image while staying quieter than a full illustration.
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(76.))
                    .rounded_full()
                    .bg(theme.field)
                    .border_1()
                    .border_color(theme.border_soft)
                    .child(icons::icon(Icon::CircleDashed, 30.0, theme.text_faint)),
            )
            .child(
                div()
                    .text_base()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_dim)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .max_w(px(340.))
                    .text_center()
                    .text_sm()
                    .text_color(theme.text_faint)
                    .child(detail.to_string()),
            )
    }
}

impl Focusable for FlowlistApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FlowlistApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measured = window.bounds().size;
        let measured = (f32::from(measured.width), f32::from(measured.height));
        let changed = self.window_size != Some(measured);
        self.window_size = Some(measured);
        // Coalesce resize bursts into one session write, then remember the
        // final size for the next launch.
        if changed && !self.window_dirty {
            self.window_dirty = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(600))
                    .await;
                this.update(cx, |this, cx| {
                    this.window_dirty = false;
                    this.save_session();
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        self.maybe_sync();
        self.sync_system_appearance(window);
        if !self.focus_handle.is_focused(window) {
            window.focus(&self.focus_handle);
        }
        window.set_window_title("Flowlist");
        // The native buttons sit over the custom header by default. Retry
        // until AppKit has built the titlebar, then only touch them again when
        // Settings changes the preference.
        if self.traffic_lights_applied != Some(self.traffic_lights_visible)
            && blur::set_traffic_lights(window, self.traffic_lights_visible)
        {
            self.traffic_lights_applied = Some(self.traffic_lights_visible);
        }
        // Keep the window's vibrancy in step with the palette: light glass for
        // a light theme, dark for a dark one, whatever macOS itself is set to.
        if self.blur_mode != Some(self.theme.appearance()) {
            if blur::sync(window, self.theme.appearance()).is_some() {
                self.blur_mode = Some(self.theme.appearance());
            }
        }
        let theme = self.theme;

        let mut root = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(theme.clear)
            .text_color(theme.text)
            .font_family("SF Pro Text")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    // A new click starts a new mouse-up lifecycle.
                    this.sort_closed_on_up = false;
                    this.group_closed_on_up = false;
                    this.priority_closed_on_up = false;
                    this.suppress_hover_tip();
                    window.focus(&this.focus_handle);
                    cx.notify();
                }),
            );
        if let Some(path) = self.active_wallpaper() {
            root = root.child(
                div().absolute().size_full().overflow_hidden().child(
                    img(path)
                        .object_fit(ObjectFit::Cover)
                        .size_full()
                        .opacity(self.wallpaper_opacity),
                ),
            );
        }
        root = root.child(self.render_header(window, cx));
        if !self.focus_targets.is_empty()
            && (self.show_focus_banner || self.focus_banner_until.is_some())
        {
            root = root.child(self.render_focus_banner(cx));
        }
        let mut body = div().flex().flex_1().min_h_0();
        body = body.child(self.render_sidebar(window, cx));
        body = body.child(self.render_content(window, cx));
        if self.show_inspector {
            body = body.child(self.render_inspector(window, cx));
        }
        root = root.child(body);
        root = root.child(self.render_status(window, cx));
        // Any open dropdown gets a full-window scrim, so a click anywhere
        // closes it instead of leaving an orphan menu behind.
        if self.sort_menu_open
            || self.group_menu_open
            || self.priority_menu_open
            || self.notification_menu.is_some()
            || self.repeat_unit_menu.is_some()
            || self.sidebar_menu.is_some()
            || self.date_picker.is_some()
        {
            root = root.child(
                div()
                    .id("menu-scrim")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    // Keep hover styles and clicks on the rows behind the
                    // scrim from firing while a dropdown is open.
                    .occlude()
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.sort_closed_on_up = this.sort_menu_open;
                            this.group_closed_on_up = this.group_menu_open;
                            this.priority_closed_on_up = this.priority_menu_open;
                            this.sort_menu_open = false;
                            this.group_menu_open = false;
                            this.priority_menu_open = false;
                            this.notification_menu = None;
                            this.repeat_unit_menu = None;
                            this.sidebar_menu = None;
                            this.date_picker = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, _, cx| {
                            this.sort_menu_open = false;
                            this.group_menu_open = false;
                            this.priority_menu_open = false;
                            this.notification_menu = None;
                            this.repeat_unit_menu = None;
                            this.sidebar_menu = None;
                            this.date_picker = None;
                            cx.notify();
                        }),
                    ),
            );
        }
        if self.sidebar_menu.is_some() {
            root = root.child(self.render_sidebar_menu(cx));
        }
        // Header dropdowns are root overlays positioned from the chip bounds
        // measured on the previous frame, so their hitboxes are exactly where
        // they paint.
        let viewport = self.window_size.unwrap_or((1254.0, 1404.0));
        if self.sort_menu_open
            && let Some(anchor) = self.sort_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 160.0, 250.0);
            root = root.child(fade_in(
                "sort-menu-fade",
                div()
                    .id("sort-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_sort_menu(cx)),
            ));
        }
        if self.group_menu_open
            && let Some(anchor) = self.group_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 160.0, 250.0);
            root = root.child(fade_in(
                "group-menu-fade",
                div()
                    .id("group-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_group_menu(cx)),
            ));
        }
        if self.priority_menu_open
            && let Some(anchor) = self.priority_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 150.0, 160.0);
            root = root.child(fade_in(
                "priority-menu-fade",
                div()
                    .id("priority-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_priority_menu(cx)),
            ));
        }
        if let Some(target) = self.notification_menu.clone()
            && let Some(anchor) = self.notification_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 210.0, 280.0);
            root = root.child(fade_in(
                "notification-menu-fade",
                div()
                    .id("notification-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_notification_menu(&target, cx)),
            ));
        }
        if let Some(id) = self.repeat_unit_menu.clone()
            && let Some(anchor) = self.repeat_unit_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 130.0, 190.0);
            root = root.child(fade_in(
                "repeat-unit-menu-fade",
                div()
                    .id("repeat-unit-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_repeat_unit_menu(&id, cx)),
            ));
        }
        if self.toast.is_some() {
            root = root.child(self.render_toast(cx));
        }
        if self.preview.is_some() {
            root = root.child(self.render_preview(window, cx));
        }
        if self.modal.is_some() {
            root = root.child(self.render_modal(window, cx));
        }
        if self.quick_open.is_some() {
            root = root.child(self.render_quick_open(window, cx));
        }
        if let Some(layer) = self.render_view_menu_layer(cx) {
            root = root.child(layer);
        }
        if let Some(field) = self.date_picker
            && let Some(anchor) = match field {
                DateField::RuleDate { rule, second } => self
                    .rule_date_anchors
                    .borrow()
                    .get(&format!("{rule}:{second}"))
                    .copied(),
                _ => self.date_anchor(field).get(),
            }
        {
            let (x, y) =
                date_picker_position(anchor, viewport, DATE_PICKER_WIDTH, DATE_PICKER_HEIGHT);
            root = root.child(fade_in(
                "date-picker-fade",
                div()
                    .id("date-picker-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_date_picker(cx)),
            ));
        }
        // Written after the body renders, so measurements taken during this
        // frame (such as the title slot width) are current in the file.
        self.sync_test_state();
        root
    }
}

static TEST_SHOT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(crate) fn test_shot_path() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("FLOWLIST_TEST_SHOT_DIR")?;
    let index = TEST_SHOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Some(std::path::PathBuf::from(dir).join(format!("shot-{index:02}.png")))
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CgPoint {
    x: f64,
    y: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CgSize {
    width: f64,
    height: f64,
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CgRect {
    origin: CgPoint,
    size: CgSize,
}

#[cfg(target_os = "macos")]
pub(crate) fn capture_window_png(window: &Window, path: &std::path::Path) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::RawWindowHandle;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_void};

    type CreateImage = unsafe extern "C" fn(CgRect, u32, u32, u32) -> *mut Object;

    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    let view = appkit.ns_view.as_ptr().cast::<Object>();
    unsafe {
        let native: *mut Object = msg_send![view, window];
        if native.is_null() {
            return;
        }
        // The app can capture its own window without the Screen Recording
        // permission. `CGWindowListCreateImage` is deprecated at compile time
        // but still present at runtime, so resolve it dynamically.
        let window_number: isize = msg_send![native, windowNumber];
        let frame = CgRect {
            origin: CgPoint {
                x: f64::INFINITY,
                y: f64::INFINITY,
            },
            size: CgSize {
                width: 0.0,
                height: 0.0,
            },
        };
        let name = CString::new("CGWindowListCreateImage").unwrap();
        let symbol = dlsym(-2isize as *mut c_void, name.as_ptr() as *const c_char);
        if symbol.is_null() {
            return;
        }
        let create_image: CreateImage = std::mem::transmute(symbol);
        let image = create_image(frame, 8, window_number as u32, 1);
        if image.is_null() {
            return;
        }
        let rep: *mut Object = msg_send![class!(NSBitmapImageRep), alloc];
        let rep: *mut Object = msg_send![rep, initWithCGImage: image];
        if rep.is_null() {
            return;
        }
        let data: *mut Object = msg_send![
            rep,
            representationUsingType: 4usize
            properties: std::ptr::null::<Object>()
        ];
        if data.is_null() {
            return;
        }
        let bytes: *const u8 = msg_send![data, bytes];
        let length: usize = msg_send![data, length];
        if bytes.is_null() || length == 0 {
            return;
        }
        let slice = std::slice::from_raw_parts(bytes, length);
        let _ = std::fs::write(path, slice);
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn capture_window_png(_window: &Window, _path: &std::path::Path) {}

/// Raise a window above the window manager for screenshot probes./// Raise a window above the window manager for screenshot probes. Normal
/// launches never call this; the main window keeps its native level and
/// corners. The quick capture panel has its own version of this.
#[cfg(target_os = "macos")]
fn raise_for_capture(window: &Window) {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::RawWindowHandle;

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    let view = appkit.ns_view.as_ptr().cast::<Object>();
    unsafe {
        let native: *mut Object = msg_send![view, window];
        if native.is_null() {
            return;
        }
        // NSFloatingWindowLevel. Windows on a lower level are left to the
        // system window manager; this one is not tiled.
        let _: () = msg_send![native, setLevel: 3i64];
        let _: () = msg_send![native, setHidesOnDeactivate: false];
    }
}

#[cfg(not(target_os = "macos"))]
fn raise_for_capture(_window: &Window) {}

/// OmniFocus-like opaque Focus banner colours. These are deliberately not the
/// theme's translucent selection or ink: the banner must stay readable and must
/// not let the wallpaper tint its right edge.
fn focus_banner_palette(theme: Theme) -> (gpui::Hsla, gpui::Hsla, gpui::Hsla, gpui::Hsla) {
    match theme.appearance() {
        Appearance::Light => (
            gpui::rgb(0xEFE7F7).into(),
            gpui::rgb(0x241B31).into(),
            gpui::rgb(0xD8CBE7).into(),
            gpui::rgb(0xE4D8F0).into(),
        ),
        Appearance::Dark => (
            gpui::rgb(0x332A42).into(),
            gpui::rgb(0xF5F0FA).into(),
            gpui::rgb(0x4B3E5E).into(),
            gpui::rgb(0x423656).into(),
        ),
    }
}

fn date_conflict_message(conflict: &DateConflict) -> String {
    let container = conflict
        .container
        .as_ref()
        .map(
            |(_, parent)| {
                if *parent { "parent action" } else { "project" }
            },
        )
        .unwrap_or("project or parent");
    let mut parts = Vec::new();
    if let Some((_, to)) = conflict.due {
        parts.push(format!(
            "Due date cannot be later than the {container}’s due date ({})",
            to.local_date().format("%a %b %-d")
        ));
    }
    if let Some((_, to)) = conflict.defer {
        parts.push(format!(
            "Defer date cannot be earlier than the {container}’s defer date ({})",
            to.local_date().format("%a %b %-d")
        ));
    }
    parts.join("; ")
}

pub fn run(paths: DataPaths) -> Result<()> {
    let store = Store::open(paths.clone())?;
    let saved: Option<session::Session> = store.read_session();
    drop(store);
    let restored = saved
        .and_then(|session| session.window)
        .map(|(width, height)| {
            size(
                px(width.clamp(880.0, 3200.0)),
                px(height.clamp(560.0, 2400.0)),
            )
        });
    let requested = restored.unwrap_or(size(px(1240.), px(800.)));
    // Screenshot and layout probes need a window the system window manager
    // leaves alone. Normal launches stay tiled; this is only set by
    // `FLOWLIST_TEST_FLOAT`.
    let test_float = std::env::var_os("FLOWLIST_TEST_FLOAT").is_some();
    let app = Application::new().with_assets(icons::Assets);
    // Closing the red button leaves the process alive with no window. Clicking
    // the Dock icon asks macOS to reopen; build the window again instead of
    // leaving a windowless app behind.
    let reopen_paths = paths.clone();
    app.on_reopen(move |cx| {
        if let Err(error) = open_main_window(cx, reopen_paths.clone(), requested, test_float) {
            eprintln!("could not reopen Flowlist: {error}");
        }
    });
    app.run(move |cx: &mut App| {
        cx.bind_keys([gpui::KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        cx.set_menus(vec![gpui::Menu {
            name: "Flowlist".into(),
            items: vec![gpui::MenuItem::action("Quit Flowlist", Quit)],
        }]);
        let window = match open_main_window(cx, paths.clone(), requested, test_float) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("could not open the Flowlist window: {error}");
                return;
            }
        };
        // A Carbon hotkey wakes the app even while another app is in
        // front. The callback only signals this foreground stream, which
        // opens (or re-focuses) the small floating capture panel.
        if let Some(mut presses) = hotkey::register() {
            use futures::StreamExt as _;
            let paths = paths.clone();
            cx.spawn(async move |cx| {
                while presses.next().await.is_some() {
                    let opened = cx.update(|cx| {
                        let theme = window
                            .read_with(cx, |app, _| app.theme)
                            .unwrap_or_else(|_| quick_capture::stored_theme(&paths));
                        quick_capture::toggle(cx, paths.clone(), theme, None)
                    });
                    match opened {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            eprintln!("flowlist: could not open quick capture: {error}");
                            let _ = window.update(cx, |app, window, cx| {
                                app.focus_capture(window, cx);
                            });
                        }
                        Err(_) => break,
                    }
                }
            })
            .detach();
        }
        cx.activate(true);
    });
    Ok(())
}

/// Open the main window. Kept separate from `run` so the same setup runs when
/// macOS reopens the app from the Dock after its last window was closed.
fn open_main_window(
    cx: &mut App,
    paths: DataPaths,
    requested: gpui::Size<gpui::Pixels>,
    test_float: bool,
) -> Result<gpui::WindowHandle<FlowlistApp>> {
    let bounds = Bounds::centered(None, requested, cx);
    let kind = if test_float {
        WindowKind::Floating
    } else {
        WindowKind::Normal
    };
    let window_paths = paths.clone();
    let window = cx.open_window(
        WindowOptions {
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Flowlist".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(18.), px(17.))),
            }),
            window_background: WindowBackgroundAppearance::Blurred,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(880.), px(560.))),
            kind,
            ..Default::default()
        },
        move |window, cx| {
            let store = match Store::open(window_paths.clone()) {
                Ok(store) => store,
                Err(error) => {
                    eprintln!("could not open the Flowlist library: {error}");
                    std::process::exit(1);
                }
            };
            let app = cx.new(|cx| FlowlistApp::new(store, cx));
            let handle = app.read(cx).focus_handle.clone();
            window.focus(&handle);
            if test_float {
                raise_for_capture(window);
                window.resize(requested);
                window.defer(cx, move |window, _| {
                    raise_for_capture(window);
                    window.resize(requested);
                });
            }
            app
        },
    )?;
    cx.activate(true);
    Ok(window)
}

/// The middle pane's horizontal band: after the sidebar, stopping before the
/// inspector. `reserve_inspector` keeps the inspector's column in the width
/// even when it is closed, which is what the title-bar search wants so the
/// field never moves. The toast wants the visible pane instead.
fn pane_band(window_width: f32, reserve_inspector: bool) -> (f32, f32) {
    let left = SIDEBAR_WIDTH;
    let reserved = if reserve_inspector {
        INSPECTOR_WIDTH
    } else {
        0.0
    };
    let width = (window_width - reserved - left).max(0.0);
    (left, width)
}

fn content_band(window_width: f32) -> (f32, f32) {
    pane_band(window_width, true)
}

/// The title-bar search sits centered on the content pane, not on the whole
/// window, and clears the action cluster at the right edge.
fn search_slot(window_width: f32) -> (f32, f32) {
    const FIELD_MAX: f32 = 420.0;
    const PANE_INSET: f32 = 16.0;

    let (pane_left, pane_width) = content_band(window_width);
    let width = FIELD_MAX.min((pane_width - 2.0 * PANE_INSET).max(0.0));
    let left = pane_left + (pane_width - width) / 2.0;
    (left, width)
}

#[cfg(test)]
mod tests {
    use super::rank_suggestions;
    use super::theme::Appearance;
    use super::{
        DATE_PICKER_HEIGHT, DATE_PICKER_WIDTH, DateField, View, accent_for, clock_dial_position,
        clock_hour_12, clock_hour_24, date_picker_position, effective_priority_filter,
        first_of_month, picker_default_time, picker_stamp, stepped_time,
    };
    use chrono::{NaiveDate, NaiveTime};
    use flowlist_core::model::{BuiltinPerspective, Priority};
    use gpui::{Bounds, point, px, size};

    fn corpus() -> Vec<String> {
        vec![
            "Work / Launch".to_string(),
            "Work / Launch follow-up".to_string(),
            "Home / Garden".to_string(),
            "Archive / Old launch".to_string(),
            "Errands".to_string(),
        ]
    }

    #[test]
    fn suggestions_prefer_prefix_then_substring() {
        let rows = rank_suggestions(corpus(), "work");
        assert_eq!(
            rows,
            vec![
                "Work / Launch".to_string(),
                "Work / Launch follow-up".to_string(),
            ]
        );

        let rows = rank_suggestions(corpus(), "launch");
        assert_eq!(rows[0], "Work / Launch");
        assert_eq!(rows[1], "Work / Launch follow-up");
        assert_eq!(rows[2], "Archive / Old launch");
    }

    #[test]
    fn suggestions_accept_paths_without_spaces_and_drop_exact_matches() {
        let rows = rank_suggestions(corpus(), "work/launch");
        // The exact path is not offered back as a completion.
        assert_eq!(rows, vec!["Work / Launch follow-up".to_string()]);
        assert!(rank_suggestions(corpus(), "  ").is_empty());
        assert_eq!(rank_suggestions(corpus(), "zzz").len(), 0);
    }

    #[test]
    fn suggestions_are_capped_at_six() {
        let many: Vec<String> = (0..10).map(|index| format!("Item {index}")).collect();
        assert_eq!(rank_suggestions(many, "item").len(), 6);
    }

    #[test]
    fn search_sits_centered_on_the_content_pane() {
        use super::search_slot;

        // Inspector open at the working size: 240..934, center 587.
        let (left, width) = search_slot(1254.0);
        assert_eq!(width, 420.0);
        assert!((left + width / 2.0 - 587.0).abs() < 0.01);

        // At the minimum window the pane narrows but the field still centers
        // on it and clears the action cluster at the right edge.
        let (left, width) = search_slot(880.0);
        assert_eq!(width, 288.0);
        assert!((left + width / 2.0 - 400.0).abs() < 0.01);
        assert!((left + width) <= 880.0 - 300.0 - 8.0 + 0.01);
    }

    #[test]
    fn search_reserves_the_inspector_and_the_toast_follows_the_visible_pane() {
        use super::{content_band, pane_band};

        // The search field always keeps the inspector's column, so it does not
        // move when the pane opens and closes.
        let (left, width) = content_band(1254.0);
        assert_eq!(left, 240.0);
        assert_eq!(width, 1254.0 - 240.0 - 320.0);
        assert!(left + width <= 1254.0 - 320.0);

        // The toast follows the pane actually on screen: with the inspector
        // closed it centres under the full middle pane.
        let (left, width) = pane_band(1254.0, false);
        assert_eq!(left, 240.0);
        assert_eq!(width, 1254.0 - 240.0);
        assert_eq!(pane_band(1254.0, true), (240.0, 1254.0 - 240.0 - 320.0));
        // A tiny window clamps to zero rather than painting outside.
        assert_eq!(pane_band(400.0, true), (240.0, 0.0));
    }

    #[test]
    fn a_priority_view_owns_its_filter() {
        // A Priority view always shows its own priority, even though the
        // header filter is remembered per view.
        assert_eq!(
            effective_priority_filter(&View::Priority(Priority::High), None),
            Some(Priority::High)
        );
        assert_eq!(
            effective_priority_filter(&View::Priority(Priority::Low), Some(Priority::Medium)),
            Some(Priority::Low)
        );
        // Other views get the filter saved with them, which is usually none.
        assert_eq!(
            effective_priority_filter(&View::Builtin(BuiltinPerspective::Inbox), None),
            None
        );
        assert_eq!(
            effective_priority_filter(
                &View::Builtin(BuiltinPerspective::Inbox),
                Some(Priority::Medium)
            ),
            Some(Priority::Medium)
        );
    }

    #[test]
    fn a_picked_day_carries_the_staged_time() {
        let date = NaiveDate::from_ymd_opt(2026, 12, 24).unwrap();
        let time = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        let timed = picker_stamp(date, Some(time));
        assert!(timed.has_time);
        assert_eq!(timed.local_date(), date);
        assert_eq!(timed.local_time(), time);

        // No staged time keeps the day all-day.
        let all_day = picker_stamp(date, None);
        assert!(!all_day.has_time);
        assert_eq!(all_day.local_date(), date);
    }

    #[test]
    fn the_clock_face_reads_and_writes_a_12_hour_day() {
        assert_eq!(clock_hour_12(0), 12);
        assert_eq!(clock_hour_12(1), 1);
        assert_eq!(clock_hour_12(11), 11);
        assert_eq!(clock_hour_12(12), 12);
        assert_eq!(clock_hour_12(13), 1);
        assert_eq!(clock_hour_12(23), 11);

        // Midnight and noon both read 12, but land in different halves.
        assert_eq!(clock_hour_24(12, false), 0);
        assert_eq!(clock_hour_24(12, true), 12);
        assert_eq!(clock_hour_24(1, false), 1);
        assert_eq!(clock_hour_24(1, true), 13);
        assert_eq!(clock_hour_24(11, true), 23);

        // Round-tripping every hour keeps the time of day.
        for hour in 0..24 {
            let pm = hour >= 12;
            assert_eq!(clock_hour_24(clock_hour_12(hour), pm), hour);
        }
    }

    #[test]
    fn the_clock_face_starts_at_12_and_runs_clockwise() {
        let (x, y) = clock_dial_position(0, 100.0, 50.0);
        assert!((x - 100.0).abs() < 0.01 && (y - 50.0).abs() < 0.01);
        // 3 o'clock sits a quarter turn to the right.
        let (x, y) = clock_dial_position(3, 100.0, 50.0);
        assert!((x - 150.0).abs() < 0.01 && (y - 100.0).abs() < 0.01);
        // 6 o'clock sits at the bottom.
        let (x, y) = clock_dial_position(6, 100.0, 50.0);
        assert!((x - 100.0).abs() < 0.01 && (y - 150.0).abs() < 0.01);
        // 9 o'clock sits a quarter turn to the left.
        let (x, y) = clock_dial_position(9, 100.0, 50.0);
        assert!((x - 50.0).abs() < 0.01 && (y - 100.0).abs() < 0.01);
    }

    #[test]
    fn all_day_dates_start_at_the_library_default() {
        let due_default = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        let defer_default = NaiveTime::from_hms_opt(8, 30, 0).unwrap();
        let fallback = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        assert_eq!(
            picker_default_time(DateField::Due, Some(due_default), None),
            due_default
        );
        assert_eq!(
            picker_default_time(DateField::Defer, None, Some(defer_default)),
            defer_default
        );
        // A field with no configured default starts at 9 AM, and a Defer date
        // never borrows the Due default.
        assert_eq!(picker_default_time(DateField::Due, None, None), fallback);
        assert_eq!(
            picker_default_time(DateField::Defer, Some(due_default), None),
            fallback
        );
    }

    #[test]
    fn time_steps_wrap_at_midnight() {
        let late = NaiveTime::from_hms_opt(23, 50, 0).unwrap();
        let early = NaiveTime::from_hms_opt(0, 5, 0).unwrap();
        assert_eq!(stepped_time(late, 15), early);
        assert_eq!(stepped_time(early, -15), late);
        assert_eq!(
            stepped_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap(), 60),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap()
        );
    }

    #[test]
    fn the_picker_opens_under_its_icon_and_stays_on_screen() {
        let anchor = Bounds {
            origin: point(px(1140.0), px(300.0)),
            size: size(px(28.0), px(28.0)),
        };
        // Right edge of the icon, then four pixels below it.
        let (x, y) = date_picker_position(
            anchor,
            (1254.0, 1000.0),
            DATE_PICKER_WIDTH,
            DATE_PICKER_HEIGHT,
        );
        assert_eq!(x, 1140.0 + 28.0 - DATE_PICKER_WIDTH);
        assert_eq!(y, 300.0 + 28.0 + 4.0);

        // A window too short for the card pulls it up to the last usable line.
        let (_, y) = date_picker_position(
            anchor,
            (1254.0, 500.0),
            DATE_PICKER_WIDTH,
            DATE_PICKER_HEIGHT,
        );
        assert_eq!(y, 500.0 - DATE_PICKER_HEIGHT);
        // And a picker near the left edge never gains a negative x.
        let left = Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(28.0), px(28.0)),
        };
        assert_eq!(
            date_picker_position(
                left,
                (1254.0, 1000.0),
                DATE_PICKER_WIDTH,
                DATE_PICKER_HEIGHT
            )
            .0,
            0.0
        );
    }

    #[test]
    fn month_navigation_starts_on_the_first() {
        let date = NaiveDate::from_ymd_opt(2026, 12, 24).unwrap();
        assert_eq!(
            first_of_month(date),
            NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
        );
    }
    #[test]
    fn an_accent_chosen_in_one_appearance_carries_to_the_other() {
        let red = 0xB42828;
        assert_eq!(accent_for(Appearance::Light, Some(red), None), Some(red));
        let carried = accent_for(Appearance::Dark, Some(red), None).expect("dark inherits");
        assert_ne!(carried, 0x181818, "must not fall back to ink");
        // An explicit tone for the other appearance always wins.
        assert_eq!(
            accent_for(Appearance::Dark, Some(red), Some(0x4C7DFF)),
            Some(0x4C7DFF)
        );
        // Clearing both sides really does return to ink.
        assert_eq!(accent_for(Appearance::Light, None, None), None);
    }
}
