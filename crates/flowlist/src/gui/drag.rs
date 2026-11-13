//! Drag payloads and the floating ghost shown while dragging a task.

use gpui::{App, Context, IntoElement, Pixels, Point, Render, Window, div, prelude::*, px};

use flowlist_core::ids::TaskId;

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

pub struct DragGhost {
    label: String,
    position: Point<Pixels>,
    theme: Theme,
    tag: bool,
}

impl DragGhost {
    pub fn new(label: String, tag: bool, position: Point<Pixels>, theme: Theme) -> Self {
        Self {
            label,
            position,
            theme,
            tag,
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
                    .child(icons::icon(
                        if self.tag { Icon::Tag } else { Icon::Project },
                        13.0,
                        self.theme.text_dim,
                    ))
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

/// Constructor used by `.on_drag`.
pub fn ghost(
    payload: &DragTask,
    position: Point<Pixels>,
    _window: &mut Window,
    cx: &mut App,
    theme: Theme,
) -> gpui::Entity<DragGhost> {
    let label = payload.label.clone();
    let tag = payload.tag;
    cx.new(|_| DragGhost::new(label, tag, position, theme))
}
