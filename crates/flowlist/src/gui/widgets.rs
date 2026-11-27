//! Small in-house widgets.
//!
//! Everything is drawn with plain GPUI elements: no SVG assets, no web view.
//! Text fields render their buffer as runs so the caret and selection land on
//! the right characters even with multi-byte input.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyView, App, Context, CursorStyle, Div, FontWeight, HighlightStyle, IntoElement, MouseButton,
    Pixels, Point, Render, SharedString, Stateful, StyledText, TextLayout, TextStyle, Window, div,
    prelude::*, px,
};

use super::FlowlistApp;
use super::icons::{self, Icon};
use super::text::TextBuffer;
use flowlist_core::model::{IconSpec, ProjectIcon};

use super::theme::Theme;

/// Shape `text` and cut it to `room` points, ending in an ellipsis.
///
/// GPUI's own `truncate` sizes the text against the space its parent offers at
/// layout time, which for a flex child can be wider than the final cell, so a
/// long path or title paints clipped mid-glyph with no ellipsis. Measuring the
/// real text first keeps the cut deliberate and the ellipsis visible.
pub fn fit_head(
    text: &str,
    room: f32,
    font_size: f32,
    weight: FontWeight,
    color: gpui::Hsla,
    window: &mut Window,
) -> String {
    let width = |value: &str| -> f32 {
        let runs = [gpui::TextRun {
            len: value.len(),
            font: gpui::Font {
                weight,
                ..gpui::font("SF Pro Text")
            },
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }];
        let shaped = window.text_system().shape_line(
            SharedString::from(value.to_string()),
            px(font_size),
            &runs,
            None,
        );
        f32::from(shaped.width)
    };
    if width(text) <= room {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        let mut candidate: String = chars[..middle].iter().collect();
        candidate.push('…');
        if width(&candidate) <= room {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut cut: String = chars[..low].iter().collect();
    cut.push('…');
    cut
}

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
    text_field_inner(theme, buffer, placeholder, focused, trailer, None, true)
}

/// A character offset from a click or drag inside a text field.
pub struct TextHit {
    pub index: usize,
    pub extend: bool,
    /// 1 for a single click, 2 for a double-click, 3 for a triple-click.
    pub click_count: usize,
}

/// The sheet's handler for a click or drag hit in a text field.
type CursorHandler = Rc<dyn Fn(&TextHit, &mut Window, &mut App)>;

/// A single-line field that also accepts mouse selection. The caller turns a
/// hit into a caret move (or, while dragging, an extended selection). Used by
/// the sheets, where the old field could only be edited with the keyboard.
pub fn text_field_with_cursor(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    on_cursor: impl Fn(&TextHit, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    text_field_inner(
        theme,
        buffer,
        placeholder,
        focused,
        0.0,
        Some(Rc::new(on_cursor)),
        true,
    )
}

/// A cursor-aware field with no border or background, for embedding inside a
/// larger control such as the repeat interval box.
pub fn inline_text_field_with_cursor(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    on_cursor: impl Fn(&TextHit, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    text_field_inner(
        theme,
        buffer,
        placeholder,
        focused,
        0.0,
        Some(Rc::new(on_cursor)),
        false,
    )
}

fn text_field_inner(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    trailer: f32,
    on_cursor: Option<CursorHandler>,
    chrome: bool,
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

    // A transparent copy of the text gives click handling a real layout to
    // map pixels back to character offsets. The visible runs keep the caret
    // and selection drawing unchanged.
    let layout: Rc<RefCell<Option<TextLayout>>> = Rc::new(RefCell::new(None));
    let measure = on_cursor.as_ref().map(|_| {
        // The visible runs inherit the field's `text_sm` (14px) style, so
        // the hidden copy must match or clicks land on the wrong character.
        let style = TextStyle {
            color: theme.text,
            font_family: "SF Pro Text".into(),
            font_size: px(14.).into(),
            line_height: px(28.).into(),
            ..Default::default()
        };
        let styled =
            StyledText::new(buffer.text().to_string()).with_default_highlights(&style, Vec::new());
        *layout.borrow_mut() = Some(styled.layout().clone());
        styled
    });
    let hit_text = buffer.text().to_string();
    let hit_layout = layout.clone();
    let hit = Rc::new(move |position: Point<Pixels>| -> Option<usize> {
        let layout = hit_layout.borrow();
        let layout = layout.as_ref()?;
        let byte = layout
            .index_for_position(position)
            .unwrap_or_else(|index| index)
            .min(hit_text.len());
        Some(hit_text[..byte].chars().count())
    });

    let cursor_down = on_cursor.clone();
    let cursor_move = on_cursor.clone();

    div()
        .flex()
        .items_center()
        .h(px(28.))
        .pl_2()
        .pr(px(8.0 + trailer))
        .when(chrome, |this| {
            this.rounded_md()
                .border_1()
                .border_color(if focused { theme.accent } else { theme.border })
                .bg(if focused { theme.raised } else { theme.field })
        })
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
        .when_some(measure, |this, measure| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(8.0))
                    .right(px(8.0 + trailer))
                    .opacity(0.0)
                    .child(measure),
            )
        })
        .when_some(cursor_down, |this, cursor| {
            let hit = hit.clone();
            this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if let Some(index) = hit(event.position) {
                    cursor(
                        &TextHit {
                            index,
                            extend: event.modifiers.shift,
                            click_count: event.click_count,
                        },
                        window,
                        cx,
                    );
                }
            })
        })
        // The pressed button on the move event, rather than a drag flag,
        // decides whether to extend: a mouse-up outside the field would
        // otherwise leave the flag stuck on.
        .when_some(cursor_move, |this, cursor| {
            let hit = hit.clone();
            this.on_mouse_move(move |event, window, cx| {
                if event.pressed_button == Some(MouseButton::Left)
                    && let Some(index) = hit(event.position)
                {
                    cursor(
                        &TextHit {
                            index,
                            extend: true,
                            click_count: 1,
                        },
                        window,
                        cx,
                    );
                }
            })
        })
}

/// A multi-line field used for notes and pasted imports.
pub fn text_area(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
) -> impl IntoElement {
    text_area_with_size(theme, buffer, placeholder, focused, height, 13.0, 18.0)
}

/// A multi-line field that also accepts mouse selection, with the default
/// multi-line type. Used by the inspector notes.
pub fn text_area_with_cursor(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
    on_cursor: impl Fn(&TextHit, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    text_area_inner(
        theme,
        buffer,
        placeholder,
        focused,
        height,
        13.0,
        18.0,
        Some(Rc::new(on_cursor)),
    )
}

/// A multi-line field with caller-chosen type, used by the inspector title so
/// editing wraps exactly where the read-only title does instead of pushing the
/// caret past a single-line field's edge.
pub fn text_area_with_size(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
    font_size: f32,
    line_height: f32,
) -> impl IntoElement {
    text_area_inner(
        theme,
        buffer,
        placeholder,
        focused,
        height,
        font_size,
        line_height,
        None,
    )
}

/// The inspector title's wrapping editor with mouse selection.
pub fn text_area_with_size_and_cursor(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
    font_size: f32,
    line_height: f32,
    on_cursor: impl Fn(&TextHit, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    text_area_inner(
        theme,
        buffer,
        placeholder,
        focused,
        height,
        font_size,
        line_height,
        Some(Rc::new(on_cursor)),
    )
}

fn text_area_inner(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    height: f32,
    font_size: f32,
    line_height: f32,
    on_cursor: Option<CursorHandler>,
) -> impl IntoElement {
    let (sel_start, sel_end) = buffer.selection();
    let cursor = buffer.cursor();
    let text = buffer.text().to_string();
    let has_selection = sel_start != sel_end;

    // The caret is painted as a thin overlay rather than inserted into the
    // string. A careted glyph such as "▏" falls back to a wide font on macOS,
    // which leaves a conspicuous blank cell to its right and pushes the text
    // after it by a whole character.
    let byte_at = |char_index: usize| {
        text.char_indices()
            .nth(char_index)
            .map(|(byte, _)| byte)
            .unwrap_or(text.len())
    };
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    if has_selection {
        highlights.push((
            byte_at(sel_start)..byte_at(sel_end),
            HighlightStyle {
                background_color: Some(theme.selection),
                ..Default::default()
            },
        ));
    }

    let text_style = TextStyle {
        color: theme.text,
        font_family: "SF Pro Text".into(),
        font_size: px(font_size).into(),
        line_height: px(line_height).into(),
        ..Default::default()
    };

    let (content, text_layout) = if buffer.is_empty() && !focused {
        (
            div()
                .text_color(theme.text_faint)
                .child(placeholder.to_string())
                .into_any_element(),
            None,
        )
    } else {
        let styled = StyledText::new(text.clone())
            .with_default_highlights(&text_style, highlights);
        let layout = styled.layout().clone();
        (styled.into_any_element(), Some(layout))
    };

    let hit_text = text.clone();
    let hit_layout = text_layout.clone();
    let hit = Rc::new(move |position: Point<Pixels>| -> Option<usize> {
        let layout = hit_layout.as_ref()?;
        let byte = layout
            .index_for_position(position)
            .unwrap_or_else(|index| index)
            .min(hit_text.len());
        Some(hit_text[..byte].chars().count())
    });

    let cursor_down = on_cursor.clone();
    let cursor_move = on_cursor.clone();

    let mut field = div()
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
        .child(content);

    if focused
        && !has_selection
        && let Some(layout) = text_layout.clone()
    {
        let caret_byte = byte_at(cursor);
        let caret_color = theme.accent;
        let caret_height = line_height;
        field = field.child(
            gpui::canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    if let Some(position) = layout.position_for_index(caret_byte) {
                        window.paint_quad(gpui::fill(
                            gpui::Bounds::new(
                                gpui::point(position.x - px(0.75), position.y),
                                gpui::size(px(1.5), px(caret_height)),
                            ),
                            caret_color,
                        ));
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
    }

    field = field.when_some(cursor_down, |this, cursor| {
        let hit = hit.clone();
        this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
            if let Some(index) = hit(event.position) {
                cursor(
                    &TextHit {
                        index,
                        extend: event.modifiers.shift,
                        click_count: event.click_count,
                    },
                    window,
                    cx,
                );
            }
        })
    });

    // As with the single-line field, the pressed button decides whether a
    // move extends the selection, so a mouse-up outside the field cannot
    // leave a drag flag stuck on.
    field = field.when_some(cursor_move, |this, cursor| {
        let hit = hit.clone();
        this.on_mouse_move(move |event, window, cx| {
            if event.pressed_button == Some(MouseButton::Left)
                && let Some(index) = hit(event.position)
            {
                cursor(
                    &TextHit {
                        index,
                        extend: true,
                        click_count: 1,
                    },
                    window,
                    cx,
                );
            }
        })
    });

    field
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

/// A chip that leads with one of the drawn icons, so row metadata reads the
/// same as the rest of the chrome instead of borrowing a text glyph.
pub fn chip_with_icon(
    _theme: Theme,
    icon: Icon,
    text: impl Into<String>,
    color: gpui::Hsla,
) -> impl IntoElement {
    div()
        .px_2()
        .py_0p5()
        .rounded_sm()
        .bg(color.opacity(0.14))
        .text_color(color)
        .text_xs()
        .flex()
        .items_center()
        .gap_1()
        .child(icons::icon(icon, 11.0, color))
        .child(text.into())
}

pub fn tag_chip(theme: Theme, text: impl Into<String>) -> impl IntoElement {
    chip(theme, format!("#{}", text.into()), theme.text_dim)
}

pub fn checkbox(
    theme: Theme,
    completed: bool,
    accent: gpui::Hsla,
    priority: Option<gpui::Hsla>,
) -> impl IntoElement {
    let idle_ring = priority.unwrap_or(theme.text_faint.opacity(0.55));
    let hover_ring = priority.unwrap_or(accent);
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(16.))
        .rounded_full()
        .border_1()
        // A priority ring gets a second point of weight so the red/yellow/blue
        // signal reads clearly without making every neutral circle heavy.
        .when(priority.is_some(), |this| this.border_2())
        // The idle ring needs to be visible on a pure white reading surface,
        // where a plain hairline disappears. Hovering pulls it to the accent.
        .border_color(if completed { accent } else { idle_ring })
        .bg(if completed { accent } else { theme.clear })
        .text_color(if completed { theme.raised } else { accent })
        .text_xs()
        .cursor_pointer()
        .hover(|style| {
            if completed {
                style.opacity(0.9)
            } else {
                style.border_color(hover_ring).bg(theme.hover)
            }
        })
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

/// A quiet, borderless creation action for the foot of the sidebar. It keeps
/// the same hit area as the labelled button while letting the rail breathe.
pub fn ghost_button_with_icon(
    theme: Theme,
    icon: Icon,
    label: impl Into<String>,
) -> impl IntoElement {
    let label = label.into();
    div()
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .rounded_md()
        .bg(theme.clear)
        .text_color(theme.text_dim)
        .text_xs()
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover).text_color(theme.text))
        .child(icons::icon(icon, 12.0, theme.text_faint))
        .child(label)
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

/// A destructive action, tinted red so it never reads like a routine button.
pub fn danger_button(theme: Theme, icon: Icon, label: impl Into<String>) -> impl IntoElement {
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
        .border_color(theme.danger)
        .bg(theme.raised)
        .text_color(theme.danger)
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .child(icons::icon(icon, 13.0, theme.danger))
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

/// A macOS-style switch for a boolean setting. The caller owns the click and
/// the label; this is only the 36x20 track and knob, so it can sit at the end
/// of a row without borrowing the button language (and its tick marks).
pub fn switch(theme: Theme, on: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .w(px(36.))
        .h(px(20.))
        .p(px(2.))
        .rounded_full()
        .border_1()
        .border_color(if on { theme.accent } else { theme.border })
        .bg(if on { theme.accent } else { theme.field })
        .when(on, |this| this.justify_end())
        .child(
            div()
                .size(px(14.))
                .rounded_full()
                .border_1()
                .border_color(if on { theme.accent } else { theme.border })
                .bg(theme.raised)
                .shadow_sm(),
        )
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

/// A tooltip waits three seconds before appearing, and a click keeps it quiet
/// until the pointer leaves the button. GPUI's built-in delay is half a
/// second, which reads as "immediately" in a dense toolbar, so the view itself
/// stays empty for the first stretch and only then draws its label.
const TIP_DELAY: Duration = Duration::from_millis(3_000);

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
/// A project or saved-view glyph: a named icon or an emoji.
pub fn icon_spec(spec: &IconSpec, size: f32, color: gpui::Hsla) -> gpui::AnyElement {
    match spec {
        IconSpec::Builtin(icon) => icon_for_project(*icon, size, color).into_any_element(),
        IconSpec::Emoji(value) => div()
            .size(px(size + 2.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(size + 1.0))
            .child(value.clone())
            .into_any_element(),
    }
}

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
        ProjectIcon::Hash => Icon::Hash,
        ProjectIcon::Plane => Icon::Plane,
        ProjectIcon::Apple => Icon::Apple,
        ProjectIcon::Basketball => Icon::Basketball,
        ProjectIcon::BeachBall => Icon::Volleyball,
        ProjectIcon::Cake => Icon::Cake,
        ProjectIcon::BookOpen => Icon::BookOpen,
        ProjectIcon::Garden => Icon::Shovel,
        ProjectIcon::Record => Icon::Disc,
        ProjectIcon::Square => Icon::Square,
        ProjectIcon::Growth => Icon::ChartLine,
        ProjectIcon::Checklist => Icon::ListChecks,
        ProjectIcon::Terminal => Icon::Terminal,
        ProjectIcon::Analytics => Icon::PieChart,
        ProjectIcon::Document => Icon::FileText,
        ProjectIcon::Dog => Icon::Dog,
        ProjectIcon::Money => Icon::DollarSign,
        ProjectIcon::Rupee => Icon::Rupee,
        ProjectIcon::Euro => Icon::Euro,
        ProjectIcon::Pound => Icon::Pound,
        ProjectIcon::Yen => Icon::Yen,
        ProjectIcon::Won => Icon::Won,
        ProjectIcon::Ruble => Icon::Ruble,
        ProjectIcon::Bitcoin => Icon::Bitcoin,
        ProjectIcon::Franc => Icon::Franc,
        ProjectIcon::Lira => Icon::Lira,
        ProjectIcon::Mail => Icon::Mail,
        ProjectIcon::Smile => Icon::Smile,
        ProjectIcon::Flag => Icon::Flagged,
        ProjectIcon::Gift => Icon::Gift,
        ProjectIcon::Support => Icon::Headset,
        ProjectIcon::Flask => Icon::FlaskConical,
        ProjectIcon::Laptop => Icon::Laptop,
        ProjectIcon::Phone => Icon::Smartphone,
        ProjectIcon::Lightbulb => Icon::Lightbulb,
        ProjectIcon::Podcast => Icon::Mic,
        ProjectIcon::Moon => Icon::Moon,
        ProjectIcon::Paintbrush => Icon::Paintbrush,
        ProjectIcon::Palette => Icon::Palette,
        ProjectIcon::Person => Icon::User,
        ProjectIcon::Running => Icon::User,
        ProjectIcon::Team => Icon::Users,
        ProjectIcon::Pill => Icon::Pill,
        ProjectIcon::Pin => Icon::Pin,
        ProjectIcon::Pizza => Icon::Pizza,
        ProjectIcon::Puzzle => Icon::Puzzle,
        ProjectIcon::Study => Icon::GraduationCap,
        ProjectIcon::Sofa => Icon::Sofa,
        ProjectIcon::Sparkles => Icon::Sparkles,
        ProjectIcon::Stethoscope => Icon::Stethoscope,
        ProjectIcon::Sun => Icon::Sun,
        ProjectIcon::Telescope => Icon::Telescope,
        ProjectIcon::Video => Icon::Video,
        ProjectIcon::Clock => Icon::Clock,
    };
    icons::icon(glyph, size, color)
}
