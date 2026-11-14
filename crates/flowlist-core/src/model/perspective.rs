//! Perspectives: the built-in views plus user-defined saved queries.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ids::{FolderId, PerspectiveId, ProjectId, TagId};
use crate::model::Priority;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinPerspective {
    Inbox,
    Projects,
    Forecast,
    Flagged,
    Review,
    Tags,
    Completed,
    Changed,
    Nearby,
    Trash,
}

impl BuiltinPerspective {
    pub const ALL: [BuiltinPerspective; 10] = [
        BuiltinPerspective::Inbox,
        BuiltinPerspective::Projects,
        BuiltinPerspective::Forecast,
        BuiltinPerspective::Flagged,
        BuiltinPerspective::Review,
        BuiltinPerspective::Tags,
        BuiltinPerspective::Completed,
        BuiltinPerspective::Changed,
        BuiltinPerspective::Nearby,
        BuiltinPerspective::Trash,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            BuiltinPerspective::Inbox => "inbox",
            BuiltinPerspective::Projects => "projects",
            BuiltinPerspective::Forecast => "forecast",
            BuiltinPerspective::Flagged => "flagged",
            BuiltinPerspective::Review => "review",
            BuiltinPerspective::Tags => "tags",
            BuiltinPerspective::Completed => "completed",
            BuiltinPerspective::Changed => "changed",
            BuiltinPerspective::Nearby => "nearby",
            BuiltinPerspective::Trash => "trash",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BuiltinPerspective::Inbox => "Inbox",
            BuiltinPerspective::Projects => "Projects",
            BuiltinPerspective::Forecast => "Forecast",
            BuiltinPerspective::Flagged => "Flagged",
            BuiltinPerspective::Review => "Review",
            BuiltinPerspective::Tags => "Tags",
            BuiltinPerspective::Completed => "Completed",
            BuiltinPerspective::Changed => "Changed",
            BuiltinPerspective::Nearby => "Nearby",
            BuiltinPerspective::Trash => "Trash",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|perspective| perspective.slug() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    None,
    Project,
    Folder,
    Tag,
    DueDate,
    DeferDate,
    Flagged,
    Estimate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Manual,
    Due,
    Defer,
    Name,
    Flagged,
    Estimate,
    Created,
    CompletedAt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DueWindow {
    Overdue,
    Today,
    Tomorrow,
    Next7Days,
    Next30Days,
    NoDate,
    On(NaiveDate),
    Before(NaiveDate),
    After(NaiveDate),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Any,
    Available,
    Blocked,
    Completed,
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    Project {
        id: ProjectId,
    },
    Folder {
        id: FolderId,
        include_children: bool,
    },
    Tag {
        id: TagId,
        include_children: bool,
    },
    InInbox,
    Flagged {
        value: bool,
    },
    Priority {
        value: Priority,
    },
    Status {
        value: TaskStatus,
    },
    Due {
        window: DueWindow,
    },
    Deferred {
        window: DueWindow,
    },
    HasRepeat {
        value: bool,
    },
    Estimate {
        min_minutes: Option<u32>,
        max_minutes: Option<u32>,
    },
    Text {
        value: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterNode {
    All(Vec<FilterNode>),
    Any(Vec<FilterNode>),
    Not(Box<FilterNode>),
    Condition(Condition),
}

// Serialized by hand: an internally-tagged recursive enum makes serde's typed
// serializer monomorphize forever. The JSON shape is still friendly:
// `{"type":"all","nodes":[...]}`.
impl Serialize for FilterNode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        match self {
            FilterNode::All(nodes) => {
                map.serialize_entry("type", "all")?;
                map.serialize_entry("nodes", nodes)?;
            }
            FilterNode::Any(nodes) => {
                map.serialize_entry("type", "any")?;
                map.serialize_entry("nodes", nodes)?;
            }
            FilterNode::Not(node) => {
                map.serialize_entry("type", "not")?;
                map.serialize_entry("node", node)?;
            }
            FilterNode::Condition(condition) => {
                map.serialize_entry("type", "condition")?;
                map.serialize_entry("condition", condition)?;
            }
        }
        map.end()
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FilterNodeRepr {
    All {
        #[serde(default)]
        nodes: Vec<FilterNode>,
    },
    Any {
        #[serde(default)]
        nodes: Vec<FilterNode>,
    },
    Not {
        node: Box<FilterNode>,
    },
    Condition {
        condition: Condition,
    },
}

impl<'de> Deserialize<'de> for FilterNode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let repr = FilterNodeRepr::deserialize(deserializer)?;
        Ok(match repr {
            FilterNodeRepr::All { nodes } => FilterNode::All(nodes),
            FilterNodeRepr::Any { nodes } => FilterNode::Any(nodes),
            FilterNodeRepr::Not { node } => FilterNode::Not(node),
            FilterNodeRepr::Condition { condition } => FilterNode::Condition(condition),
        })
    }
}

impl Default for FilterNode {
    fn default() -> Self {
        FilterNode::All(Vec::new())
    }
}

impl FilterNode {
    pub fn all(nodes: Vec<FilterNode>) -> Self {
        FilterNode::All(nodes)
    }

    pub fn any(nodes: Vec<FilterNode>) -> Self {
        FilterNode::Any(nodes)
    }

    pub fn condition(condition: Condition) -> Self {
        FilterNode::Condition(condition)
    }

    /// All of these conditions must match. An empty list matches everything.
    pub fn all_conditions(conditions: Vec<Condition>) -> Self {
        FilterNode::All(conditions.into_iter().map(FilterNode::Condition).collect())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            FilterNode::All(nodes) | FilterNode::Any(nodes) => nodes.is_empty(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Perspective {
    pub id: PerspectiveId,
    pub name: String,
    #[serde(default)]
    pub filter: FilterNode,
    #[serde(default = "default_group")]
    pub group_by: GroupBy,
    #[serde(default)]
    pub sort: SortBy,
    #[serde(default)]
    pub show_completed: bool,
    #[serde(default)]
    pub position: u32,
}

fn default_group() -> GroupBy {
    GroupBy::None
}

impl Perspective {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PerspectiveId::new(),
            name: name.into(),
            filter: FilterNode::All(Vec::new()),
            group_by: GroupBy::None,
            sort: SortBy::Manual,
            show_completed: false,
            position: 0,
        }
    }
}
