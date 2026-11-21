//! Empty-state illustrations for Today and Forecast.
//!
//! These are built from plain GPUI boxes and the icon set rather than bitmaps,
//! so every stroke follows the active theme. A slow repeat animation bobs the
//! drawing and moves a small accent dot around it; the static shapes stay
//! crisp at any scale.

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use gpui::{Animation, AnimationExt, Hsla, IntoElement, Pixels, div, prelude::*, px};

use super::icons::{self, Icon};
use super::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoodleKind {
    Today,
    Forecast,
}

pub fn doodle(kind: DoodleKind, theme: Theme) -> impl IntoElement {
    let mut drawing = frame();
    drawing = match kind {
        DoodleKind::Today => drawing.child(today(theme)),
        DoodleKind::Forecast => drawing.child(forecast(theme)),
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
    div().relative().w(px(299.)).h(px(224.))
}

/// The illustrations are drawn in a compact 176 x 132 coordinate space and
/// scaled up as one so they keep their proportions on a desktop pane.
fn p(value: f32) -> Pixels {
    px(value * 1.7)
}

fn orbit(theme: Theme, cx: f32, cy: f32, rx: f32, ry: f32, seconds: u64) -> impl IntoElement {
    div()
        .absolute()
        .size(p(5.))
        .rounded_full()
        .bg(theme.accent)
        .with_animation(
            "doodle-orbit",
            Animation::new(Duration::from_secs(seconds)).repeat(),
            move |element, delta| {
                let angle = delta * TAU;
                element
                    .left(p(cx + angle.cos() * rx))
                    .top(p(cy + angle.sin() * ry))
            },
        )
}

fn sparkle(theme: Theme, x: f32, y: f32, size: f32, color: Hsla) -> impl IntoElement {
    div()
        .absolute()
        .left(p(x))
        .top(p(y))
        .child(icons::icon(Icon::Plus, size * 1.7, color))
        .with_animation(
            "doodle-sparkle",
            Animation::new(Duration::from_millis(1500)).repeat(),
            |element, delta| {
                let alpha = 0.25 + 0.75 * (delta * PI).sin().abs();
                element.opacity(alpha)
            },
        )
}

fn today(theme: Theme) -> impl IntoElement {
    frame()
        .child(
            div()
                .absolute()
                .left(p(58.))
                .top(p(14.))
                .child(icons::icon(Icon::Sun, 66. * 1.7, theme.warning)),
        )
        .child(
            div()
                .absolute()
                .left(p(55.))
                .top(p(32.))
                .flex()
                .gap(p(13.))
                .child(div().size(p(4.)).rounded_full().bg(theme.raised))
                .child(div().size(p(4.)).rounded_full().bg(theme.raised)),
        )
        .child(
            div()
                .absolute()
                .left(p(24.))
                .top(p(86.))
                .child(
                    div()
                        .relative()
                        .w(p(42.))
                        .h(p(20.))
                        .child(
                            div()
                                .absolute()
                                .left(p(0.))
                                .top(p(6.))
                                .w(p(38.))
                                .h(p(12.))
                                .rounded_full()
                                .bg(theme.field)
                                .border_1()
                                .border_color(theme.border_soft),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(p(8.))
                                .top(p(0.))
                                .size(p(15.))
                                .rounded_full()
                                .bg(theme.field)
                                .border_1()
                                .border_color(theme.border_soft),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(p(19.))
                                .top(p(3.))
                                .size(p(11.))
                                .rounded_full()
                                .bg(theme.field)
                                .border_1()
                                .border_color(theme.border_soft),
                        )
                        .with_animation(
                            "doodle-cloud",
                            Animation::new(Duration::from_secs(5)).repeat(),
                            |element, delta| {
                                element.left(p((delta * TAU).sin() * 5.))
                            },
                        ),
                ),
        )
        .child(sparkle(theme, 136., 36., 9., theme.accent))
        .child(orbit(theme, 90., 56., 66., 44., 9))
}

fn forecast(theme: Theme) -> impl IntoElement {
    let mut card = div()
        .absolute()
        .left(p(38.))
        .top(p(18.))
        .w(p(100.))
        .h(p(90.))
        .rounded(p(13.))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.border_soft)
        .child(
            div()
                .absolute()
                .left(p(0.))
                .top(p(0.))
                .w(p(100.))
                .h(p(24.))
                .rounded(p(13.))
                .bg(theme.accent_soft),
        )
        .child(
            div()
                .absolute()
                .left(p(17.))
                .top(p(11.))
                .flex()
                .gap(p(5.))
                .child(div().size(p(4.)).rounded_full().bg(theme.accent))
                .child(div().size(p(4.)).rounded_full().bg(theme.accent)),
        );
    for row in 0..3 {
        for column in 0..4 {
            let selected = row == 1 && column == 2;
            card = card.child(
                div()
                    .absolute()
                    .left(p(16. + column as f32 * 18.))
                    .top(p(38. + row as f32 * 15.))
                    .size(p(if selected { 6. } else { 4. }))
                    .rounded_full()
                    .bg(if selected { theme.accent } else { theme.text_faint.opacity(0.45) }),
            );
        }
    }
    card.child(orbit(theme, 88., 64., 70., 46., 10))
}
