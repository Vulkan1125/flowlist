//! Perspectives: the built-in views plus user-defined saved queries.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ids::{FolderId, PerspectiveId, ProjectId, TagId};
use crate::model::{IconSpec, Priority};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinPerspective {
    Inbox,
    Today,
    Projects,
    Forecast,
    Flagged,
    Review,
    Tags,
    Completed,
    Archive,
    Changed,
    Attachments,
    Fullscreen,
    Nearby,
    Trash,
}

impl BuiltinPerspective {
    /// The rows offered in the sidebar and Settings matrix. `Fullscreen` is
    /// deliberately absent: it is a transient list opened by a reminder's
    /// View all action, not a permanent perspective.
    pub const ALL: [BuiltinPerspective; 13] = [
        BuiltinPerspective::Inbox,
        BuiltinPerspective::Today,
        BuiltinPerspective::Projects,
        BuiltinPerspective::Forecast,
        BuiltinPerspective::Flagged,
        BuiltinPerspective::Review,
        BuiltinPerspective::Tags,
        BuiltinPerspective::Completed,
        BuiltinPerspective::Archive,
        BuiltinPerspective::Changed,
        BuiltinPerspective::Attachments,
        BuiltinPerspective::Nearby,
        BuiltinPerspective::Trash,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            BuiltinPerspective::Inbox => "inbox",
            BuiltinPerspective::Today => "today",
            BuiltinPerspective::Projects => "projects",
            BuiltinPerspective::Forecast => "forecast",
            BuiltinPerspective::Flagged => "flagged",
            BuiltinPerspective::Review => "review",
            BuiltinPerspective::Tags => "tags",
            BuiltinPerspective::Completed => "completed",
            BuiltinPerspective::Archive => "archive",
            BuiltinPerspective::Changed => "changed",
            BuiltinPerspective::Attachments => "attachments",
            BuiltinPerspective::Fullscreen => "fullscreen",
            BuiltinPerspective::Nearby => "nearby",
            BuiltinPerspective::Trash => "trash",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BuiltinPerspective::Inbox => "Inbox",
            BuiltinPerspective::Today => "Today",
            BuiltinPerspective::Projects => "Projects",
            BuiltinPerspective::Forecast => "Forecast",
            BuiltinPerspective::Flagged => "Flagged",
            BuiltinPerspective::Review => "Review",
            BuiltinPerspective::Tags => "Tags",
            BuiltinPerspective::Completed => "Completed",
            BuiltinPerspective::Archive => "Archive",
            BuiltinPerspective::Changed => "Changed",
            BuiltinPerspective::Attachments => "Attachments",
            BuiltinPerspective::Fullscreen => "Full-screen",
            BuiltinPerspective::Nearby => "Nearby",
            BuiltinPerspective::Trash => "Trash",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        // The transient Full-screen list is valid in a saved session even
        // though it never appears in the sidebar.
        if value == "fullscreen" {
            return Some(Self::Fullscreen);
        }
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
    PlannedDate,
    CompletedDate,
    DroppedDate,
    AddedDate,
    ChangedDate,
    NextReview,
    TagsCombined,
    Flagged,
    Estimate,
}

impl GroupBy {
    pub fn slug(self) -> &'static str {
        match self {
            GroupBy::None => "none",
            GroupBy::Project => "project",
            GroupBy::Folder => "folder",
            GroupBy::Tag => "tag",
            GroupBy::DueDate => "due_date",
            GroupBy::DeferDate => "defer_date",
            GroupBy::PlannedDate => "planned_date",
            GroupBy::CompletedDate => "completed_date",
            GroupBy::DroppedDate => "dropped_date",
            GroupBy::AddedDate => "added_date",
            GroupBy::ChangedDate => "changed_date",
            GroupBy::NextReview => "next_review",
            GroupBy::TagsCombined => "tags_combined",
            GroupBy::Flagged => "flagged",
            GroupBy::Estimate => "estimate",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        [
            GroupBy::None,
            GroupBy::Project,
            GroupBy::Folder,
            GroupBy::Tag,
            GroupBy::DueDate,
            GroupBy::DeferDate,
            GroupBy::PlannedDate,
            GroupBy::CompletedDate,
            GroupBy::DroppedDate,
            GroupBy::AddedDate,
            GroupBy::ChangedDate,
            GroupBy::NextReview,
            GroupBy::TagsCombined,
            GroupBy::Flagged,
            GroupBy::Estimate,
        ]
        .into_iter()
        .find(|group| group.slug() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Manual,
    Due,
    Defer,
    Planned,
    Name,
    Priority,
    Flagged,
    Estimate,
    Created,
    CompletedAt,
    DueFlagged,
    Tag,
    AddedDate,
    ChangedDate,
    DroppedDate,
    NextReview,
}

impl SortBy {
    pub fn slug(self) -> &'static str {
        match self {
            SortBy::Manual => "manual",
            SortBy::Due => "due",
            SortBy::Defer => "defer",
            SortBy::Planned => "planned",
            SortBy::Name => "name",
            SortBy::Priority => "priority",
            SortBy::Flagged => "flagged",
            SortBy::Estimate => "estimate",
            SortBy::Created => "created",
            SortBy::CompletedAt => "completed_at",
            SortBy::DueFlagged => "due_flagged",
            SortBy::Tag => "tag",
            SortBy::AddedDate => "added_date",
            SortBy::ChangedDate => "changed_date",
            SortBy::DroppedDate => "dropped_date",
            SortBy::NextReview => "next_review",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        [
            SortBy::Manual,
            SortBy::Due,
            SortBy::Defer,
            SortBy::Planned,
            SortBy::Name,
            SortBy::Priority,
            SortBy::Flagged,
            SortBy::Estimate,
            SortBy::Created,
            SortBy::CompletedAt,
            SortBy::DueFlagged,
            SortBy::Tag,
            SortBy::AddedDate,
            SortBy::ChangedDate,
            SortBy::DroppedDate,
            SortBy::NextReview,
        ]
        .into_iter()
        .find(|sort| sort.slug() == value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DueWindow {
    Overdue,
    Today,
    Tomorrow,
    Next7Days,
    Next30Days,
    /// After today, or no date at all. Used by views such as Running that
    /// want work that has not reached its deadline yet.
    Later,
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

/// The two quick status rules OmniFocus offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterStatus {
    DueSoon,
    Flagged,
}

impl FilterStatus {
    pub const ALL: [FilterStatus; 2] = [FilterStatus::DueSoon, FilterStatus::Flagged];

    pub fn label(self) -> &'static str {
        match self {
            FilterStatus::DueSoon => "Due Soon",
            FilterStatus::Flagged => "Flagged",
        }
    }
}

/// OmniFocus availability, as applied to an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityFilter {
    FirstAvailable,
    Available,
    Remaining,
    Completed,
    Dropped,
}

impl AvailabilityFilter {
    pub const ALL: [AvailabilityFilter; 5] = [
        AvailabilityFilter::FirstAvailable,
        AvailabilityFilter::Available,
        AvailabilityFilter::Remaining,
        AvailabilityFilter::Completed,
        AvailabilityFilter::Dropped,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AvailabilityFilter::FirstAvailable => "First Available",
            AvailabilityFilter::Available => "Available",
            AvailabilityFilter::Remaining => "Remaining",
            AvailabilityFilter::Completed => "Completed",
            AvailabilityFilter::Dropped => "Dropped",
        }
    }
}

/// The five options behind OmniFocus's “Has a tag that…” rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagStatusFilter {
    ActiveOrOnHold,
    OnHold,
    Dropped,
    ActiveWithAvailableActions,
    ActiveWithNoAvailableActions,
}

impl TagStatusFilter {
    pub const ALL: [TagStatusFilter; 5] = [
        TagStatusFilter::ActiveOrOnHold,
        TagStatusFilter::OnHold,
        TagStatusFilter::Dropped,
        TagStatusFilter::ActiveWithAvailableActions,
        TagStatusFilter::ActiveWithNoAvailableActions,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TagStatusFilter::ActiveOrOnHold => "active or on hold",
            TagStatusFilter::OnHold => "on hold",
            TagStatusFilter::Dropped => "dropped",
            TagStatusFilter::ActiveWithAvailableActions => "active with available actions",
            TagStatusFilter::ActiveWithNoAvailableActions => {
                "active with no available actions"
            }
        }
    }
}

/// Whether an item is a project, an action group, or either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKindFilter {
    Project,
    Group,
    ProjectOrGroup,
    NotProjectOrGroup,
}

impl ItemKindFilter {
    pub const ALL: [ItemKindFilter; 4] = [
        ItemKindFilter::Project,
        ItemKindFilter::Group,
        ItemKindFilter::ProjectOrGroup,
        ItemKindFilter::NotProjectOrGroup,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ItemKindFilter::Project => "a project",
            ItemKindFilter::Group => "a group",
            ItemKindFilter::ProjectOrGroup => "a project or group",
            ItemKindFilter::NotProjectOrGroup => "not a project or group",
        }
    }
}

/// The seven options behind OmniFocus's “Has a project that…” rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatusFilter {
    Active,
    ActiveOrOnHold,
    OnHold,
    Completed,
    Dropped,
    ActiveWithNoRemainingActions,
    ActiveWithFutureDefer,
}

impl ProjectStatusFilter {
    pub const ALL: [ProjectStatusFilter; 7] = [
        ProjectStatusFilter::Active,
        ProjectStatusFilter::ActiveOrOnHold,
        ProjectStatusFilter::OnHold,
        ProjectStatusFilter::Completed,
        ProjectStatusFilter::Dropped,
        ProjectStatusFilter::ActiveWithNoRemainingActions,
        ProjectStatusFilter::ActiveWithFutureDefer,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ProjectStatusFilter::Active => "is active",
            ProjectStatusFilter::ActiveOrOnHold => "is active or on hold",
            ProjectStatusFilter::OnHold => "is on hold",
            ProjectStatusFilter::Completed => "has been completed",
            ProjectStatusFilter::Dropped => "has been dropped",
            ProjectStatusFilter::ActiveWithNoRemainingActions => {
                "is active with no remaining actions"
            }
            ProjectStatusFilter::ActiveWithFutureDefer => "is active with a future defer date",
        }
    }
}

/// Which date a range rule inspects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    Due,
    Planned,
    Defer,
    Completed,
    Dropped,
    Added,
    Changed,
}

impl DateField {
    /// The order OmniFocus lists these in the date-range pop-up.
    pub const ALL: [DateField; 7] = [
        DateField::Due,
        DateField::Planned,
        DateField::Defer,
        DateField::Completed,
        DateField::Dropped,
        DateField::Added,
        DateField::Changed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DateField::Due => "Due Date",
            DateField::Planned => "Planned Date",
            DateField::Defer => "Defer Date",
            DateField::Completed => "Completed Date",
            DateField::Dropped => "Dropped Date",
            DateField::Added => "Added Date",
            DateField::Changed => "Changed Date",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateUnit {
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl DateUnit {
    pub const ALL: [DateUnit; 5] = [
        DateUnit::Hours,
        DateUnit::Days,
        DateUnit::Weeks,
        DateUnit::Months,
        DateUnit::Years,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DateUnit::Hours => "hours",
            DateUnit::Days => "days",
            DateUnit::Weeks => "weeks",
            DateUnit::Months => "months",
            DateUnit::Years => "years",
        }
    }
}

/// The right-hand pop-up of OmniFocus's “Has date in range” rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DateRangeFilter {
    On {
        date: NaiveDate,
    },
    Yesterday,
    Today,
    Tomorrow,
    InPast {
        amount: u32,
        unit: DateUnit,
    },
    InNext {
        amount: u32,
        unit: DateUnit,
    },
    Between {
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    },
}

impl Default for DateRangeFilter {
    fn default() -> Self {
        DateRangeFilter::Today
    }
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
    Planned {
        window: DueWindow,
    },
    HasRepeat {
        value: bool,
    },
    FullscreenReminder {
        value: bool,
    },
    Estimate {
        min_minutes: Option<u32>,
        max_minutes: Option<u32>,
    },
    Text {
        value: String,
    },

    // ---------------------------------------------------------- OmniFocus 4
    StatusKind {
        value: FilterStatus,
    },
    Availability {
        value: AvailabilityFilter,
    },
    HasEstimate {
        value: bool,
    },
    EstimateLessThan {
        minutes: u32,
    },
    EstimateGreaterThan {
        minutes: u32,
    },
    HasDate {
        field: DateField,
        value: bool,
    },
    DateRange {
        field: DateField,
        range: DateRangeFilter,
    },
    IsUntagged {
        value: bool,
    },
    TagStatus {
        value: TagStatusFilter,
    },
    TagsAny {
        ids: Vec<TagId>,
        include_children: bool,
    },
    TagsAll {
        ids: Vec<TagId>,
        include_children: bool,
    },
    ItemKind {
        value: ItemKindFilter,
    },
    InSingleActionsList,
    ProjectStatus {
        value: ProjectStatusFilter,
    },
    Within {
        project: Option<ProjectId>,
        folder: Option<FolderId>,
        include_children: bool,
    },
    Search {
        terms: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterNode {
    All(Vec<FilterNode>),
    Any(Vec<FilterNode>),
    /// “None of the following”: matches when no child matches.
    None(Vec<FilterNode>),
    Not(Box<FilterNode>),
    Condition(Condition),
    Comment(String),
    /// A rule kept in the tree but skipped while matching.
    Disabled(Box<FilterNode>),
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
            FilterNode::None(nodes) => {
                map.serialize_entry("type", "none")?;
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
            FilterNode::Comment(comment) => {
                map.serialize_entry("type", "comment")?;
                map.serialize_entry("comment", comment)?;
            }
            FilterNode::Disabled(node) => {
                map.serialize_entry("type", "disabled")?;
                map.serialize_entry("node", node)?;
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
    None {
        #[serde(default)]
        nodes: Vec<FilterNode>,
    },
    Not {
        node: Box<FilterNode>,
    },
    Condition {
        condition: Condition,
    },
    Comment {
        #[serde(default)]
        comment: String,
    },
    Disabled {
        node: Box<FilterNode>,
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
            FilterNodeRepr::None { nodes } => FilterNode::None(nodes),
            FilterNodeRepr::Not { node } => FilterNode::Not(node),
            FilterNodeRepr::Condition { condition } => FilterNode::Condition(condition),
            FilterNodeRepr::Comment { comment } => FilterNode::Comment(comment),
            FilterNodeRepr::Disabled { node } => FilterNode::Disabled(node),
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

    pub fn none(nodes: Vec<FilterNode>) -> Self {
        FilterNode::None(nodes)
    }

    pub fn comment(comment: impl Into<String>) -> Self {
        FilterNode::Comment(comment.into())
    }

    pub fn disabled(node: FilterNode) -> Self {
        FilterNode::Disabled(Box::new(node))
    }

    /// All of these conditions must match. An empty list matches everything.
    pub fn all_conditions(conditions: Vec<Condition>) -> Self {
        FilterNode::All(conditions.into_iter().map(FilterNode::Condition).collect())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            FilterNode::All(nodes) | FilterNode::Any(nodes) | FilterNode::None(nodes) => {
                nodes.is_empty()
            }
            FilterNode::Comment(_) | FilterNode::Disabled(_) => true,
            _ => false,
        }
    }

    /// Comments and turned-off rules never affect a match. Group evaluation
    /// drops them before combining children so a disabled rule inside `any`
    /// cannot accidentally match everything.
    pub fn is_ignorable(&self) -> bool {
        matches!(self, FilterNode::Comment(_) | FilterNode::Disabled(_))
    }

    /// Whether the tree contains a rule that can actually narrow the library.
    pub fn has_conditions(&self) -> bool {
        match self {
            FilterNode::All(nodes)
            | FilterNode::Any(nodes)
            | FilterNode::None(nodes) => nodes.iter().any(FilterNode::has_conditions),
            FilterNode::Not(node) | FilterNode::Disabled(node) => node.has_conditions(),
            FilterNode::Condition(_) => true,
            FilterNode::Comment(_) => false,
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
    /// How project rows are grouped when a saved view matches projects.
    /// `None` keeps the single Projects block above the action groups.
    #[serde(default = "default_project_group")]
    pub project_group_by: GroupBy,
    /// Ordering for project rows. `Manual` keeps library order.
    #[serde(default)]
    pub project_sort: SortBy,
    #[serde(default)]
    pub show_completed: bool,
    #[serde(default)]
    pub position: u32,
    /// Pinned views sit above the SAVED section and stay visible when it is
    /// collapsed. The flag travels with the perspective, so every client sees
    /// the same rail.
    #[serde(default)]
    pub pinned: bool,
    /// Optional rail glyph. Pinned views default to a pin and other saved
    /// views to the perspective symbol, but either can be replaced.
    #[serde(default)]
    pub icon: Option<IconSpec>,
    /// One-line description shown under the view title. Empty falls back to
    /// the generic Saved perspective subtitle.
    #[serde(default, alias = "note")]
    pub description: String,
}

fn default_group() -> GroupBy {
    GroupBy::None
}

fn default_project_group() -> GroupBy {
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
            project_group_by: GroupBy::None,
            project_sort: SortBy::Manual,
            show_completed: false,
            position: 0,
            pinned: false,
            icon: None,
            description: String::new(),
        }
    }
}
