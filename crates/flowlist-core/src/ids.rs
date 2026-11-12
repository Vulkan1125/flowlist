//! Stable entity identifiers.
//!
//! Flowlist uses ULIDs: they are time-ordered (so new items sort naturally),
//! globally unique, and safe to expose over the CLI and MCP. Every id can be
//! matched by an unambiguous prefix, which keeps command-line usage pleasant.

use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

macro_rules! id_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(Ulid::generate().to_string())
            }

            pub fn from_string(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Short form shown in human-oriented output. Prefix resolution
            /// accepts any unambiguous prefix, including this one.
            pub fn short(&self) -> &str {
                let end = self.0.len().min(8);
                &self.0[..end]
            }

            pub fn starts_with(&self, prefix: &str) -> bool {
                self.0.starts_with(prefix)
            }

            pub fn label() -> &'static str {
                $label
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

id_type!(TaskId, "task");
id_type!(ProjectId, "project");
id_type!(FolderId, "folder");
id_type!(TagId, "tag");
id_type!(PerspectiveId, "perspective");

/// A node that can live inside a folder or at the root of the library.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ContainerId {
    Folder(FolderId),
    Project(ProjectId),
}

impl ContainerId {
    pub fn short(&self) -> &str {
        match self {
            ContainerId::Folder(id) => id.short(),
            ContainerId::Project(id) => id.short(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            ContainerId::Folder(id) => id.to_string(),
            ContainerId::Project(id) => id.to_string(),
        }
    }
}

impl fmt::Display for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerId::Folder(id) => write!(f, "{id}"),
            ContainerId::Project(id) => write!(f, "{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sortable_and_prefixable() {
        let a = TaskId::new();
        let b = TaskId::new();
        assert_ne!(a, b);
        assert_eq!(a.short().len(), 8);
        assert!(a.starts_with(a.short()));
        assert_eq!(a.to_string(), a.as_str());
    }

    #[test]
    fn container_roundtrips_through_json() {
        let c = ContainerId::Project(ProjectId::from("abc"));
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("project"));
        let back: ContainerId = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
