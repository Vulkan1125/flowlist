//! Small in-house widgets.
//!
//! Everything is drawn with plain GPUI elements: no SVG assets, no web view.
//! Text fields render their buffer as runs so the caret and selection land on
//! the right characters even with multi-byte input.

use gpui::{
    AnyView, App, CursorStyle, FontWeight, IntoElement, Render, SharedString, Window, div,
    prelude::*, px,
};

use super::icons::{self, Icon};
use super::text::TextBuffer;
use flowlist_core::model::ProjectIcon;

use super::theme::Theme;

pub fn caret(theme: Theme) -> impl IntoElement {
    div().w(px(1.5)).h(px(15.)).bg(theme.accent).flex_shrink_0()
}

fn run(text: String, selected: bool, theme: Theme) -> impl IntoElement {
    div()
        .whitespace_normal()
        .when(selected, |this| this.bg(theme.selection))
        .child(text)
}

/// A single-line editable field.
pub fn text_field(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
) -> impl IntoElement {
    let chars: Vec<char> = buffer.text().chars().collect();
    let (start, end) = buffer.selection();
    let has_selection = start != end;
    let before: String = chars[..start.min(chars.len())].iter().collect();
    let selected: String = chars[start.min(chars.len())..end.min(chars.len())]
        .iter()
        .collect();
    let after: String = chars[end.min(chars.len())..].iter().collect();
    let empty = chars.is_empty();

    div()
        .flex()
        .items_center()
        .h(px(28.))
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(if focused { theme.accent } else { theme.border })
        .bg(if focused { theme.raised } else { theme.field })
        .text_sm()
        .cursor(CursorStyle::IBeam)
        .overflow_hidden()
        .child(
            div()
                .flex()
                .items_center()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(run(before, false, theme))
                .when(has_selection, |this| {
                    this.child(run(selected.clone(), true, theme))
                })
                .when(!has_selection && focused, |this| this.child(caret(theme)))
                .child(run(after, false, theme))
                .when(empty, |this| {
                    this.child(
                        div()
                            .text_color(theme.text_faint)
                            .child(placeholder.to_string()),
                    )
                }),
        )
}

/// A multi-line field used for notes and pasted imports.
pub fn text_area(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
) -> impl IntoElement {
    let (sel_start, sel_end) = buffer.selection();
    let cursor = buffer.cursor();
    let text = buffer.text().to_string();
    let mut offset = 0usize;
    let mut lines: Vec<gpui::AnyElement> = Vec::new();

    for raw in text.split('\n') {
        let line_len = raw.chars().count();
        let line_start = offset;
        let line_end = offset + line_len;
        let line_chars: Vec<char> = raw.chars().collect();

        let caret_here = focused
            && cursor >= line_start
            && (cursor < line_end || (cursor == line_end && line_end == text.chars().count()));

        let mut children: Vec<gpui::AnyElement> = Vec::new();
        if sel_start != sel_end && sel_start < line_end && sel_end > line_start {
            let local_start = sel_start.saturating_sub(line_start).min(line_chars.len());
            let local_end = (sel_end - line_start).min(line_chars.len());
            let before: String = line_chars[..local_start].iter().collect();
            let selected: String = line_chars[local_start..local_end].iter().collect();
            let after: String = line_chars[local_end..].iter().collect();
            children.push(run(before, false, theme).into_any_element());
            children.push(run(selected, true, theme).into_any_element());
            children.push(run(after, false, theme).into_any_element());
        } else {
            let col = if caret_here {
                cursor.saturating_sub(line_start).min(line_chars.len())
            } else {
                line_chars.len()
            };
            let before: String = line_chars[..col].iter().collect();
            let after: String = line_chars[col..].iter().collect();
            children.push(run(before, false, theme).into_any_element());
            if caret_here {
                children.push(caret(theme).into_any_element());
            }
            children.push(run(after, false, theme).into_any_element());
        }

        lines.push(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .min_h(px(17.))
                .children(children)
                .into_any_element(),
        );
        offset = line_end + 1;
    }

    div()
        .h(px(height))
        .w_full()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(if focused { theme.accent } else { theme.border })
        .bg(if focused { theme.raised } else { theme.field })
        .text_sm()
        .overflow_hidden()
        .cursor(CursorStyle::IBeam)
        .child(
            div()
                .flex()
                .flex_col()
                .when(buffer.is_empty() && !focused, |this| {
                    this.child(
                        div()
                            .text_color(theme.text_faint)
                            .child(placeholder.to_string()),
                    )
                })
                .children(lines),
        )
}

pub fn chip(_theme: Theme, text: impl Into<String>, color: gpui::Hsla) -> impl IntoElement {
    div()
        .px_2()
        .py_0p5()
        .rounded_sm()
        .bg(color.opacity(0.14))
        .text_color(color)
        .text_xs()
        .child(text.into())
}

pub fn tag_chip(theme: Theme, text: impl Into<String>) -> impl IntoElement {
    chip(theme, text, theme.text_dim)
}

pub fn checkbox(theme: Theme, completed: bool, accent: gpui::Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(16.))
        .rounded_full()
        .border_1()
        .border_color(if completed { accent } else { theme.border })
        .bg(if completed { accent } else { theme.field })
        .text_color(if completed { theme.raised } else { accent })
        .text_xs()
        .when(completed, |this| {
            this.child(icons::icon(Icon::Check, 11.0, theme.bg))
        })
}

pub fn progress(theme: Theme, done: usize, total: usize) -> impl IntoElement {
    let ratio = if total == 0 {
        0.0
    } else {
        (done as f32 / total as f32).clamp(0.0, 1.0)
    };
    div()
        .h(px(4.))
        .w_full()
        .rounded_full()
        .bg(theme.border_soft)
        .child(
            div()
                .h_full()
                .w(gpui::relative(ratio))
                .rounded_full()
                .bg(theme.accent),
        )
}

pub fn section_label(theme: Theme, text: impl Into<String>) -> impl IntoElement {
    div()
        .px_3()
        .pt_3()
        .pb_1()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_faint)
        .child(text.into())
}

pub fn button(theme: Theme, label: impl Into<String>, primary: bool) -> impl IntoElement {
    let label = label.into();
    div()
        .h(px(26.))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(if primary { theme.accent } else { theme.border })
        .bg(if primary { theme.accent } else { theme.raised })
        .text_color(if primary { theme.raised } else { theme.text })
        .text_sm()
        .cursor_pointer()
        .hover(|style| {
            if primary {
                style.opacity(0.9)
            } else {
                style.bg(theme.hover)
            }
        })
        .child(label)
}

/// A button that leads with an icon, used for creation actions so the plus
/// mark is drawn rather than typed.
pub fn button_with_icon(
    theme: Theme,
    icon: Icon,
    label: impl Into<String>,
    primary: bool,
) -> impl IntoElement {
    let label = label.into();
    div()
        .h(px(26.))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .rounded_md()
        .border_1()
        .border_color(if primary { theme.accent } else { theme.border })
        .bg(if primary { theme.accent } else { theme.raised })
        .text_color(if primary { theme.raised } else { theme.text })
        .text_sm()
        .cursor_pointer()
        .hover(|style| {
            if primary {
                style.opacity(0.9)
            } else {
                style.bg(theme.hover)
            }
        })
        .child(icons::icon(
            icon,
            13.0,
            if primary { theme.bg } else { theme.text_dim },
        ))
        .child(label)
}

pub fn icon_button(theme: Theme, icon: Icon, active: bool) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.))
        .rounded_md()
        .bg(if active { theme.selection } else { theme.clear })
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .child(icons::icon(
            icon,
            16.0,
            if active { theme.accent } else { theme.text_dim },
        ))
}

pub fn count(theme: Theme, value: usize) -> impl IntoElement {
    div()
        .min_w(px(18.))
        .px_1()
        .rounded_sm()
        .bg(theme.hover)
        .text_color(theme.text_faint)
        .text_xs()
        .text_center()
        .child(value.to_string())
}

/// A compact filter/metric button: same height as the search field so sort,
/// group and toggles line up with it.
pub fn toggle_chip(theme: Theme, label: impl Into<String>, active: bool) -> impl IntoElement {
    div()
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(if active { theme.accent } else { theme.border })
        .bg(if active { theme.selection } else { theme.field })
        .text_xs()
        .text_color(if active { theme.accent } else { theme.text_dim })
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .child(label.into())
}

pub fn divider(theme: Theme) -> impl IntoElement {
    div().h(px(1.)).w_full().bg(theme.border_soft)
}

/// A tooltip shown after GPUI's built-in hover delay. Icon-only controls attach
/// one so their meaning is discoverable without opening the help card.
struct TipView {
    label: SharedString,
    theme: Theme,
}

impl Render for TipView {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .text_xs()
            .text_color(theme.text)
            .whitespace_nowrap()
            .child(self.label.clone())
    }
}

/// Builder for `.tooltip(...)`. GPUI waits before showing it, so passing the
/// mouse across a row stays quiet.
pub fn tip(
    theme: Theme,
    label: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let label = label.into();
    move |_window, cx| {
        let label = label.clone();
        cx.new(|_| TipView { label, theme }).into()
    }
}

/// The small icon that stands in for a project in the rail and dashboards.
pub fn icon_for_project(icon: ProjectIcon, size: f32, color: gpui::Hsla) -> impl IntoElement {
    let glyph = match icon {
        ProjectIcon::Project => Icon::Project,
        ProjectIcon::Folder => Icon::Folder,
        ProjectIcon::Star => Icon::Star,
        ProjectIcon::Briefcase => Icon::Briefcase,
        ProjectIcon::Home => Icon::Home,
        ProjectIcon::Heart => Icon::Heart,
        ProjectIcon::Book => Icon::Book,
        ProjectIcon::Code => Icon::Code,
        ProjectIcon::Rocket => Icon::Rocket,
        ProjectIcon::Coffee => Icon::Coffee,
    };
    icons::icon(glyph, size, color)
}
