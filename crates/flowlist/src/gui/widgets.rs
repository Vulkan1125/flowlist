//! Small in-house widgets.
//!
//! Everything is drawn with plain GPUI elements: no SVG assets, no web view.
//! Text fields render their buffer as runs so the caret and selection land on
//! the right characters even with multi-byte input.

use std::time::Duration;

use gpui::{
    AnyView, App, Context, CursorStyle, Div, FontWeight, HighlightStyle, IntoElement, Render,
    SharedString, Stateful, StyledText, TextStyle, Window, div, prelude::*, px,
};

use super::FlowlistApp;
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
    text_field_with_trailer(theme, buffer, placeholder, focused, 0.0)
}

/// A text field that leaves room for a control painted over its right edge,
/// such as the clear button on the title-bar search. `trailer` is the extra
/// right padding in pixels; the normal field is the same with zero.
pub fn text_field_with_trailer(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    trailer: f32,
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
        .pl_2()
        .pr(px(8.0 + trailer))
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
    let chars: Vec<char> = text.chars().collect();
    let has_selection = sel_start != sel_end;

    // Build the visible string with a caret glyph at the cursor. StyledText
    // lays the whole run out with normal wrapping, so long notes wrap inside
    // the inspector instead of running past its edge.
    let mut display = String::with_capacity(text.len() + 1);
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    let mut selection: Option<(usize, usize)> = None;

    let push_caret = |display: &mut String, highlights: &mut Vec<_>| {
        let start = display.len();
        display.push('▏');
        highlights.push((
            start..display.len(),
            HighlightStyle {
                color: Some(theme.accent),
                ..Default::default()
            },
        ));
    };

    for (index, ch) in chars.iter().enumerate() {
        if focused && !has_selection && index == cursor {
            push_caret(&mut display, &mut highlights);
        }
        let start = display.len();
        display.push(*ch);
        if has_selection && index >= sel_start && index < sel_end {
            selection = Some(match selection {
                Some((first, _)) => (first, display.len()),
                None => (start, display.len()),
            });
        }
    }
    if focused && !has_selection && cursor >= chars.len() {
        push_caret(&mut display, &mut highlights);
    }
    if let Some((start, end)) = selection {
        highlights.push((
            start..end,
            HighlightStyle {
                background_color: Some(theme.selection),
                ..Default::default()
            },
        ));
    }

    let text_style = TextStyle {
        color: theme.text,
        font_family: "SF Pro Text".into(),
        font_size: px(13.).into(),
        line_height: px(18.).into(),
        ..Default::default()
    };

    let content = if buffer.is_empty() && !focused {
        div()
            .text_color(theme.text_faint)
            .child(placeholder.to_string())
            .into_any_element()
    } else {
        StyledText::new(display)
            .with_default_highlights(&text_style, highlights)
            .into_any_element()
    };

    div()
        .min_h(px(height))
        .w_full()
        .min_w_0()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(if focused { theme.accent } else { theme.border })
        .bg(if focused { theme.raised } else { theme.field })
        .text_sm()
        .cursor(CursorStyle::IBeam)
        .child(content)
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
    shown: bool,
}

/// A tooltip waits before appearing, and a click keeps it quiet until the
/// pointer leaves the button. GPUI's built-in delay is half a second, which
/// reads as "immediately" in a dense toolbar, so the view itself stays empty
/// for the first stretch and only then draws its label.
const TIP_DELAY: Duration = Duration::from_millis(4_500);

impl TipView {
    fn new(label: SharedString, theme: Theme, window: &Window, cx: &mut Context<Self>) -> Self {
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(TIP_DELAY).await;
            this.update(cx, |view, cx| {
                view.shown = true;
                cx.notify();
            })
            .ok();
            // The tooltip is prepared outside the normal view tree, so ask
            // the window to redraw once the label is ready.
            cx.update(|window, _| window.refresh()).ok();
        })
        .detach();
        Self {
            label,
            theme,
            shown: false,
        }
    }
}

impl Render for TipView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.shown {
            return div().w(px(0.)).h(px(0.)).into_any_element();
        }
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
            .into_any_element()
    }
}

/// Attach a delayed, click-aware tooltip. A click on the element mutes its
/// tooltip until the pointer leaves; the next hover starts fresh.
pub(super) trait TipExt: Sized {
    fn flow_tip(
        self,
        app: &FlowlistApp,
        cx: &mut Context<FlowlistApp>,
        theme: Theme,
        label: impl Into<SharedString>,
    ) -> Self;
}

macro_rules! impl_tip_ext {
    ($($ty:ty),*) => {
        $(
            impl TipExt for $ty {
                fn flow_tip(
                    self,
                    app: &FlowlistApp,
                    cx: &mut Context<FlowlistApp>,
                    theme: Theme,
                    label: impl Into<SharedString>,
                ) -> Self {
                    let label = label.into();
                    let muted = app.tip_suppressed(&label);
                    let el = if muted {
                        self
                    } else {
                        self.tooltip(tip(theme, label.clone()))
                    };
                    el.on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        this.note_tip_hover(label.clone(), *hovered, cx);
                    }))
                }
            }
        )*
    };
}

impl_tip_ext!(Stateful<Div>);

/// Builder for `.tooltip(...)`. GPUI waits before showing it, so passing the
/// mouse across a row stays quiet.
pub fn tip(
    theme: Theme,
    label: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let label = label.into();
    move |window, cx| {
        let label = label.clone();
        cx.new(|cx| TipView::new(label, theme, window, cx)).into()
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
