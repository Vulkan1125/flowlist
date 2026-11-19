//! Colours and metrics for the Flowlist window.
//!
//! Eleven theme families, each with a light and a dark palette. Every palette
//! pairs an opaque reading surface with translucent chrome for the blur
//! backdrop, so macOS can show the desktop through the sidebar and inspector
//! without putting wallpaper contrast behind task text.

use gpui::{Hsla, rgb, rgba, transparent_black};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 2] = [Appearance::Light, Appearance::Dark];

    pub fn slug(self) -> &'static str {
        match self {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Appearance::Light),
            "dark" => Some(Appearance::Dark),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Appearance::Light => Appearance::Dark,
            Appearance::Dark => Appearance::Light,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Plain,
    Paper,
    Mist,
    Catppuccin,
    Nord,
    Gruvbox,
    Dracula,
    TokyoNight,
    Vesper,
    Graphite,
    Midnight,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 11] = [
        ThemeKind::Plain,
        ThemeKind::Paper,
        ThemeKind::Mist,
        ThemeKind::Catppuccin,
        ThemeKind::Nord,
        ThemeKind::Gruvbox,
        ThemeKind::Dracula,
        ThemeKind::TokyoNight,
        ThemeKind::Vesper,
        ThemeKind::Graphite,
        ThemeKind::Midnight,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            ThemeKind::Plain => "plain",
            ThemeKind::Paper => "paper",
            ThemeKind::Mist => "mist",
            ThemeKind::Catppuccin => "catppuccin",
            ThemeKind::Nord => "nord",
            ThemeKind::Gruvbox => "gruvbox",
            ThemeKind::Dracula => "dracula",
            ThemeKind::TokyoNight => "tokyo-night",
            ThemeKind::Vesper => "vesper",
            ThemeKind::Graphite => "graphite",
            ThemeKind::Midnight => "midnight",
        }
    }

    /// Legacy sessions stored only `"light"` or `"dark"`; keep reading them so
    /// an upgrade never resets someone's choice.
    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "plain" => Some(ThemeKind::Plain),
            "light" | "paper" => Some(ThemeKind::Paper),
            "dark" | "graphite" => Some(ThemeKind::Graphite),
            "mist" => Some(ThemeKind::Mist),
            "catppuccin" | "catppuccin-mocha" => Some(ThemeKind::Catppuccin),
            "nord" => Some(ThemeKind::Nord),
            "gruvbox" => Some(ThemeKind::Gruvbox),
            "dracula" => Some(ThemeKind::Dracula),
            "tokyo-night" | "tokyo" => Some(ThemeKind::TokyoNight),
            "vesper" => Some(ThemeKind::Vesper),
            "midnight" => Some(ThemeKind::Midnight),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Plain => "Plain",
            ThemeKind::Paper => "Paper",
            ThemeKind::Mist => "Mist",
            ThemeKind::Catppuccin => "Catppuccin",
            ThemeKind::Nord => "Nord",
            ThemeKind::Gruvbox => "Gruvbox",
            ThemeKind::Dracula => "Dracula",
            ThemeKind::TokyoNight => "Tokyo Night",
            ThemeKind::Vesper => "Vesper",
            ThemeKind::Graphite => "Graphite",
            ThemeKind::Midnight => "Midnight",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ThemeKind::Plain => "White / ink",
            ThemeKind::Paper => "Warm neutral",
            ThemeKind::Mist => "Cool neutral",
            ThemeKind::Catppuccin => "Latte / Mocha",
            ThemeKind::Nord => "Snow / polar night",
            ThemeKind::Gruvbox => "Retro warm",
            ThemeKind::Dracula => "Alucard / purple night",
            ThemeKind::TokyoNight => "Day / neon city",
            ThemeKind::Vesper => "Ink and amber",
            ThemeKind::Graphite => "Warm grey",
            ThemeKind::Midnight => "Deep blue",
        }
    }

    /// The mode a family opens in when a session only stores its name.
    pub fn default_appearance(self) -> Appearance {
        match self {
            ThemeKind::Plain | ThemeKind::Paper | ThemeKind::Mist => Appearance::Light,
            _ => Appearance::Dark,
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|kind| *kind == self)
            .unwrap_or_default();
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|kind| *kind == self)
            .unwrap_or_default();
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub kind: ThemeKind,
    pub appearance: Appearance,
    /// The reading surface. Opaque so task text never competes with the desktop.
    pub bg: Hsla,
    /// Sidebar, inspector, header and status bar. Translucent over the blur.
    pub glass: Hsla,
    /// Top stop of the chrome gradient: the same tint with a breath of the
    /// theme accent pulled through it. Dia layers a low-alpha wash over its
    /// vibrancy instead of using one flat colour, and that is what keeps a
    /// large pane from reading as grey.
    pub glass_top: Hsla,
    /// Bottom stop: the tint settled back toward the reading surface.
    pub glass_bottom: Hsla,
    /// Cards, menus, toasts and the drag ghost.
    pub raised: Hsla,
    /// Inputs and chips that sit on glass or on the reading surface.
    pub field: Hsla,
    /// Hover wash.
    pub hover: Hsla,
    /// Nothing at all, for idle rows that must not stack another glass tint.
    pub clear: Hsla,
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
}

struct Colors {
    bg: u32,
    glass: u32,
    raised: u32,
    field: u32,
    hover: u32,
    border: u32,
    border_soft: u32,
    text: u32,
    text_dim: u32,
    text_faint: u32,
    accent: u32,
    accent_soft: u32,
    selection: u32,
    danger: u32,
    warning: u32,
    success: u32,
}

/// Mix two packed colours, keeping the alpha of `base`.
fn tint_toward(base: u32, other: u32, weight: f32) -> u32 {
    let channel = |shift: u32| {
        let from = ((base >> shift) & 0xFF) as f32;
        let to = ((other >> shift) & 0xFF) as f32;
        ((from * (1.0 - weight) + to * weight).round() as u32) << shift
    };
    (base & 0xFF) | channel(8) | channel(16) | channel(24)
}

impl Colors {
    fn theme(self, kind: ThemeKind, appearance: Appearance) -> Theme {
        let text: Hsla = rgb(self.text).into();
        Theme {
            kind,
            appearance,
            bg: rgb(self.bg).into(),
            glass: rgba(self.glass).into(),
            glass_top: rgba(tint_toward(self.glass, self.accent, 0.06)).into(),
            glass_bottom: rgba(tint_toward(self.glass, self.bg, 0.05)).into(),
            raised: rgba(self.raised).into(),
            field: rgba(self.field).into(),
            hover: rgba(self.hover).into(),
            clear: transparent_black(),
            border: rgb(self.border).into(),
            border_soft: rgb(self.border_soft).into(),
            text,
            text_dim: rgb(self.text_dim).into(),
            text_faint: rgb(self.text_faint).into(),
            accent: rgb(self.accent).into(),
            accent_soft: rgba(self.accent_soft).into(),
            selection: rgba(self.selection).into(),
            danger: rgb(self.danger).into(),
            warning: rgb(self.warning).into(),
            success: rgb(self.success).into(),
        }
    }
}

impl Theme {
    pub fn new(kind: ThemeKind, appearance: Appearance) -> Self {
        let colors = match (kind, appearance) {
            (ThemeKind::Plain, Appearance::Light) => Colors {
                bg: 0xFFFFFF,
                glass: 0xFFFFFFE8,
                raised: 0xFFFFFFF7,
                field: 0xF7F7F7D9,
                hover: 0x0000000D,
                border: 0xE4E4E4,
                border_soft: 0xF0F0F0,
                text: 0x0A0A0A,
                text_dim: 0x4F4F4F,
                text_faint: 0x707070,
                accent: 0x181818,
                accent_soft: 0x18181824,
                selection: 0x1818181A,
                danger: 0xA61B1B,
                warning: 0x7A5300,
                success: 0x1F6B3C,
            },
            (ThemeKind::Plain, Appearance::Dark) => Colors {
                bg: 0x0B0B0B,
                glass: 0x161616E8,
                raised: 0x1C1C1CF7,
                field: 0xFFFFFF12,
                hover: 0xFFFFFF1A,
                border: 0x333333,
                border_soft: 0x252525,
                text: 0xF7F7F7,
                text_dim: 0xC2C2C2,
                text_faint: 0x919191,
                accent: 0xF2F2F2,
                accent_soft: 0xFFFFFF36,
                selection: 0xFFFFFF30,
                danger: 0xF0B4B4,
                warning: 0xE8C070,
                success: 0x9BD8AB,
            },
            (ThemeKind::Paper, Appearance::Light) => Colors {
                bg: 0xF7F4EE,
                glass: 0xEFECE4E4,
                raised: 0xFFFDF9F5,
                field: 0xFFFFFFA8,
                hover: 0x26241F10,
                border: 0xDBD5C9,
                border_soft: 0xE7E2D8,
                text: 0x27241E,
                text_dim: 0x58524B,
                text_faint: 0x746C63,
                accent: 0x245690,
                accent_soft: 0x2E6FB82B,
                selection: 0x2E6FB81E,
                danger: 0x903933,
                warning: 0x734E13,
                success: 0x305F3C,
            },
            (ThemeKind::Paper, Appearance::Dark) => Colors {
                bg: 0x1B1A18,
                glass: 0x201F1DE2,
                raised: 0x2A2926F5,
                field: 0xFFFFFF0F,
                hover: 0xFFFFFF18,
                border: 0x3B3833,
                border_soft: 0x302D29,
                text: 0xECE8E0,
                text_dim: 0xC8C3BC,
                text_faint: 0xA4A098,
                accent: 0x9CC5F4,
                accent_soft: 0x82B4F040,
                selection: 0x82B4F045,
                danger: 0xEEB3AB,
                warning: 0xE6BB6A,
                success: 0x97CEA5,
            },
            (ThemeKind::Mist, Appearance::Light) => Colors {
                bg: 0xF2F5F7,
                glass: 0xE6EDF2E4,
                raised: 0xFCFDFFF5,
                field: 0xFFFFFFB3,
                hover: 0x1E293310,
                border: 0xD3DCE3,
                border_soft: 0xE0E7EC,
                text: 0x1E2933,
                text_dim: 0x49545F,
                text_faint: 0x606E7C,
                accent: 0x115F59,
                accent_soft: 0x167C7426,
                selection: 0x167C741E,
                danger: 0x8D3B36,
                warning: 0x744D14,
                success: 0x255F45,
            },
            (ThemeKind::Mist, Appearance::Dark) => Colors {
                bg: 0x131922,
                glass: 0x1A2230E2,
                raised: 0x222C3BF5,
                field: 0xFFFFFF0D,
                hover: 0xFFFFFF18,
                border: 0x303B4C,
                border_soft: 0x242E3C,
                text: 0xE6ECF5,
                text_dim: 0xBEC8D1,
                text_faint: 0x9AA4B0,
                accent: 0xADC3FF,
                accent_soft: 0x8BA9FF40,
                selection: 0x8BA9FF45,
                danger: 0xF5B4B4,
                warning: 0xEBC06A,
                success: 0x7ED6A5,
            },
            (ThemeKind::Catppuccin, Appearance::Light) => Colors {
                bg: 0xEFF1F5,
                glass: 0xE6E9EFE4,
                raised: 0xFFFFFFF5,
                field: 0xFFFFFFB3,
                hover: 0x4C4F6910,
                border: 0xCCD0DA,
                border_soft: 0xDCE0E8,
                text: 0x3A3C50,
                text_dim: 0x4D4F63,
                text_faint: 0x66697F,
                accent: 0x0947C5,
                accent_soft: 0x1E66F52B,
                selection: 0x1E66F51E,
                danger: 0xA60C2E,
                warning: 0x744910,
                success: 0x265E1A,
            },
            (ThemeKind::Catppuccin, Appearance::Dark) => Colors {
                bg: 0x1E1E2E,
                glass: 0x181825E2,
                raised: 0x313244F5,
                field: 0xFFFFFF0E,
                hover: 0xFFFFFF18,
                border: 0x45475A,
                border_soft: 0x313244,
                text: 0xCED7F4,
                text_dim: 0xB5BCD1,
                text_faint: 0x969AAD,
                accent: 0x94BBFA,
                accent_soft: 0x89B4FA40,
                selection: 0x89B4FA45,
                danger: 0xF5A2B8,
                warning: 0xF9E2AF,
                success: 0xA6E3A1,
            },
            (ThemeKind::Nord, Appearance::Light) => Colors {
                bg: 0xECEFF4,
                glass: 0xE5E9F0E4,
                raised: 0xFDFEFFF5,
                field: 0xFFFFFFB3,
                hover: 0x2E344010,
                border: 0xD8DEE9,
                border_soft: 0xE5E9F0,
                text: 0x2E3440,
                text_dim: 0x495165,
                text_faint: 0x5E6B85,
                accent: 0x3C5475,
                accent_soft: 0x5E81AC2B,
                selection: 0x5E81AC1E,
                danger: 0x8B3841,
                warning: 0x83402B,
                success: 0x455934,
            },
            (ThemeKind::Nord, Appearance::Dark) => Colors {
                bg: 0x2E3440,
                glass: 0x2E3440E2,
                raised: 0x3B4252F5,
                field: 0xFFFFFF10,
                hover: 0xFFFFFF18,
                border: 0x4C566A,
                border_soft: 0x3B4252,
                text: 0xFCFDFD,
                text_dim: 0xD9DFE9,
                text_faint: 0xB1B8C7,
                accent: 0xC2E0E8,
                accent_soft: 0x88C0D040,
                selection: 0x88C0D045,
                danger: 0xEED5D6,
                warning: 0xF1D8A8,
                success: 0xD1E0C6,
            },
            (ThemeKind::Gruvbox, Appearance::Light) => Colors {
                bg: 0xFBF1C7,
                glass: 0xEBDBB2E4,
                raised: 0xFFF9E6F5,
                field: 0xFFFFFFB3,
                hover: 0x3C383610,
                border: 0xD5C4A1,
                border_soft: 0xE5D5B5,
                text: 0x393534,
                text_dim: 0x4F4844,
                text_faint: 0x6E6158,
                accent: 0x69440C,
                accent_soft: 0xB576142B,
                selection: 0xB576141E,
                danger: 0x9D0006,
                warning: 0x882E03,
                success: 0x514D0A,
            },
            (ThemeKind::Gruvbox, Appearance::Dark) => Colors {
                bg: 0x282828,
                glass: 0x1D2021E2,
                raised: 0x3C3836F5,
                field: 0xFFFFFF0F,
                hover: 0xFFFFFF18,
                border: 0x504945,
                border_soft: 0x3C3836,
                text: 0xEDDEB8,
                text_dim: 0xD5C4A1,
                text_faint: 0xA9A092,
                accent: 0xFEB170,
                accent_soft: 0xFE801940,
                selection: 0xFE801945,
                danger: 0xFDAEA4,
                warning: 0xFABD2F,
                success: 0xC6C929,
            },
            (ThemeKind::Dracula, Appearance::Light) => Colors {
                bg: 0xF8F8F2,
                glass: 0xEFEFE9E4,
                raised: 0xFFFFFBF5,
                field: 0xFFFFFFB3,
                hover: 0x1F1F1F10,
                border: 0xD8D8D0,
                border_soft: 0xE8E8E0,
                text: 0x1F1F1F,
                text_dim: 0x4C4C4C,
                text_faint: 0x6C664B,
                accent: 0x583CC5,
                accent_soft: 0x644AC92B,
                selection: 0x644AC91E,
                danger: 0x9F2E20,
                warning: 0x675410,
                success: 0x12640A,
            },
            (ThemeKind::Dracula, Appearance::Dark) => Colors {
                bg: 0x282A36,
                glass: 0x21222CE2,
                raised: 0x343746F5,
                field: 0xFFFFFF10,
                hover: 0xFFFFFF18,
                border: 0x44475A,
                border_soft: 0x343746,
                text: 0xF8F8F2,
                text_dim: 0xC4C7D3,
                text_faint: 0x98A3C4,
                accent: 0xD4B9FB,
                accent_soft: 0xBD93F940,
                selection: 0xBD93F945,
                danger: 0xFFB0B0,
                warning: 0xF1FA8C,
                success: 0x50FA7B,
            },
            (ThemeKind::TokyoNight, Appearance::Light) => Colors {
                bg: 0xE1E2E7,
                glass: 0xD0D5E3E4,
                raised: 0xF7F8FBF5,
                field: 0xFFFFFFB3,
                hover: 0x3760BF10,
                border: 0xC4C8DA,
                border_soft: 0xD8DCE8,
                text: 0x2A3048,
                text_dim: 0x3F435F,
                text_faint: 0x565C85,
                accent: 0x0F4490,
                accent_soft: 0x2E7DE92B,
                selection: 0x2E7DE91E,
                danger: 0x90072F,
                warning: 0x6D3A00,
                success: 0x3A4C26,
            },
            (ThemeKind::TokyoNight, Appearance::Dark) => Colors {
                bg: 0x1A1B26,
                glass: 0x16161EE2,
                raised: 0x292E42F5,
                field: 0xFFFFFF0D,
                hover: 0xFFFFFF18,
                border: 0x3B4261,
                border_soft: 0x292E42,
                text: 0xCAD3F7,
                text_dim: 0xB1B8D9,
                text_faint: 0x8E96B8,
                accent: 0x96B7F9,
                accent_soft: 0x7AA2F740,
                selection: 0x7AA2F745,
                danger: 0xF99CAD,
                warning: 0xE0AF68,
                success: 0x9ECE6A,
            },
            (ThemeKind::Vesper, Appearance::Light) => Colors {
                bg: 0xFBF6EF,
                glass: 0xF3EDE4E4,
                raised: 0xFFFDFAF5,
                field: 0xFFFFFFB3,
                hover: 0x1A1A1A10,
                border: 0xE4DACC,
                border_soft: 0xEFE7DB,
                text: 0x1A1A1A,
                text_dim: 0x575350,
                text_faint: 0x746D67,
                accent: 0x8C4007,
                accent_soft: 0xB453092B,
                selection: 0xB453091E,
                danger: 0x9A340A,
                warning: 0x7D4A06,
                success: 0x0D605A,
            },
            (ThemeKind::Vesper, Appearance::Dark) => Colors {
                bg: 0x101010,
                glass: 0x161616E2,
                raised: 0x1C1C1CF5,
                field: 0xFFFFFF0E,
                hover: 0xFFFFFF18,
                border: 0x2A2A2A,
                border_soft: 0x1F1F1F,
                text: 0xFFFFFF,
                text_dim: 0xD7D7D7,
                text_faint: 0x999999,
                accent: 0xFFC799,
                accent_soft: 0xFFC79940,
                selection: 0xFFC79945,
                danger: 0xFF9D9D,
                warning: 0xFFC799,
                success: 0x99FFE4,
            },
            (ThemeKind::Graphite, Appearance::Light) => Colors {
                bg: 0xF1F1EF,
                glass: 0xE7E7E4E4,
                raised: 0xFFFFFFF5,
                field: 0xFFFFFFB3,
                hover: 0x20202010,
                border: 0xD5D5D1,
                border_soft: 0xE3E3DF,
                text: 0x232322,
                text_dim: 0x50504D,
                text_faint: 0x6A6A66,
                accent: 0x3F536D,
                accent_soft: 0x4F6B8A2B,
                selection: 0x4F6B8A1E,
                danger: 0x893833,
                warning: 0x6B4C14,
                success: 0x345A42,
            },
            (ThemeKind::Graphite, Appearance::Dark) => Colors {
                bg: 0x1B1A18,
                glass: 0x201F1DE2,
                raised: 0x2A2926F5,
                field: 0xFFFFFF0F,
                hover: 0xFFFFFF18,
                border: 0x3B3833,
                border_soft: 0x302D29,
                text: 0xECE8E0,
                text_dim: 0xC8C3BC,
                text_faint: 0xA4A098,
                accent: 0x9CC5F4,
                accent_soft: 0x82B4F040,
                selection: 0x82B4F045,
                danger: 0xEEB3AB,
                warning: 0xE6BB6A,
                success: 0x97CEA5,
            },
            (ThemeKind::Midnight, Appearance::Light) => Colors {
                bg: 0xEEF2F9,
                glass: 0xE4EAF5E4,
                raised: 0xFCFDFFF5,
                field: 0xFFFFFFB3,
                hover: 0x1B243310,
                border: 0xD2DAE8,
                border_soft: 0xE0E7F2,
                text: 0x1B2433,
                text_dim: 0x465169,
                text_faint: 0x5E6C86,
                accent: 0x2545C6,
                accent_soft: 0x3B5BDB2B,
                selection: 0x3B5BDB1E,
                danger: 0x982E22,
                warning: 0x754A0C,
                success: 0x225D45,
            },
            (ThemeKind::Midnight, Appearance::Dark) => Colors {
                bg: 0x131922,
                glass: 0x1A2230E2,
                raised: 0x222C3BF5,
                field: 0xFFFFFF0D,
                hover: 0xFFFFFF18,
                border: 0x303B4C,
                border_soft: 0x242E3C,
                text: 0xE6ECF5,
                text_dim: 0xBEC8D1,
                text_faint: 0x9AA4B0,
                accent: 0xADC3FF,
                accent_soft: 0x8BA9FF40,
                selection: 0x8BA9FF45,
                danger: 0xF5B4B4,
                warning: 0xEBC06A,
                success: 0x7ED6A5,
            },
        };

        colors.theme(kind, appearance)
    }

    pub fn kind(&self) -> ThemeKind {
        self.kind
    }

    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// The translucent chrome fill. Panes run this gradient instead of a flat
    /// tint so the glass has a direction, the way Dia's window background
    /// pairs a base tint with an overlay gradient.
    pub fn glass_fill(&self, angle: f32) -> gpui::Background {
        gpui::linear_gradient(
            angle,
            gpui::linear_color_stop(self.glass_top, 0.0),
            gpui::linear_color_stop(self.glass_bottom, 1.0),
        )
    }

    /// Light/dark within the current theme family.
    pub fn toggled(self) -> Self {
        Self::new(self.kind, self.appearance.toggled())
    }

    /// The next theme family, keeping the current light/dark mode.
    pub fn next_theme(self) -> Self {
        Self::new(self.kind.next(), self.appearance)
    }

    /// The previous theme family, keeping the current light/dark mode.
    pub fn previous_theme(self) -> Self {
        Self::new(self.kind.previous(), self.appearance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Rgba;

    fn channels(color: Hsla) -> (f32, f32, f32) {
        let rgba = Rgba::from(color);
        (rgba.r, rgba.g, rgba.b)
    }

    fn linear(value: f32) -> f32 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: Hsla) -> f32 {
        let (r, g, b) = channels(color);
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    }

    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let (lighter, darker) = if luminance(a) >= luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Paint a translucent colour over an opaque one, the way the window
    /// composites chrome over the blurred desktop.
    fn over(top: Hsla, bottom: Hsla) -> Hsla {
        let (tr, tg, tb) = channels(top);
        let (br, bg, bb) = channels(bottom);
        let a = top.a;
        let mix = |top: f32, bottom: f32| ((top * a + bottom * (1.0 - a)) * 255.0) as u32;
        gpui::rgba((mix(tr, br) << 24) | (mix(tg, bg) << 16) | (mix(tb, bb) << 8) | 0xFF).into()
    }

    fn worst_contrast(on: Hsla, backgrounds: &[Hsla]) -> f32 {
        backgrounds
            .iter()
            .map(|background| contrast(on, *background))
            .fold(f32::INFINITY, f32::min)
    }

    #[test]
    fn legacy_sessions_still_open() {
        assert_eq!(ThemeKind::from_slug("light"), Some(ThemeKind::Paper));
        assert_eq!(ThemeKind::from_slug("dark"), Some(ThemeKind::Graphite));
        assert_eq!(ThemeKind::from_slug("plain"), Some(ThemeKind::Plain));
        assert_eq!(ThemeKind::from_slug("midnight"), Some(ThemeKind::Midnight));
        assert_eq!(ThemeKind::from_slug("sepia"), None);
    }

    #[test]
    fn popular_theme_slugs_round_trip() {
        for kind in [
            ThemeKind::Catppuccin,
            ThemeKind::Nord,
            ThemeKind::Gruvbox,
            ThemeKind::Dracula,
            ThemeKind::TokyoNight,
            ThemeKind::Vesper,
        ] {
            assert_eq!(ThemeKind::from_slug(kind.slug()), Some(kind));
        }
        assert_eq!(
            ThemeKind::from_slug("catppuccin-mocha"),
            Some(ThemeKind::Catppuccin)
        );
    }

    #[test]
    fn every_theme_has_both_modes() {
        for kind in ThemeKind::ALL {
            for appearance in Appearance::ALL {
                let theme = Theme::new(kind, appearance);
                assert_eq!(theme.kind(), kind);
                assert_eq!(theme.appearance(), appearance);
                assert_eq!(theme.glass.a < 1.0, true);
            }
        }
    }

    /// The gradient is a wash, not a second surface: each stop must keep the
    /// tint's opacity so `glass_fill` never stealthily makes chrome more
    /// transparent than the contrast audit approved.
    #[test]
    fn chrome_gradient_keeps_the_tint_opacity() {
        for kind in ThemeKind::ALL {
            for appearance in Appearance::ALL {
                let theme = Theme::new(kind, appearance);
                assert_eq!(theme.glass_top.a, theme.glass.a);
                assert_eq!(theme.glass_bottom.a, theme.glass.a);
                assert_ne!(theme.glass_top, theme.glass_bottom);
            }
        }
    }

    /// Legibility is part of the theme contract, not a tuning suggestion.
    ///
    /// Chrome is translucent, so every foreground colour is checked against the
    /// tint over a pure black (light themes) or pure white (dark themes)
    /// wallpaper — the worst case a blurred desktop can produce. If these
    /// numbers ever drop, the palette is wrong, not the test.
    #[test]
    fn every_palette_stays_legible_on_every_surface() {
        for kind in ThemeKind::ALL {
            for appearance in Appearance::ALL {
                let theme = Theme::new(kind, appearance);
                let wallpaper = match appearance {
                    Appearance::Light => gpui::black(),
                    Appearance::Dark => gpui::white(),
                };
                let glass = over(theme.glass, wallpaper);
                let glass_top = over(theme.glass_top, wallpaper);
                let glass_bottom = over(theme.glass_bottom, wallpaper);
                let raised = over(theme.raised, theme.bg);
                let field = over(theme.field, theme.bg);
                let selection = over(theme.selection, theme.bg);
                let label = match appearance {
                    Appearance::Light => over(gpui::white(), theme.bg),
                    Appearance::Dark => raised,
                };
                let surfaces = [
                    theme.bg,
                    raised,
                    field,
                    selection,
                    glass,
                    glass_top,
                    glass_bottom,
                ];
                let context = format!("{kind:?} {appearance:?}");

                for (name, color) in [
                    ("text", theme.text),
                    ("text_dim", theme.text_dim),
                    ("text_faint", theme.text_faint),
                    ("accent", theme.accent),
                    ("danger", theme.danger),
                    ("warning", theme.warning),
                    ("success", theme.success),
                ] {
                    let worst = worst_contrast(color, &surfaces);
                    let floor = match name {
                        "text" => 4.5,
                        "text_dim" => 4.5,
                        "text_faint" => 3.0,
                        _ => 4.4,
                    };
                    assert!(
                        worst >= floor,
                        "{context}: {name} reaches only {worst:.2}:1 on the worst surface"
                    );
                }

                assert!(
                    contrast(theme.text, theme.bg) >= 7.0,
                    "{context}: body text is below 7:1 on the reading surface"
                );
                assert!(
                    contrast(theme.text_dim, theme.bg) >= 4.5,
                    "{context}: secondary text is below 4.5:1 on the reading surface"
                );
                assert!(
                    contrast(theme.text_faint, theme.bg) >= 3.0,
                    "{context}: tertiary text is below 3:1 on the reading surface"
                );
                assert!(
                    contrast(label, theme.accent) >= 4.4,
                    "{context}: primary button label is unreadable on the accent fill"
                );
                assert!(
                    theme.glass.a <= 0.95,
                    "{context}: the glass is so opaque that the blur is gone"
                );
            }
        }
    }

    #[test]
    fn cycling_visits_every_family_and_wraps() {
        let mut kind = ThemeKind::Plain;
        let mut seen = Vec::new();
        for _ in 0..ThemeKind::ALL.len() {
            seen.push(kind);
            kind = kind.next();
        }
        assert_eq!(seen, ThemeKind::ALL);
        assert_eq!(kind, ThemeKind::Plain);
        assert_eq!(ThemeKind::Plain.previous(), ThemeKind::Midnight);
    }
}
