//! A project or saved-view glyph: either one of the named icons or a single
//! emoji. Stored as a plain string so old libraries and new emoji choices use
//! the same field.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use unicode_segmentation::UnicodeSegmentation;

use super::ProjectIcon;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconSpec {
    Builtin(ProjectIcon),
    Emoji(String),
}

impl Default for IconSpec {
    fn default() -> Self {
        Self::Builtin(ProjectIcon::default())
    }
}

impl IconSpec {
    pub fn builtin(icon: ProjectIcon) -> Self {
        Self::Builtin(icon)
    }

    /// Parse a CLI or JSON value: a known icon slug, or one emoji.
    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("icon cannot be empty".to_string());
        }
        if let Some(icon) = ProjectIcon::from_slug(value) {
            return Ok(Self::Builtin(icon));
        }
        if is_single_emoji(value) {
            return Ok(Self::Emoji(value.to_string()));
        }
        Err(format!("unknown icon “{value}”"))
    }

    /// Load a stored value, falling back to the default rather than failing a
    /// whole library over one bad glyph.
    pub fn from_stored(value: &str) -> Self {
        Self::parse(value).unwrap_or_default()
    }

    pub fn builtin_icon(&self) -> Option<ProjectIcon> {
        match self {
            Self::Builtin(icon) => Some(*icon),
            Self::Emoji(_) => None,
        }
    }

    pub fn emoji(&self) -> Option<&str> {
        match self {
            Self::Builtin(_) => None,
            Self::Emoji(value) => Some(value),
        }
    }

    pub fn stored(&self) -> String {
        match self {
            Self::Builtin(icon) => icon.slug().to_string(),
            Self::Emoji(value) => value.clone(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Builtin(icon) => icon.label().to_string(),
            Self::Emoji(value) => value.clone(),
        }
    }
}

impl Serialize for IconSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Builtin(icon) => serializer.serialize_str(icon.slug()),
            Self::Emoji(value) => serializer.serialize_str(value),
        }
    }
}

impl<'de> Deserialize<'de> for IconSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_stored(&value))
    }
}

/// True for one grapheme cluster that contains an emoji or symbol.
pub fn is_single_emoji(value: &str) -> bool {
    let mut graphemes = value.graphemes(true);
    let Some(grapheme) = graphemes.next() else {
        return false;
    };
    if graphemes.next().is_some() {
        return false;
    }
    grapheme.chars().any(|ch| {
        let code = ch as u32;
        matches!(
            code,
            0x1F000..=0x1FAFF
                | 0x2600..=0x27BF
                | 0x2B00..=0x2BFF
                | 0x2190..=0x21FF
                | 0x2300..=0x23FF
                | 0x25A0..=0x25FF
                | 0xFE0F
                | 0x20E3
        )
    })
}

#[cfg(test)]
mod tests {
    use super::IconSpec;
    use crate::model::ProjectIcon;

    #[test]
    fn named_icons_and_emojis_share_one_string_field() {
        assert_eq!(
            serde_json::from_str::<IconSpec>("\"star\"").unwrap(),
            IconSpec::Builtin(ProjectIcon::Star)
        );
        assert_eq!(
            serde_json::from_str::<IconSpec>("\"🏃\"").unwrap(),
            IconSpec::Emoji("🏃".to_string())
        );
        assert_eq!(
            serde_json::to_string(&IconSpec::Builtin(ProjectIcon::Star)).unwrap(),
            "\"star\""
        );
        assert_eq!(
            serde_json::to_string(&IconSpec::Emoji("🏃".to_string())).unwrap(),
            "\"🏃\""
        );
    }

    #[test]
    fn the_retired_running_figure_still_loads() {
        assert_eq!(
            IconSpec::from_stored("running"),
            IconSpec::Builtin(ProjectIcon::Running)
        );
    }
}
