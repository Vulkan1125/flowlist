//! The Flowlist window: state, commands, keyboard routing and layout.
//!
//! The GUI shares its entire domain layer with the CLI and MCP server. Every
//! edit goes through `Store::execute`, so undo, validation and cross-process
//! locking behave exactly the same everywhere.

mod blur;
mod content;
mod drag;
mod hotkey;
mod icons;
mod inspector;
mod modals;
mod perspective;
mod session;
mod sidebar;
mod text;
mod theme;
mod vim;
mod widgets;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{Datelike, Timelike};
use gpui::{
    App, Application, Bounds, ClipboardItem, Context, CursorStyle, FocusHandle, Focusable,
    IntoElement, KeyDownEvent, Keystroke, MouseButton, Render, ScrollHandle, SharedString, Window,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};

use flowlist_core::command::{
    Command, ContainerPlacement, NewProject, NewTask, Outcome, ProjectPatch, SettingsPatch,
    TagPatch, TaskPatch, TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Db, OrderKey, Position};
use flowlist_core::ids::{ChecklistId, ContainerId, FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{BuiltinPerspective, GroupBy, Priority, ProjectKind, SortBy, TagStatus};
use flowlist_core::parse::{parse_duration, parse_quick, parse_repeat, parse_stamp};
use flowlist_core::paths::DataPaths;
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, format_duration, local_date, now};
use flowlist_core::{export, taskpaper};

use drag::{DragProject, DragTask};
use icons::Icon;

pub use session::{Session, View};
use theme::Theme;
use widgets::TipExt;

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
    Due,
    Estimate,
    Repeat,
}

impl EditField {
    fn next(self) -> Self {
        match self {
            EditField::Title => EditField::Notes,
            EditField::Notes => EditField::Project,
            EditField::Project => EditField::Defer,
            EditField::Defer => EditField::Due,
            EditField::Due => EditField::Estimate,
            EditField::Estimate => EditField::Repeat,
            EditField::Repeat => EditField::Title,
        }
    }

    fn previous(self) -> Self {
        match self {
            EditField::Title => EditField::Repeat,
            EditField::Notes => EditField::Title,
            EditField::Project => EditField::Notes,
            EditField::Defer => EditField::Project,
            EditField::Due => EditField::Defer,
            EditField::Estimate => EditField::Due,
            EditField::Repeat => EditField::Estimate,
        }
    }
}

/// The two task dates that can be changed from the inspector's calendar
/// popover. Each has its own measured anchor so the popover opens under the
/// field it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DateField {
    Defer,
    Due,
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
            DateField::Due => EditField::Due,
        }
    }

    pub(super) fn slug(self) -> &'static str {
        match self {
            DateField::Defer => "defer",
            DateField::Due => "due",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            DateField::Defer => "Defer",
            DateField::Due => "Due",
        }
    }

    pub(super) fn placeholder(self) -> &'static str {
        match self {
            DateField::Defer => "No defer date",
            DateField::Due => "No due date",
        }
    }
}

enum BufferKey {
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
        DateField::Due => due.unwrap_or(fallback),
        DateField::Defer => defer.unwrap_or(fallback),
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

pub struct FlowlistApp {
    store: Store,
    theme: Theme,
    view: View,
    /// First day of the month the Forecast calendar shows.
    forecast_month: chrono::NaiveDate,
    /// A clicked calendar date; `None` shows the normal Forecast buckets.
    forecast_date: Option<chrono::NaiveDate>,
    /// The width the Forecast calendar painted on the last frame, so a GUI
    /// probe can tell fixed cells from a full-width pane.
    forecast_calendar_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The months and width chips, measured so the GUI probe can click them.
    forecast_months_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    forecast_width_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// Sidebar row boxes keyed by target, so a GUI probe can drag a project
    /// onto a folder without guessing at coordinates.
    sidebar_row_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Painted boxes for the start of each sidebar row's name (its icon), plus
    /// `empty:<parent>` for the "no projects" note, so a probe can check that
    /// nested rows are indented under their folder.
    sidebar_name_boxes: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// The sidebar row under the pointer while a project is being dragged, so
    /// that row can highlight and the test state can report the drop target.
    sidebar_drag_target: Option<String>,
    /// Task rows painted on the last frame, in order, with their depth. The
    /// list reflects which subtasks are visible, not just what the query
    /// matched.
    content_rows: Rc<RefCell<Vec<PaintedTaskRow>>>,
    /// Subtask disclosure chevrons, measured so the GUI probe can click them.
    task_disclosures: Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    /// Applies to every list, and is what the sidebar priority rows set.
    priority_filter: Option<Priority>,
    /// Hide everything blocked behind another task (First Available).
    only_available: bool,
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
    group_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    priority_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The first text field in the open sheet, measured so a GUI probe can
    /// click and drag inside it.
    modal_field_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    /// The Due and Defer field rows, measured so their calendar popover can
    /// paint under the right field.
    due_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
    defer_anchor: Rc<Cell<Option<Bounds<gpui::Pixels>>>>,
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
    /// How wide the title's slot painted on the last frame. A view name that
    /// does not fit there is left out entirely instead of showing a stub.
    title_space: Rc<Cell<Option<f32>>>,
    /// Whether the last frame painted the view title. Exposed through the
    /// test-state file so the layout rule can be checked without a screenshot.
    title_visible: Rc<Cell<bool>>,
    /// The right-click menu on a sidebar row, opened at the mouse position.
    sidebar_menu: Option<sidebar::SidebarMenu>,
    /// A click's mouse-up closed the sort or group menu. The chip under the
    /// same click must not immediately reopen it.
    sort_closed_on_up: bool,
    group_closed_on_up: bool,
    priority_closed_on_up: bool,
    /// The project inspector's icon grid, shown only on request.
    project_icon_open: bool,
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
    vim_mode: bool,
    vim: vim::VimState,
    sidebar_cursor: Option<sidebar::SidebarTarget>,
    /// The sidebar row under the pointer, if any. Hover-only row actions
    /// render only for this row so an idle row spends no width on them.
    sidebar_hover: Option<sidebar::SidebarTarget>,
    checklist_cursor: Option<usize>,
    focus: FocusArea,
    focus_handle: FocusHandle,
    capture: text::TextBuffer,
    search: text::TextBuffer,
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
    modal_focus: usize,
    modal_scroll: ScrollHandle,
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
    last_sync: Instant,
    /// Set by `FLOWLIST_TEST_STATE`; the window writes a small JSON state file
    /// for the GUI smoke test. Off in normal use.
    test_state_path: Option<PathBuf>,
    test_state_last: Option<String>,
    /// The appearance the window glass was last pinned to, if the platform had
    /// a blur view to tune.
    blur_mode: Option<theme::Appearance>,
    /// Set once AppKit has hidden the close/miniaturize/zoom buttons.
    traffic_lights_hidden: bool,
}

impl FlowlistApp {
    fn new(store: Store, cx: &mut Context<Self>) -> Self {
        let session: Session = store.read_session().unwrap_or_default();
        let mut view = session.view(store.db());
        if !view_exists(store.db(), &view) {
            view = View::Builtin(BuiltinPerspective::Inbox);
        }
        let selected = session
            .selected_tasks(store.db())
            .into_iter()
            .filter(|id| store.db().tasks.contains_key(id))
            .collect();
        let (theme_kind, theme_appearance) = session.theme_choice();
        let theme = Theme::new(theme_kind, theme_appearance);
        let sort = session.sort.as_deref().and_then(sort_from_slug);
        let group = session.group.as_deref().and_then(group_from_slug);
        let show_completed = session.show_completed.unwrap_or(false);
        let show_inspector = session.inspector.unwrap_or(true);
        let vim_mode = session.vim_mode.unwrap_or(true);
        let priority_filter = session.priority_filter();
        let only_available = session.only_available.unwrap_or(false);
        let follow_system = session.follow_system.unwrap_or(false);
        let view_options = session.view_options.clone();
        let window_size = session.window;
        let focus_handle = cx.focus_handle();
        let toast = store_warning(&store);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update(cx, |this, cx| {
                    if this.store.reload_if_changed().unwrap_or(false) {
                        this.prune_selection();
                        cx.notify();
                    }
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

        let mut app = Self {
            store,
            theme,
            view,
            forecast_month,
            forecast_date: None,
            forecast_calendar_anchor: Rc::new(Cell::new(None)),
            forecast_months_anchor: Rc::new(Cell::new(None)),
            forecast_width_anchor: Rc::new(Cell::new(None)),
            sidebar_row_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sidebar_name_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            sidebar_drag_target: None,
            content_rows: Rc::new(RefCell::new(Vec::new())),
            task_disclosures: Rc::new(RefCell::new(BTreeMap::new())),
            priority_filter,
            only_available,
            follow_system,
            view_options,
            sort_menu_open: false,
            group_menu_open: false,
            priority_menu_open: false,
            sort_anchor: Rc::new(Cell::new(None)),
            group_anchor: Rc::new(Cell::new(None)),
            priority_anchor: Rc::new(Cell::new(None)),
            modal_field_anchor: Rc::new(Cell::new(None)),
            due_anchor: Rc::new(Cell::new(None)),
            defer_anchor: Rc::new(Cell::new(None)),
            date_picker: None,
            date_picker_month: forecast_month,
            date_picker_cursor: today,
            date_picker_time: None,
            date_picker_step: DatePickerStep::Calendar,
            date_picker_unit: ClockUnit::Hour,
            date_picker_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            theme_boxes: Rc::new(RefCell::new(BTreeMap::new())),
            title_space: Rc::new(Cell::new(None)),
            title_visible: Rc::new(Cell::new(true)),
            sidebar_menu: None,
            sort_closed_on_up: false,
            group_closed_on_up: false,
            priority_closed_on_up: false,
            project_icon_open: false,
            suggestion_index: 0,
            suggestion_engaged: false,
            selected,
            anchor: None,
            expanded: BTreeSet::new(),
            collapsed_groups: BTreeSet::new(),
            show_inspector,
            show_completed,
            vim_mode,
            vim: vim::VimState::default(),
            sidebar_cursor: None,
            sidebar_hover: None,
            checklist_cursor: None,
            focus: FocusArea::Content,
            focus_handle,
            capture: text::TextBuffer::empty(),
            search: text::TextBuffer::empty(),
            field: text::TextBuffer::empty(),
            tag_buffer: text::TextBuffer::empty(),
            checklist_buffer: text::TextBuffer::empty(),
            checklist_editing: None,
            editing: None,
            field_error: None,
            draft_task: None,
            modal: None,
            theme_on_open: None,
            modal_focus: 0,
            modal_scroll: ScrollHandle::new(),
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
            last_sync: Instant::now(),
            test_state_path: std::env::var_os("FLOWLIST_TEST_STATE").map(PathBuf::from),
            test_state_last: None,
            blur_mode: None,
            traffic_lights_hidden: false,
        };
        app.restore_view_options();
        app
    }

    // ------------------------------------------------------------- session

    /// Switch light/dark inside the current family. A manual switch opts out
    /// of following the system appearance; otherwise the next frame would put
    /// the old appearance straight back.
    fn toggle_appearance(&mut self) {
        self.theme = self.theme.toggled();
        self.follow_system = false;
        self.toast = Some(format!(
            "{} · {}",
            self.theme.kind().label(),
            self.theme.appearance().label()
        ));
        self.save_session();
    }

    fn save_session(&self) {
        let session = Session {
            view: Some(self.view.key()),
            selected: self.selected.iter().map(|id| id.to_string()).collect(),
            inspector: Some(self.show_inspector),
            theme: Some(self.theme.kind().slug().to_string()),
            appearance: Some(self.theme.appearance().slug().to_string()),
            window: self.window_size,
            sort: self.sort.map(|sort| sort_slug(sort).to_string()),
            group: self.group.map(|group| group_slug(group).to_string()),
            show_completed: Some(self.show_completed),
            vim_mode: Some(self.vim_mode),
            priority_filter: self.priority_filter.map(|p| p.slug().to_string()),
            only_available: Some(self.only_available),
            follow_system: Some(self.follow_system),
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

    // -------------------------------------------------------------- views

    fn current_groups(&self) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let mut groups = match &self.view {
            // Review has its own dashboard; Nearby its own ranked list. The
            // rest still need ids here so j/k, x and undo work on them.
            View::Builtin(BuiltinPerspective::Review) => Vec::new(),
            View::Builtin(BuiltinPerspective::Forecast) if self.forecast_date.is_some() => {
                let date = self.forecast_date.expect("guard checked");
                let ids = query::tasks_on_date(db, date, at);
                let title = query::forecast::day_label(date, local_date(at));
                vec![Group {
                    key: format!("forecast-day:{date}"),
                    title,
                    position: 0,
                    rows: query::rows_for(db, &ids, at),
                }]
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
            groups = self.filter_groups(groups);
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
        groups
    }

    fn project_groups(&self, id: &ProjectId) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let rows: Vec<TaskRow> = db
            .project_outline(id, self.show_completed)
            .into_iter()
            .map(|row| {
                let mut task_row = query::row_for(db, &row.task, at);
                task_row.depth = row.depth;
                task_row
            })
            .collect();
        let title = db
            .projects
            .get(id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project".to_string());
        vec![Group {
            key: format!("project:{id}"),
            title,
            position: 0,
            rows,
        }]
    }

    fn filter_groups(&self, groups: Vec<Group>) -> Vec<Group> {
        let db = self.store.db();
        let needle = self.search.text().trim().to_lowercase();
        let mut out = Vec::new();
        for mut group in groups {
            group.rows.retain(|row| {
                let Some(task) = db.tasks.get(&row.id) else {
                    return false;
                };
                // Stop at the first hit: title and notes cover most queries
                // without building the project path or joined tag list.
                if task.title.to_lowercase().contains(&needle)
                    || task.notes.to_lowercase().contains(&needle)
                {
                    return true;
                }
                if let Some(project) = task.project.as_ref().map(|id| db.project_path(id))
                    && project.to_lowercase().contains(&needle)
                {
                    return true;
                }
                task.tags.iter().any(|id| {
                    db.tags
                        .get(id)
                        .is_some_and(|tag| tag.name.to_lowercase().contains(&needle))
                })
            });
            if !group.rows.is_empty() {
                out.push(group);
            }
        }
        out
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

    /// Switch views without notifying. Callers that already repaint use this
    /// from inside a command; `open_view` is the interactive entry point.
    fn set_view(&mut self, view: View) {
        self.remember_view_options();
        self.project_icon_open = false;
        self.view = view;
        self.restore_view_options();
        self.selected.clear();
        self.anchor = None;
        self.search.clear();
        self.focus = FocusArea::Content;
        self.save_session();
    }

    fn view_title(&self) -> String {
        self.view.title(self.store.db())
    }

    fn view_subtitle(&self) -> &'static str {
        self.view.subtitle()
    }

    // ----------------------------------------------------------- selection

    fn selected_one(&self) -> Option<TaskId> {
        self.selected.first().cloned()
    }

    fn select_only(&mut self, id: TaskId) {
        if self.draft_task.as_ref() != Some(&id) {
            self.draft_task = None;
        }
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
                self.toast = outcome
                    .message
                    .clone()
                    .or_else(|| Some(outcome.label.clone()));
                self.prune_selection();
                self.save_session();
                Some(outcome)
            }
            Err(error) => {
                self.toast = Some(format!("Could not save: {error}"));
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

    fn remove_attachment(&mut self, path: String, cx: &mut Context<Self>) {
        if let Some(id) = self.selected_one() {
            let attachments = self
                .store
                .db()
                .tasks
                .get(&id)
                .map(|task| task.attachments.clone())
                .unwrap_or_default();
            let attachments: Vec<String> = attachments
                .into_iter()
                .filter(|candidate| candidate != &path)
                .collect();
            self.execute(Command::UpdateTask {
                id,
                patch: TaskPatch {
                    attachments: Some(attachments),
                    ..Default::default()
                },
            });
        } else if let View::Project(id) = self.view.clone() {
            let attachments = self
                .store
                .db()
                .projects
                .get(&id)
                .map(|project| project.attachments.clone())
                .unwrap_or_default();
            let attachments: Vec<String> = attachments
                .into_iter()
                .filter(|candidate| candidate != &path)
                .collect();
            self.execute(Command::UpdateProject {
                id,
                patch: ProjectPatch {
                    attachments: Some(attachments),
                    ..Default::default()
                },
            });
        }
        cx.notify();
    }

    fn open_attachment(&mut self, path: String) {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        match std::process::Command::new(opener).arg(&path).spawn() {
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
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::SetTaskPriority {
            ids: vec![id],
            priority,
        });
        cx.notify();
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
        self.toast = Some(format!("{verb} {count} task(s) · ⌘Z to undo"));
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
                (Command::DeleteTag { id: id.clone() }, View::Tag(id))
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
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement: TaskPlacement::Parent {
                parent,
                position: Position::End,
            },
        });
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
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement,
        });
        cx.notify();
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
        let Some(task_id) = self.selected_one() else {
            self.checklist_editing = None;
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let title = self.checklist_buffer.text().trim().to_string();
        if title.is_empty() {
            if self.checklist_editing.is_some() {
                self.toast = Some("A checklist item needs a title".to_string());
            } else {
                self.focus = FocusArea::Content;
            }
            cx.notify();
            return;
        }
        match self.checklist_editing.take() {
            Some(item) => {
                self.execute(Command::UpdateChecklistItem {
                    task: task_id,
                    item,
                    title,
                    completed: None,
                });
                self.checklist_buffer.clear();
                self.focus = FocusArea::Content;
            }
            None => {
                self.execute(Command::AddChecklistItem {
                    task: task_id,
                    title,
                    position: None,
                });
                self.checklist_buffer.clear();
            }
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
            DateField::Due => self.due_anchor.clone(),
        }
    }

    /// The stamp currently stored in the selected task's date field.
    fn date_picker_value(&self, field: DateField) -> Option<Stamp> {
        let id = self.selected_one()?;
        let task = self.store.db().tasks.get(&id)?;
        match field {
            DateField::Defer => task.defer,
            DateField::Due => task.due,
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
        let stamp = picker_stamp(date, self.date_picker_time);
        self.date_picker = None;
        self.date_picker_time = None;
        self.date_picker_step = DatePickerStep::Calendar;
        self.date_picker_unit = ClockUnit::Hour;
        let mut patch = TaskPatch::default();
        match field {
            DateField::Defer => patch.defer = Some(Some(stamp)),
            DateField::Due => patch.due = Some(Some(stamp)),
        }
        self.execute(Command::UpdateTask { id, patch });
        cx.notify();
    }

    fn clear_date_picker(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.date_picker else {
            return;
        };
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
            DateField::Due => patch.due = Some(None),
        }
        self.execute(Command::UpdateTask { id, patch });
        cx.notify();
    }

    fn begin_edit(&mut self, field: EditField, cx: &mut Context<Self>) {
        // Leaving the title editor ends the new-task draft even if the user
        // clicked straight into another field instead of pressing Return.
        if self.editing == Some(EditField::Title) && field != EditField::Title {
            self.draft_task = None;
        }
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
            EditField::Due => task
                .due
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Estimate => task.estimate_min.map(format_duration).unwrap_or_default(),
            EditField::Repeat => task
                .repeat
                .as_ref()
                .map(|rule| rule.description())
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
        cx.notify();
    }

    fn commit_field(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.editing else {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let Some(id) = self.selected_one() else {
            self.editing = None;
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let raw = self.field.text().to_string();
        let value = raw.trim().to_string();
        let mut patch = TaskPatch::default();
        match field {
            EditField::Title => {
                if value.is_empty() {
                    self.field_error = Some("A task needs a title".to_string());
                    cx.notify();
                    return;
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
                        return;
                    }
                    patch.project_path = Some(Some(value));
                }
            }
            EditField::Defer | EditField::Due => {
                let stamp = if value.is_empty() {
                    None
                } else {
                    match parse_stamp(&value, now()) {
                        Ok(stamp) => Some(stamp),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return;
                        }
                    }
                };
                if field == EditField::Defer {
                    patch.defer = Some(stamp);
                } else {
                    patch.due = Some(stamp);
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
                            return;
                        }
                    }
                };
                patch.estimate_min = Some(estimate);
            }
            EditField::Repeat => {
                let repeat = if value.is_empty() {
                    None
                } else {
                    match parse_repeat(&value, now()) {
                        Ok(rule) => Some(rule),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return;
                        }
                    }
                };
                patch.repeat = Some(repeat);
            }
        }
        self.execute(Command::UpdateTask { id, patch });
        self.editing = None;
        self.field_error = None;
        self.draft_task = None;
        self.focus = FocusArea::Content;
        cx.notify();
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
        let current = self.is_open(&key, true);
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

    fn toggle_group_collapsed(&mut self, key: String, cx: &mut Context<Self>) {
        self.toggle_open(key, true);
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

    /// Remember which sidebar row a dragged project is over. The folder row,
    /// LIBRARY heading and the empty sidebar area paint themselves from this.
    fn set_drag_target(&mut self, target: Option<String>, cx: &mut Context<Self>) {
        if self.sidebar_drag_target != target {
            self.sidebar_drag_target = target;
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

    fn drop_on_project(&mut self, project: ProjectId, payload: &DragTask, cx: &mut Context<Self>) {
        let ids = self.drag_ids(payload);
        if payload.tag {
            return;
        }
        self.execute(Command::MoveTasks {
            ids,
            placement: TaskPlacement::Project {
                project,
                position: Position::End,
            },
        });
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
        self.execute(Command::MoveTasks {
            ids,
            placement: TaskPlacement::Parent {
                parent: target,
                position: Position::End,
            },
        });
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
        self.toast = Some(format!("Copied {} task(s)", lines.len()));
        cx.notify();
    }

    // -------------------------------------------------------------- modals

    fn open_modal(&mut self, modal: modals::Modal, cx: &mut Context<Self>) {
        // Settings previews theme changes live, so remember what to put back
        // if the sheet is cancelled.
        if matches!(modal, modals::Modal::Settings { .. }) {
            self.theme_on_open = Some(self.theme);
        } else {
            self.theme_on_open = None;
        }
        self.modal = Some(modal);
        self.modal_focus = 0;
        self.modal_scroll.set_offset(point(px(0.), px(0.)));
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.sidebar_menu = None;
        self.focus = FocusArea::Modal;
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
        if let Some(theme) = self.theme_on_open.take() {
            self.theme = theme;
        }
        self.modal = None;
        self.modal_focus = 0;
        self.modal_scroll.set_offset(point(px(0.), px(0.)));
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn commit_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        let result = self.apply_modal(&modal);
        match result {
            Ok(()) => {
                self.modal = None;
                self.modal_focus = 0;
                self.focus = FocusArea::Content;
                // The previewed theme is now committed; drop the undo copy.
                self.theme_on_open = None;
            }
            Err(message) => {
                self.modal = Some(modal);
                self.toast = Some(message);
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
                name,
                notes,
                kind,
                icon,
                ..
            } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A project needs a name".to_string());
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
                new_project.icon = *icon;
                new_project.sequential = *kind == ProjectKind::Sequential;
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
            modals::Modal::DeleteProject { id, .. } => {
                self.execute(Command::DeleteProject { id: id.clone() });
                if self.view == View::Project(id.clone()) {
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
            modals::Modal::NewFolder { name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A folder needs a name".to_string());
                }
                let parent = match &self.view {
                    View::Folder(id) => Some(id.clone()),
                    _ => None,
                };
                self.execute(Command::CreateFolder {
                    name,
                    parent,
                    position: ContainerPosition::End,
                });
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
                radius,
                trash_days,
                vim_mode,
                follow_system,
                theme,
                appearance,
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
                self.execute(Command::UpdateSettings(SettingsPatch {
                    default_review_days: Some(review),
                    show_completed_days: Some(completed),
                    changed_days: Some(changed),
                    nearby_radius_km: Some(radius),
                    trash_retention_days: Some(trash),
                    ..Default::default()
                }));
                if let Err(error) = self.store.sweep_trash() {
                    self.toast = Some(format!("Could not empty expired trash: {error}"));
                }
                self.vim_mode = *vim_mode;
                self.follow_system = *follow_system;
                self.theme = Theme::new(*theme, *appearance);
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
        let stroke = &event.keystroke;
        let mods = stroke.modifiers;

        if self.focus == FocusArea::Modal {
            if mods.platform {
                if stroke.key == "w" || stroke.key == "q" {
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
                    if let Some(modals::Modal::Settings {
                        theme, appearance, ..
                    }) = &self.modal
                    {
                        // The shortcut previews just like the swatches do.
                        self.theme = Theme::new(*theme, *appearance);
                    }
                    cx.notify();
                    return;
                }
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
                        let count = self
                            .modal
                            .as_ref()
                            .map(|modal| modal.field_count())
                            .unwrap_or(0);
                        if count > 0 {
                            self.modal_focus = if mods.shift {
                                (self.modal_focus + count - 1) % count
                            } else {
                                (self.modal_focus + 1) % count
                            };
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
                        self.tab_focus(mods.shift, cx);
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
                    self.focus = FocusArea::Search;
                    self.search.select_all();
                }
                "i" => {
                    self.show_inspector = !self.show_inspector;
                    self.save_session();
                }
                "t" => {
                    if mods.shift || mods.alt {
                        self.theme = if mods.shift {
                            self.theme.next_theme()
                        } else {
                            self.theme.previous_theme()
                        };
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
                    || self.sidebar_menu.is_some()
                {
                    self.sort_menu_open = false;
                    self.group_menu_open = false;
                    self.priority_menu_open = false;
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
                self.selected.clear();
                self.anchor = None;
                self.save_session();
            }
            "m" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.cycle_forecast_months(cx);
            }
            "w" if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) => {
                self.toggle_forecast_width(cx);
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
                    || self.sidebar_menu.is_some()
                {
                    self.sort_menu_open = false;
                    self.group_menu_open = false;
                    self.priority_menu_open = false;
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
            format!("Pasted {created} task(s)")
        } else {
            format!("Pasted {created} task(s), skipped {failed} line(s)")
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
        cx: &mut Context<Self>,
    ) {
        self.modal_focus = index;
        self.focus = FocusArea::Modal;
        if let Some(modal) = self.modal.as_mut()
            && let Some(buffer) = modal.field_mut(index)
        {
            buffer.set_cursor(char_index, extend);
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
            FocusArea::Field => self.commit_field(cx),
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

    fn tab_focus(&mut self, backwards: bool, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Field => {
                let next = self.editing.map(|field| {
                    if backwards {
                        field.previous()
                    } else {
                        field.next()
                    }
                });
                self.commit_field(cx);
                if let Some(next) = next {
                    self.begin_edit(next, cx);
                }
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

fn handle_buffer_key(
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
        SortBy::Name => "name",
        SortBy::Flagged => "flagged",
        SortBy::Estimate => "estimate",
        SortBy::Created => "created",
        SortBy::CompletedAt => "completed",
    }
}

fn sort_from_slug(value: &str) -> Option<SortBy> {
    match value {
        "manual" => Some(SortBy::Manual),
        "due" => Some(SortBy::Due),
        "defer" => Some(SortBy::Defer),
        "name" => Some(SortBy::Name),
        "flagged" => Some(SortBy::Flagged),
        "estimate" => Some(SortBy::Estimate),
        "created" => Some(SortBy::Created),
        "completed" => Some(SortBy::CompletedAt),
        _ => None,
    }
}

fn sort_label(sort: Option<SortBy>) -> &'static str {
    match sort {
        None => "View",
        Some(SortBy::Manual) => "Manual",
        Some(SortBy::Due) => "Due",
        Some(SortBy::Defer) => "Defer",
        Some(SortBy::Name) => "Name",
        Some(SortBy::Flagged) => "Flagged",
        Some(SortBy::Estimate) => "Estimate",
        Some(SortBy::Created) => "Created",
        Some(SortBy::CompletedAt) => "Completed",
    }
}

fn group_slug(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "none",
        GroupBy::Project => "project",
        GroupBy::Folder => "folder",
        GroupBy::Tag => "tag",
        GroupBy::DueDate => "due",
        GroupBy::DeferDate => "defer",
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
        "flagged" => Some(GroupBy::Flagged),
        "estimate" => Some(GroupBy::Estimate),
        _ => None,
    }
}

fn group_label(group: Option<GroupBy>) -> &'static str {
    match group {
        None => "View",
        Some(GroupBy::None) => "None",
        Some(GroupBy::Project) => "Project",
        Some(GroupBy::Folder) => "Folder",
        Some(GroupBy::Tag) => "Tag",
        Some(GroupBy::DueDate) => "Due",
        Some(GroupBy::DeferDate) => "Defer",
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
            BuiltinPerspective::Inbox => db.inbox_outline(false).len(),
            BuiltinPerspective::Projects => db
                .projects
                .values()
                .filter(|project| project.status.is_open())
                .count(),
            BuiltinPerspective::Forecast => {
                let today = local_date(at);
                db.tasks
                    .values()
                    .filter(|task| {
                        !task.completed
                            && !task.is_trashed()
                            && (task.due.is_some_and(|stamp| stamp.local_date() <= today)
                                || task.defer.is_some_and(|stamp| stamp.local_date() <= today))
                    })
                    .count()
            }
            BuiltinPerspective::Flagged => db
                .tasks
                .values()
                .filter(|task| !task.completed && !task.is_trashed() && task.flagged)
                .count(),
            BuiltinPerspective::Review => query::review_projects(db, at).len(),
            BuiltinPerspective::Tags => db
                .flatten_tags()
                .into_iter()
                .filter(|row| db.tag_usage_count(&row.tag, true) > 0)
                .count(),
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
                    })
                    .count()
            }
            BuiltinPerspective::Nearby => query::nearby(db, at).len(),
            BuiltinPerspective::Changed => {
                let days = db.settings.changed_days.max(1) as i64;
                let cutoff = at - chrono::Duration::days(days);
                db.tasks
                    .values()
                    .filter(|task| {
                        !task.completed && !task.is_trashed() && task.modified_at >= cutoff
                    })
                    .count()
            }
            BuiltinPerspective::Attachments => db
                .tasks
                .values()
                .filter(|task| {
                    !task.completed && !task.is_trashed() && !task.attachments.is_empty()
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
            projects.sort_by_key(|project| project.name.to_lowercase());
            projects
                .iter()
                .map(|project| {
                    let (_, total) = db.project_progress(&project.id);
                    let open = db.project_open_count(&project.id);
                    let badge = match sidebar::project_badge(open, total) {
                        sidebar::ProjectBadge::Count(count) => count.to_string(),
                        sidebar::ProjectBadge::Done => "tick".to_string(),
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
        let mut state = serde_json::json!({
            "view": self.view.key(),
            "modal": self.modal.as_ref().map(|modal| modal.title()),
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
            "title_space": self.title_space.get(),
            "title_visible": self.title_visible.get(),
            "sidebar_menu": self.sidebar_menu.as_ref().map(|menu| menu.key()),
            "forecast_date": self.forecast_date.map(|date| date.to_string()),
            "trashed": self.store.db().trash_count(),
            "traffic_lights": if self.traffic_lights_hidden { "hidden" } else { "visible" },
            "modal_theme": match &self.modal {
                Some(modals::Modal::Settings { theme, appearance, .. }) => {
                    Some(format!("{}/{}", theme.slug(), appearance.slug()))
                }
                _ => None,
            },
        });
        // Inserted after the macro: the full field list is already at the
        // json! recursion limit, so new debug fields go in one at a time.
        state["project_badges"] = serde_json::json!(project_badges);
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
        state["theme_boxes"] = serde_json::json!(self.theme_boxes.borrow().clone());
        state["collapsed"] =
            serde_json::json!(self.collapsed_groups.iter().cloned().collect::<Vec<_>>());
        let settings = &self.store.db().settings;
        let window_width = self.window_size.map_or(1254.0, |(width, _)| width);
        state["forecast_months"] = serde_json::json!(settings.forecast_months.min(2));
        state["forecast_full_width"] = serde_json::json!(settings.forecast_full_width);
        state["forecast_month_count"] = serde_json::json!(content::forecast_month_count(
            window_width,
            self.show_inspector,
            settings.forecast_months.min(2),
        ));
        state["forecast_calendar"] = serde_json::json!(anchor(self.forecast_calendar_anchor.get()));
        state["forecast_months_anchor"] =
            serde_json::json!(anchor(self.forecast_months_anchor.get()));
        state["forecast_width_anchor"] =
            serde_json::json!(anchor(self.forecast_width_anchor.get()));
        state["modal_project_icon"] = serde_json::json!(match &self.modal {
            Some(modals::Modal::NewProject { icon, .. }) => Some(icon.slug()),
            _ => None,
        });
        state["sidebar_rows"] = serde_json::json!(self.sidebar_row_boxes.borrow().clone());
        state["sidebar_drag_target"] = serde_json::json!(self.sidebar_drag_target);
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
        state["task_disclosures"] = serde_json::json!(self.task_disclosures.borrow().clone());
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
            self.theme = Theme::new(self.theme.kind(), appearance);
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

    fn render_header(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let window_width = self.window_size.map_or(1254.0, |(width, _)| width);
        let (search_left, search_width) = search_slot(window_width, self.show_inspector);
        div()
            .flex()
            .items_center()
            .justify_between()
            .relative()
            .h(px(46.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.glass)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .pl_4()
                    .cursor(CursorStyle::OpenHand)
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Flowlist"),
                    ),
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
                                "Search tasks",
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
                    .gap_1()
                    .pr_3()
                    .child(
                        div()
                            .id("add-task")
                            .mr_1()
                            .flow_tip(self, cx, theme, "Add a task (⌃⌥Space)")
                            .child(widgets::button_with_icon(
                                theme,
                                Icon::Plus,
                                "Add task",
                                true,
                            ))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.focus_capture(window, cx)),
                            ),
                    )
                    .child(
                        div()
                            .id("undo")
                            .opacity(if self.store.can_undo() { 1.0 } else { 0.4 })
                            .flow_tip(self, cx, theme, "Undo (⌘Z)")
                            .child(widgets::icon_button(theme, Icon::Undo, false))
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    )
                    .child(
                        div()
                            .id("redo")
                            .opacity(if self.store.can_redo() { 1.0 } else { 0.4 })
                            .flow_tip(self, cx, theme, "Redo (⇧⌘Z)")
                            .child(widgets::icon_button(theme, Icon::Redo, false))
                            .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
                    )
                    .child(
                        div()
                            .id("help")
                            .flow_tip(self, cx, theme, "Keyboard and mouse help (?)")
                            .child(widgets::icon_button(theme, Icon::Help, false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_modal(modals::Modal::Help, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("settings")
                            .flow_tip(self, cx, theme, "Settings (⌘,)")
                            .child(widgets::icon_button(theme, Icon::Settings, false))
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
                            .child(widgets::icon_button(
                                theme,
                                if dark { Icon::Sun } else { Icon::Moon },
                                false,
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
                            .child(widgets::icon_button(theme, Icon::Info, self.show_inspector))
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
                    .child(format!("rev {revision}"))
                    .child(
                        div()
                            .max_w(px(360.))
                            .overflow_hidden()
                            .whitespace_nowrap()
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
        div()
            .absolute()
            .bottom(px(38.))
            .w_full()
            .flex()
            .justify_center()
            .child(
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
            )
    }

    fn render_empty(&self, title: &str, detail: &str) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .gap_2()
            .py_12()
            .child(icons::icon(Icon::CircleDashed, 34.0, theme.text_faint))
            .child(
                div()
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
        // The custom glass header replaces the system titlebar, so the
        // standard close/miniaturize/zoom controls are hidden rather than
        // floating over the app name.
        if !self.traffic_lights_hidden && blur::hide_traffic_lights(window) {
            self.traffic_lights_hidden = true;
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
            .flex()
            .flex_col()
            .size_full()
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
        root = root.child(self.render_header(window, cx));
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
            root = root.child(
                div()
                    .id("sort-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_sort_menu(cx)),
            );
        }
        if self.group_menu_open
            && let Some(anchor) = self.group_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 160.0, 250.0);
            root = root.child(
                div()
                    .id("group-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_group_menu(cx)),
            );
        }
        if self.priority_menu_open
            && let Some(anchor) = self.priority_anchor.get()
        {
            let (x, y) = menu_position(anchor, viewport, 150.0, 160.0);
            root = root.child(
                div()
                    .id("priority-menu-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_priority_menu(cx)),
            );
        }
        if let Some(field) = self.date_picker
            && let Some(anchor) = self.date_anchor(field).get()
        {
            let (x, y) =
                date_picker_position(anchor, viewport, DATE_PICKER_WIDTH, DATE_PICKER_HEIGHT);
            root = root.child(
                div()
                    .id("date-picker-layer")
                    .absolute()
                    .occlude()
                    .left(px(x))
                    .top(px(y))
                    .child(self.render_date_picker(cx)),
            );
        }
        if self.toast.is_some() {
            root = root.child(self.render_toast(cx));
        }
        if self.modal.is_some() {
            root = root.child(self.render_modal(window, cx));
        }
        // Written after the body renders, so measurements taken during this
        // frame (such as the title slot width) are current in the file.
        self.sync_test_state();
        root
    }
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
    Application::new()
        .with_assets(icons::Assets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, restored.unwrap_or(size(px(1240.), px(800.))), cx);
            let window = match cx.open_window(
                WindowOptions {
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Flowlist".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(18.), px(-2.))),
                    }),
                    window_background: WindowBackgroundAppearance::Blurred,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(880.), px(560.))),
                    ..Default::default()
                },
                |window, cx| {
                    let store = match Store::open(paths.clone()) {
                        Ok(store) => store,
                        Err(error) => {
                            eprintln!("could not open the Flowlist library: {error}");
                            std::process::exit(1);
                        }
                    };
                    let app = cx.new(|cx| FlowlistApp::new(store, cx));
                    let handle = app.read(cx).focus_handle.clone();
                    window.focus(&handle);
                    app
                },
            ) {
                Ok(window) => window,
                Err(error) => {
                    eprintln!("could not open the Flowlist window: {error}");
                    return;
                }
            };
            // A Carbon hotkey wakes the app even while another app is in
            // front. The callback only signals this foreground stream.
            if let Some(mut presses) = hotkey::register() {
                use futures::StreamExt as _;
                cx.spawn(async move |cx| {
                    while presses.next().await.is_some() {
                        let updated = window.update(cx, |app, window, cx| {
                            app.focus_capture(window, cx);
                        });
                        if updated.is_err() {
                            break;
                        }
                    }
                })
                .detach();
            }
            cx.activate(true);
        });
    Ok(())
}

/// The title-bar search sits centered on the content pane, not on the whole
/// window: the sidebar is 240 wide and the inspector 320. The field keeps a
/// usable width by giving way to the title-bar action cluster at the right
/// edge, shrinking first and, below that, shifting left just enough.
fn search_slot(window_width: f32, show_inspector: bool) -> (f32, f32) {
    const ACTIONS: f32 = 300.0;
    const FIELD_MAX: f32 = 420.0;
    const FIELD_MIN: f32 = 120.0;
    const PANE_INSET: f32 = 16.0;
    const ACTIONS_GAP: f32 = 8.0;

    let pane_left = SIDEBAR_WIDTH;
    let pane_right = window_width - if show_inspector { INSPECTOR_WIDTH } else { 0.0 };
    let pane_width = (pane_right - pane_left).max(0.0);
    let pane_center = pane_left + pane_width / 2.0;
    let mut width = FIELD_MAX.min((pane_width - 2.0 * PANE_INSET).max(0.0));
    let mut left = pane_left + (pane_width - width) / 2.0;

    if !show_inspector {
        // With the inspector hidden the actions float over the content pane's
        // right edge. Try to stay centered by shortening the field; if the
        // centered field would still be cramped, sit in the free band instead.
        let right_limit = window_width - ACTIONS - ACTIONS_GAP;
        let centered_room = 2.0 * (right_limit - pane_center);
        if centered_room >= FIELD_MIN {
            width = width.min(centered_room);
            left = pane_left + (pane_width - width) / 2.0;
        } else {
            let band_left = pane_left + PANE_INSET;
            let band_width = (right_limit - band_left).max(0.0);
            width = width.min(band_width);
            left = (right_limit - width).max(band_left);
        }
    }
    (left, width)
}

#[cfg(test)]
mod tests {
    use super::rank_suggestions;
    use super::{
        DATE_PICKER_HEIGHT, DATE_PICKER_WIDTH, DateField, View, clock_dial_position, clock_hour_12,
        clock_hour_24, date_picker_position, effective_priority_filter, first_of_month,
        picker_default_time, picker_stamp, stepped_time,
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
        let (left, width) = search_slot(1254.0, true);
        assert_eq!(width, 420.0);
        assert!((left + width / 2.0 - 587.0).abs() < 0.01);

        // Inspector hidden: the pane runs to the window edge, center 747. The
        // field shortens a little so the action cluster keeps its gutter.
        let (left, width) = search_slot(1254.0, false);
        assert_eq!(width, 398.0);
        assert!((left + width / 2.0 - 747.0).abs() < 0.01);

        // At the minimum window, a centered 160px field would collide with
        // the actions, so the field uses the free band and stays clear.
        let (left, width) = search_slot(880.0, false);
        assert_eq!(width, 316.0);
        assert_eq!(left, 256.0);
        assert!((left + width) <= 880.0 - 300.0 - 8.0 + 0.01);

        // With the inspector open at the minimum size, centering still fits.
        let (left, width) = search_slot(880.0, true);
        assert_eq!(width, 288.0);
        assert!((left + width / 2.0 - 400.0).abs() < 0.01);
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
}
