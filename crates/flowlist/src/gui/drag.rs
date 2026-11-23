//! Drag payloads and the floating ghost shown while dragging a task.

use gpui::{App, Context, IntoElement, Pixels, Point, Render, Window, div, prelude::*, px};

use flowlist_core::ids::{ChecklistId, FolderId, ProjectId, TaskId};

use super::icons::{self, Icon};
use super::theme::Theme;

#[derive(Clone, Debug)]
pub struct DragTask {
    pub id: TaskId,
    pub label: String,
    pub tag: bool,
}

impl DragTask {
    pub fn task(id: TaskId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            tag: false,
        }
    }
}

/// A one-off step being dragged inside its task's checklist.
#[derive(Clone, Debug)]
pub struct DragChecklist {
    pub task: TaskId,
    pub item: ChecklistId,
    pub label: String,
}

/// A project row being dragged onto a folder. Projects cannot nest, so this
/// only ever targets folders (or the Library heading to move back out).
#[derive(Clone, Debug)]
pub struct DragProject {
    pub id: ProjectId,
    pub label: String,
}

impl DragProject {
    pub fn project(id: ProjectId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}

/// A folder row being dragged onto another folder, or onto the Library heading
/// to move it back to the root. Folders can nest, so any folder is a target
/// unless it sits inside the dragged one.
#[derive(Clone, Debug)]
pub struct DragFolder {
    pub id: FolderId,
    pub label: String,
}

impl DragFolder {
    pub fn folder(id: FolderId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}

/// Dragging the Forecast calendar's bottom grip. The payload carries no state:
/// the live height comes from where the pointer sits under the calendar's top
/// edge, so a resize always tracks the cursor exactly.
#[derive(Clone, Copy, Debug)]
pub struct DragForecastResize;

pub struct DragGhost {
    label: String,
    position: Point<Pixels>,
    theme: Theme,
    icon: Icon,
}

impl DragGhost {
    pub fn new(label: String, icon: Icon, position: Point<Pixels>, theme: Theme) -> Self {
        Self {
            label,
            position,
            theme,
            icon,
        }
    }
}

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let width = px(240.);
        let height = px(30.);
        div()
            .pl(self.position.x - px(24.))
            .pt(self.position.y - px(15.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w(width)
                    .h(height)
                    .px_2()
                    .rounded_md()
                    .border_1()
                    .border_color(self.theme.accent)
                    .bg(self.theme.raised)
                    .text_color(self.theme.text)
                    .text_sm()
                    .shadow_md()
                    .child(icons::icon(self.icon, 13.0, self.theme.text_dim))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(self.label.clone()),
                    ),
            )
    }
}

/// Dragging the Settings sheet's bottom-right corner. Like the calendar grip,
/// the live size comes from the pointer's position inside the sheet.
#[derive(Clone, Copy, Debug)]
pub struct DragSettingsResize;

/// Dragging the Settings sheet's custom scrollbar thumb.
#[derive(Clone, Copy, Debug)]
pub struct DragSettingsScroll;

pub fn settings_scroll_ghost(
    _payload: &DragSettingsScroll,
    _position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<ResizeGhost> {
    cx.new(|_| ResizeGhost)
}

pub fn settings_resize_ghost(
    _payload: &DragSettingsResize,
    _position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<ResizeGhost> {
    cx.new(|_| ResizeGhost)
}

/// Constructor used by the calendar's resize grip. The block itself shows the
/// live height, so the drag paints nothing under the pointer.
pub fn resize_ghost(
    _payload: &DragForecastResize,
    _position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
) -> gpui::Entity<ResizeGhost> {
    cx.new(|_| ResizeGhost)
}

pub struct ResizeGhost;

impl Render for ResizeGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size(px(0.))
    }
}

/// Constructor used by a one-off row's `.on_drag`.
pub fn checklist_ghost(
    payload: &DragChecklist,
    position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
    theme: Theme,
) -> gpui::Entity<DragGhost> {
    cx.new(|_| DragGhost::new(payload.label.clone(), Icon::Check, position, theme))
}

/// Constructor used by `.on_drag`.
pub fn ghost(
    payload: &DragTask,
    position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
    theme: Theme,
) -> gpui::Entity<DragGhost> {
    let label = payload.label.clone();
    let icon = if payload.tag {
        Icon::Tag
    } else {
        Icon::Project
    };
    cx.new(|_| DragGhost::new(label, icon, position, theme))
}

/// Constructor used by a project row's `.on_drag`.
pub fn project_ghost(
    payload: &DragProject,
    position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
    theme: Theme,
) -> gpui::Entity<DragGhost> {
    let label = payload.label.clone();
    cx.new(|_| DragGhost::new(label, Icon::Project, position, theme))
}

/// Constructor used by a folder row's `.on_drag`.
pub fn folder_ghost(
    payload: &DragFolder,
    position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
    theme: Theme,
) -> gpui::Entity<DragGhost> {
    let label = payload.label.clone();
    cx.new(|_| DragGhost::new(label, Icon::Folder, position, theme))
}
