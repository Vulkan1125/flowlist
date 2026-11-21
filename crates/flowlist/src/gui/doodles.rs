//! Hand-drawn empty-state illustrations.
//!
//! These are built from plain GPUI boxes and the icon set rather than bitmaps,
//! so every stroke follows the active theme. A slow repeat animation bobs the
//! whole drawing and moves a small accent dot around it; the static shapes
//! stay crisp at any scale.

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use gpui::{Animation, AnimationExt, IntoElement, div, prelude::*, px};

use super::icons::{self, Icon};
use super::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoodleKind {
    Inbox,
    Today,
    Forecast,
    Folder,
    Completed,
    Search,
}

pub fn doodle(kind: DoodleKind, theme: Theme) -> impl IntoElement {
    let mut drawing = frame();
    drawing = match kind {
        DoodleKind::Inbox => drawing.child(inbox(theme)),
        DoodleKind::Today => drawing.child(today(theme)),
        DoodleKind::Forecast => drawing.child(forecast(theme)),
        DoodleKind::Folder => drawing.child(folder(theme)),
        DoodleKind::Completed => drawing.child(completed(theme)),
        DoodleKind::Search => drawing.child(search(theme)),
    };
    drawing.with_animation(
        "empty-doodle-bob",
        Animation::new(Duration::from_secs(4)).repeat(),
        |element, delta| {
            let offset = (delta * TAU).sin() * 3.0;
            element.relative().top(px(offset))
        },
    )
}

fn frame() -> gpui::Div {
    div().relative().w(px(176.)).h(px(132.))
}

fn orbit(theme: Theme, cx: f32, cy: f32, rx: f32, ry: f32, seconds: u64) -> impl IntoElement {
    div()
        .absolute()
        .size(px(5.))
        .rounded_full()
        .bg(theme.accent)
        .with_animation(
            "doodle-orbit",
            Animation::new(Duration::from_secs(seconds)).repeat(),
            move |element, delta| {
                let angle = delta * TAU;
                element
                    .left(px(cx + angle.cos() * rx))
                    .top(px(cy + angle.sin() * ry))
            },
        )
}

fn sparkle(theme: Theme, x: f32, y: f32, size: f32, color: gpui::Hsla) -> impl IntoElement {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .child(icons::icon(Icon::Plus, size, color))
        .with_animation(
            "doodle-sparkle",
            Animation::new(Duration::from_millis(1500)).repeat(),
            |element, delta| {
                let alpha = 0.25 + 0.75 * (delta * PI).sin().abs();
                element.opacity(alpha)
            },
        )
}

fn bar(theme: Theme, x: f32, y: f32, width: f32) -> impl IntoElement {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .h(px(4.))
        .rounded_full()
        .bg(theme.border_soft)
}

fn inbox(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(px(40.))
                .top(px(16.))
                .w(px(96.))
                .h(px(92.))
                .rounded(px(14.))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border_soft),
        )
        .child(bar(theme, 56., 36., 58.))
        .child(bar(theme, 56., 50., 44.))
        .child(
            div()
                .absolute()
                .left(px(52.))
                .top(px(54.))
                .child(icons::icon(Icon::Check, 44., theme.accent)),
        )
        .child(sparkle(theme, 140., 20., 10., theme.warning))
        .child(sparkle(theme, 28., 96., 8., theme.accent))
        .child(orbit(theme, 88., 70., 72., 48., 11))
}

fn today(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(px(58.))
                .top(px(14.))
                .child(icons::icon(Icon::Sun, 66., theme.warning)),
        )
        .child(
            div()
                .absolute()
                .left(px(55.))
                .top(px(32.))
                .flex()
                .gap(px(13.))
                .child(div().size(px(4.)).rounded_full().bg(theme.raised))
                .child(div().size(px(4.)).rounded_full().bg(theme.raised)),
        )
        .child(
            div().absolute().left(px(24.)).top(px(86.)).child(
                div()
                    .relative()
                    .w(px(42.))
                    .h(px(20.))
                    .child(
                        div()
                            .absolute()
                            .left(px(0.))
                            .top(px(6.))
                            .w(px(38.))
                            .h(px(12.))
                            .rounded_full()
                            .bg(theme.field)
                            .border_1()
                            .border_color(theme.border_soft),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(8.))
                            .top(px(0.))
                            .size(px(15.))
                            .rounded_full()
                            .bg(theme.field)
                            .border_1()
                            .border_color(theme.border_soft),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(19.))
                            .top(px(3.))
                            .size(px(11.))
                            .rounded_full()
                            .bg(theme.field)
                            .border_1()
                            .border_color(theme.border_soft),
                    )
                    .with_animation(
                        "doodle-cloud",
                        Animation::new(Duration::from_secs(5)).repeat(),
                        |element, delta| element.left(px((delta * TAU).sin() * 5.)),
                    ),
            ),
        )
        .child(sparkle(theme, 136., 36., 9., theme.accent))
        .child(orbit(theme, 90., 56., 66., 44., 9))
}

fn forecast(theme: Theme) -> impl IntoElement {
    let mut card = div()
        .absolute()
        .left(px(38.))
        .top(px(18.))
        .w(px(100.))
        .h(px(90.))
        .rounded(px(13.))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border_soft)
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .w(px(100.))
                .h(px(24.))
                .rounded(px(13.))
                .bg(theme.accent_soft),
        )
        .child(
            div()
                .absolute()
                .left(px(17.))
                .top(px(11.))
                .flex()
                .gap(px(5.))
                .child(div().size(px(4.)).rounded_full().bg(theme.accent))
                .child(div().size(px(4.)).rounded_full().bg(theme.accent)),
        );
    for row in 0..3 {
        for column in 0..4 {
            let selected = row == 1 && column == 2;
            card = card.child(
                div()
                    .absolute()
                    .left(px(16. + column as f32 * 18.))
                    .top(px(38. + row as f32 * 15.))
                    .size(px(if selected { 6. } else { 4. }))
                    .rounded_full()
                    .bg(if selected {
                        theme.accent
                    } else {
                        theme.text_faint.opacity(0.45)
                    }),
            );
        }
    }
    card.child(orbit(theme, 88., 64., 70., 46., 10))
}

fn folder(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(px(36.))
                .top(px(38.))
                .child(icons::icon(Icon::Folder, 82., theme.accent)),
        )
        .child(
            div().absolute().left(px(96.)).top(px(18.)).child(
                div()
                    .relative()
                    .w(px(46.))
                    .h(px(36.))
                    .rounded(px(6.))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border_soft)
                    .child(bar(theme, 8., 11., 28.))
                    .child(bar(theme, 8., 20., 20.))
                    .with_animation(
                        "doodle-page",
                        Animation::new(Duration::from_secs(5)).repeat(),
                        |element, delta| {
                            element
                                .left(px((delta * TAU).sin() * 5.))
                                .top(px((delta * TAU).cos() * 3.))
                        },
                    ),
            ),
        )
        .child(sparkle(theme, 30., 24., 8., theme.warning))
        .child(orbit(theme, 88., 66., 74., 48., 12))
}

fn completed(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(px(44.))
                .top(px(22.))
                .w(px(88.))
                .h(px(80.))
                .rounded(px(14.))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border_soft),
        )
        .child(
            div()
                .absolute()
                .left(px(60.))
                .top(px(48.))
                .child(icons::icon(Icon::Check, 48., theme.success)),
        )
        .child(sparkle(theme, 138., 24., 9., theme.success))
        .child(orbit(theme, 88., 66., 70., 47., 10))
}

fn search(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(px(52.))
                .top(px(26.))
                .child(icons::icon(Icon::Search, 64., theme.accent)),
        )
        .child(sparkle(theme, 132., 28., 9., theme.warning))
        .child(sparkle(theme, 34., 94., 7., theme.accent))
        .child(orbit(theme, 90., 60., 70., 46., 9))
}
