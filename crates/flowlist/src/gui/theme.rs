//! Colours and metrics for the Flowlist window.
//!
//! Two hand-tuned palettes (paper and graphite) keep the interface calm: one
//! accent colour, warm greys and a single warm red for destructive actions.

use gpui::{Hsla, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

impl Mode {
    pub fn slug(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Mode::Light),
            "dark" => Some(Mode::Dark),
            _ => None,
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub mode: Mode,
    /// The window shell.
    pub bg: Hsla,
    /// The sidebar.
    pub sidebar: Hsla,
    /// Cards, inputs and raised surfaces.
    pub raised: Hsla,
    /// Hover wash.
    pub hover: Hsla,
    pub border: Hsla,
    pub border_soft: Hsla,
    pub text: Hsla,
    pub text_dim: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub accent_soft: Hsla,
    pub selection: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub shadow: Hsla,
}

impl Theme {
    pub fn new(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self {
                mode,
                bg: rgb(0xF6F5F2).into(),
                sidebar: rgb(0xEDEBE6).into(),
                raised: rgb(0xFFFFFF).into(),
                hover: rgb(0x0000000A).into(),
                border: rgb(0xDCD8D0).into(),
                border_soft: rgb(0xE7E4DD).into(),
                text: rgb(0x26241F).into(),
                text_dim: rgb(0x6B665C).into(),
                text_faint: rgb(0x9A948A).into(),
                accent: rgb(0x2F6FB7).into(),
                accent_soft: rgb(0x2F6FB71F).into(),
                selection: rgb(0x2F6FB724).into(),
                danger: rgb(0xB84A3F).into(),
                warning: rgb(0xB07A1E).into(),
                success: rgb(0x3E7D4E).into(),
                shadow: rgb(0x0000000F).into(),
            },
            Mode::Dark => Self {
                mode,
                bg: rgb(0x181715).into(),
                sidebar: rgb(0x131211).into(),
                raised: rgb(0x232220).into(),
                hover: rgb(0xFFFFFF0A).into(),
                border: rgb(0x35332F).into(),
                border_soft: rgb(0x2A2825).into(),
                text: rgb(0xE9E6DF).into(),
                text_dim: rgb(0xA39D92).into(),
                text_faint: rgb(0x736E65).into(),
                accent: rgb(0x7DB2EE).into(),
                accent_soft: rgb(0x7DB2EE1F).into(),
                selection: rgb(0x7DB2EE26).into(),
                danger: rgb(0xE0857A).into(),
                warning: rgb(0xE3B563).into(),
                success: rgb(0x8CC79A).into(),
                shadow: rgb(0x00000033).into(),
            },
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}
