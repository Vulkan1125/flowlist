//! The custom-perspective editor.
//!
//! OmniFocus exposes one hierarchical contents tree: every rule sits beneath a
//! top-level All, Any or None group, groups nest, rules can be turned off, and
//! comments annotate the tree without affecting the result. This module mirrors
//! that shape. The saved form is still the shared recursive `FilterNode`, so
//! CLI- and MCP-authored perspectives round-trip without loss.

use std::collections::BTreeSet;

use flowlist_core::db::Db;
use flowlist_core::ids::{PerspectiveId, ProjectId, TagId};
use flowlist_core::model::{
    AvailabilityFilter, Condition, DateField, DateRangeFilter, DueWindow, FilterNode, FilterStatus,
    GroupBy, IconSpec, ItemKindFilter, Perspective, Priority, ProjectStatusFilter, SortBy,
    TagStatusFilter, TaskStatus,
};

use super::text::TextBuffer;

/// Which half of the icon card is showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconTab {
    #[default]
    Icons,
    Emoji,
}

/// Due windows offered by the editor. The CLI and MCP server can still write
/// the date-based variants; they survive editing untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DueChoice {
    Any,
    Overdue,
    Today,
    Tomorrow,
    Next7Days,
    Next30Days,
    Later,
    NoDate,
}

impl DueChoice {
    pub fn label(self) -> &'static str {
        match self {
            DueChoice::Any => "Any",
            DueChoice::Overdue => "Overdue",
            DueChoice::Today => "Today",
            DueChoice::Tomorrow => "Tomorrow",
            DueChoice::Next7Days => "Next 7 days",
            DueChoice::Next30Days => "Next 30 days",
            DueChoice::Later => "Later / none",
            DueChoice::NoDate => "No date",
        }
    }

    pub fn window(self) -> Option<DueWindow> {
        match self {
            DueChoice::Any => None,
            DueChoice::Overdue => Some(DueWindow::Overdue),
            DueChoice::Today => Some(DueWindow::Today),
            DueChoice::Tomorrow => Some(DueWindow::Tomorrow),
            DueChoice::Next7Days => Some(DueWindow::Next7Days),
            DueChoice::Next30Days => Some(DueWindow::Next30Days),
            DueChoice::Later => Some(DueWindow::Later),
            DueChoice::NoDate => Some(DueWindow::NoDate),
        }
    }

    fn from_window(window: DueWindow) -> Option<Self> {
        match window {
            DueWindow::Overdue => Some(DueChoice::Overdue),
            DueWindow::Today => Some(DueChoice::Today),
            DueWindow::Tomorrow => Some(DueChoice::Tomorrow),
            DueWindow::Next7Days => Some(DueChoice::Next7Days),
            DueWindow::Next30Days => Some(DueChoice::Next30Days),
            DueWindow::Later => Some(DueChoice::Later),
            DueWindow::NoDate => Some(DueChoice::NoDate),
            _ => None,
        }
    }
}

/// Deferred tasks are never overdue, so the editor leaves that window out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeferChoice {
    Any,
    Today,
    Tomorrow,
    Next7Days,
    Next30Days,
    NoDate,
}

impl DeferChoice {
    pub fn label(self) -> &'static str {
        match self {
            DeferChoice::Any => "Any",
            DeferChoice::Today => "Today",
            DeferChoice::Tomorrow => "Tomorrow",
            DeferChoice::Next7Days => "Next 7 days",
            DeferChoice::Next30Days => "Next 30 days",
            DeferChoice::NoDate => "No date",
        }
    }

    pub fn window(self) -> Option<DueWindow> {
        match self {
            DeferChoice::Any => None,
            DeferChoice::Today => Some(DueWindow::Today),
            DeferChoice::Tomorrow => Some(DueWindow::Tomorrow),
            DeferChoice::Next7Days => Some(DueWindow::Next7Days),
            DeferChoice::Next30Days => Some(DueWindow::Next30Days),
            DeferChoice::NoDate => Some(DueWindow::NoDate),
        }
    }

    fn from_window(window: DueWindow) -> Option<Self> {
        match window {
            DueWindow::Today => Some(DeferChoice::Today),
            DueWindow::Tomorrow => Some(DeferChoice::Tomorrow),
            DueWindow::Next7Days => Some(DeferChoice::Next7Days),
            DueWindow::Next30Days => Some(DeferChoice::Next30Days),
            DueWindow::NoDate => Some(DeferChoice::NoDate),
            _ => None,
        }
    }
}

/// A control in the perspective editor. Each one owns a menu of every value,
/// so choosing a project or a status is a jump rather than a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewField {
    Project,
    Tag,
    Status,
    Due,
    Planned,
    Defer,
    Fullscreen,
    Group,
    Sort,
    ProjectGroup,
    ProjectSort,
}

impl ViewField {
    pub fn key(self) -> &'static str {
        match self {
            ViewField::Project => "project",
            ViewField::Tag => "tag",
            ViewField::Status => "status",
            ViewField::Due => "due",
            ViewField::Planned => "planned",
            ViewField::Defer => "defer",
            ViewField::Fullscreen => "fullscreen",
            ViewField::Group => "group",
            ViewField::Sort => "sort",
            ViewField::ProjectGroup => "project-group",
            ViewField::ProjectSort => "project-sort",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewField::Project => "Projects",
            ViewField::Tag => "Tags",
            ViewField::Status => "Status",
            ViewField::Due => "Due dates",
            ViewField::Planned => "Planned dates",
            ViewField::Defer => "Defer dates",
            ViewField::Fullscreen => "Full-screen reminders",
            ViewField::Group => "Group by",
            ViewField::Sort => "Sort by",
            ViewField::ProjectGroup => "Group projects by",
            ViewField::ProjectSort => "Sort projects by",
        }
    }

    /// Fields whose menu stays open while several values are toggled.
    pub fn is_multi(self) -> bool {
        matches!(
            self,
            ViewField::Project
                | ViewField::Tag
                | ViewField::Due
                | ViewField::Planned
                | ViewField::Defer
        )
    }
}

/// One menu value offered by a [`ViewField`]. Project and tag choices carry
/// the library id so the editor can label them with the current name.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewChoice {
    Project(Option<ProjectId>),
    Tag(Option<TagId>),
    Status(TaskStatus),
    Due(DueChoice),
    Planned(DueChoice),
    Defer(DeferChoice),
    Fullscreen(Option<bool>),
    Group(GroupBy),
    Sort(SortBy),
    ProjectGroup(GroupBy),
    ProjectSort(SortBy),
}

impl ViewChoice {
    /// A stable id fragment for probes and element ids.
    pub fn slug(&self) -> String {
        match self {
            ViewChoice::Project(Some(id)) => id.to_string(),
            ViewChoice::Tag(Some(id)) => id.to_string(),
            ViewChoice::Project(None) | ViewChoice::Tag(None) => "any".to_string(),
            ViewChoice::Status(status) => status_slug(*status).to_string(),
            ViewChoice::Due(choice) => match choice {
                DueChoice::Any => "any",
                DueChoice::Overdue => "overdue",
                DueChoice::Today => "today",
                DueChoice::Tomorrow => "tomorrow",
                DueChoice::Next7Days => "next7",
                DueChoice::Next30Days => "next30",
                DueChoice::Later => "later",
                DueChoice::NoDate => "none",
            }
            .to_string(),
            ViewChoice::Fullscreen(Some(true)) => "has".to_string(),
            ViewChoice::Fullscreen(Some(false)) => "none".to_string(),
            ViewChoice::Fullscreen(None) => "any".to_string(),
            ViewChoice::Planned(choice) => match choice {
                DueChoice::Any => "any",
                DueChoice::Overdue => "overdue",
                DueChoice::Today => "today",
                DueChoice::Tomorrow => "tomorrow",
                DueChoice::Next7Days => "next7",
                DueChoice::Next30Days => "next30",
                DueChoice::Later => "later",
                DueChoice::NoDate => "none",
            }
            .to_string(),
            ViewChoice::Defer(choice) => match choice {
                DeferChoice::Any => "any",
                DeferChoice::Today => "today",
                DeferChoice::Tomorrow => "tomorrow",
                DeferChoice::Next7Days => "next7",
                DeferChoice::Next30Days => "next30",
                DeferChoice::NoDate => "none",
            }
            .to_string(),
            ViewChoice::ProjectGroup(group) => match group {
                GroupBy::None => "none",
                GroupBy::Project => "project",
                GroupBy::Folder => "folder",
                GroupBy::DueDate => "due",
                GroupBy::DeferDate => "defer",
                GroupBy::PlannedDate => "planned",
                GroupBy::AddedDate => "added",
                GroupBy::ChangedDate => "changed",
                GroupBy::NextReview => "review",
                _ => "other",
            }
            .to_string(),
            ViewChoice::ProjectSort(sort) => match sort {
                SortBy::Manual => "manual",
                SortBy::Due => "due",
                SortBy::Defer => "defer",
                SortBy::Planned => "planned",
                SortBy::Name => "name",
                SortBy::Created => "created",
                SortBy::AddedDate => "added",
                SortBy::ChangedDate => "changed",
                SortBy::NextReview => "review",
                _ => "other",
            }
            .to_string(),
            ViewChoice::Group(group) => match group {
                GroupBy::None => "none",
                GroupBy::Project => "project",
                GroupBy::Folder => "folder",
                GroupBy::Tag => "tag",
                GroupBy::DueDate => "due",
                GroupBy::DeferDate => "defer",
                GroupBy::PlannedDate => "planned",
                GroupBy::CompletedDate => "completed",
                GroupBy::DroppedDate => "dropped",
                GroupBy::AddedDate => "added",
                GroupBy::ChangedDate => "changed",
                GroupBy::NextReview => "review",
                GroupBy::TagsCombined => "tags",
                GroupBy::Flagged => "flagged",
                GroupBy::Estimate => "estimate",
            }
            .to_string(),
            ViewChoice::Sort(sort) => match sort {
                SortBy::Manual => "manual",
                SortBy::Due => "due",
                SortBy::Defer => "defer",
                SortBy::Planned => "planned",
                SortBy::Name => "name",
                SortBy::Priority => "priority",
                SortBy::Flagged => "flagged",
                SortBy::Estimate => "estimate",
                SortBy::Created => "created",
                SortBy::CompletedAt => "completed",
                SortBy::DueFlagged => "due-flagged",
                SortBy::Tag => "tag",
                SortBy::AddedDate => "added",
                SortBy::ChangedDate => "changed",
                SortBy::DroppedDate => "dropped",
                SortBy::NextReview => "review",
            }
            .to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerspectiveTab {
    Simple,
    Comprehensive,
}

impl PerspectiveTab {
    pub fn label(self) -> &'static str {
        match self {
            PerspectiveTab::Simple => "Simple",
            PerspectiveTab::Comprehensive => "Comprehensive",
        }
    }
}

/// A rule in the editor tree. `id` survives rebuilds of the visible rows, so
/// the open editor and row menu stay attached to the same rule while the rest
/// of the tree changes.
#[derive(Clone, Debug)]
pub struct RuleNode {
    pub id: u64,
    pub enabled: bool,
    pub kind: RuleKind,
    /// Primary editable text: a comment, search terms, an amount, or a date.
    pub text: TextBuffer,
    /// Second editable text, used by Between date ranges.
    pub text2: TextBuffer,
}

#[derive(Clone, Debug)]
pub enum RuleKind {
    All(Vec<RuleNode>),
    Any(Vec<RuleNode>),
    None(Vec<RuleNode>),
    Comment,
    Condition(Condition),
}

impl RuleKind {
    pub fn group_label(&self) -> Option<&'static str> {
        match self {
            RuleKind::All(_) => Some("All of the following"),
            RuleKind::Any(_) => Some("Any of the following"),
            RuleKind::None(_) => Some("None of the following"),
            _ => None,
        }
    }

    pub fn is_group(&self) -> bool {
        matches!(self, RuleKind::All(_) | RuleKind::Any(_) | RuleKind::None(_))
    }
}

/// The rule catalogue in the order OmniFocus presents it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleTemplate {
    StatusDueSoon,
    StatusFlagged,
    Availability,
    HasEstimate,
    EstimateLessThan,
    EstimateGreaterThan,
    EstimateRange,
    HasDue,
    HasPlanned,
    HasDefer,
    DateInRange,
    Repeating,
    Untagged,
    TagStatus,
    TagsAny,
    TagsAll,
    ItemKind,
    Inbox,
    SingleActions,
    ProjectStatus,
    Within,
    Search,
    GroupAll,
    GroupAny,
    GroupNone,
    Comment,
}

impl RuleTemplate {
    pub const ALL: [RuleTemplate; 26] = [
        RuleTemplate::StatusDueSoon,
        RuleTemplate::StatusFlagged,
        RuleTemplate::Availability,
        RuleTemplate::HasEstimate,
        RuleTemplate::EstimateLessThan,
        RuleTemplate::EstimateGreaterThan,
        RuleTemplate::EstimateRange,
        RuleTemplate::HasDue,
        RuleTemplate::HasPlanned,
        RuleTemplate::HasDefer,
        RuleTemplate::DateInRange,
        RuleTemplate::Repeating,
        RuleTemplate::Untagged,
        RuleTemplate::TagStatus,
        RuleTemplate::TagsAny,
        RuleTemplate::TagsAll,
        RuleTemplate::ItemKind,
        RuleTemplate::Inbox,
        RuleTemplate::SingleActions,
        RuleTemplate::ProjectStatus,
        RuleTemplate::Within,
        RuleTemplate::Search,
        RuleTemplate::GroupAll,
        RuleTemplate::GroupAny,
        RuleTemplate::GroupNone,
        RuleTemplate::Comment,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RuleTemplate::StatusDueSoon => "Status: Due Soon",
            RuleTemplate::StatusFlagged => "Status: Flagged",
            RuleTemplate::Availability => "Availability",
            RuleTemplate::HasEstimate => "Has an estimated duration",
            RuleTemplate::EstimateLessThan => "Has an estimated duration less than",
            RuleTemplate::EstimateGreaterThan => "Has an estimated duration greater than",
            RuleTemplate::EstimateRange => "Has an estimated duration between",
            RuleTemplate::HasDue => "Has a due date",
            RuleTemplate::HasPlanned => "Has a planned date",
            RuleTemplate::HasDefer => "Has a defer date",
            RuleTemplate::DateInRange => "Has date in range",
            RuleTemplate::Repeating => "Is repeating",
            RuleTemplate::Untagged => "Is untagged",
            RuleTemplate::TagStatus => "Has a tag that…",
            RuleTemplate::TagsAny => "Is tagged with any of",
            RuleTemplate::TagsAll => "Is tagged with all of",
            RuleTemplate::ItemKind => "Is a project, group, or neither",
            RuleTemplate::Inbox => "Is in the Inbox",
            RuleTemplate::SingleActions => "Is in a single actions list",
            RuleTemplate::ProjectStatus => "Has a project that…",
            RuleTemplate::Within => "Is contained within project or folder",
            RuleTemplate::Search => "Matches search terms",
            RuleTemplate::GroupAll => "All of the following",
            RuleTemplate::GroupAny => "Any of the following",
            RuleTemplate::GroupNone => "None of the following",
            RuleTemplate::Comment => "Comment",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            RuleTemplate::StatusDueSoon | RuleTemplate::StatusFlagged => "STATUS",
            RuleTemplate::Availability => "STATUS",
            RuleTemplate::HasEstimate
            | RuleTemplate::EstimateLessThan
            | RuleTemplate::EstimateGreaterThan
            | RuleTemplate::EstimateRange
            | RuleTemplate::HasDue
            | RuleTemplate::HasPlanned
            | RuleTemplate::HasDefer
            | RuleTemplate::DateInRange
            | RuleTemplate::Repeating => "DATES",
            RuleTemplate::Untagged
            | RuleTemplate::TagStatus
            | RuleTemplate::TagsAny
            | RuleTemplate::TagsAll => "TAGS",
            RuleTemplate::ItemKind
            | RuleTemplate::Inbox
            | RuleTemplate::SingleActions => "STRUCTURE",
            RuleTemplate::ProjectStatus | RuleTemplate::Within => "PROJECTS",
            RuleTemplate::Search => "TEXT",
            RuleTemplate::GroupAll
            | RuleTemplate::GroupAny
            | RuleTemplate::GroupNone
            | RuleTemplate::Comment => "LOGIC",
        }
    }

    pub fn build(self, next_id: &mut u64) -> RuleNode {
        let kind = match self {
            RuleTemplate::StatusDueSoon => RuleKind::Condition(Condition::StatusKind {
                value: FilterStatus::DueSoon,
            }),
            RuleTemplate::StatusFlagged => RuleKind::Condition(Condition::StatusKind {
                value: FilterStatus::Flagged,
            }),
            RuleTemplate::Availability => RuleKind::Condition(Condition::Availability {
                value: AvailabilityFilter::Remaining,
            }),
            RuleTemplate::HasEstimate => RuleKind::Condition(Condition::HasEstimate { value: true }),
            RuleTemplate::EstimateLessThan => RuleKind::Condition(Condition::EstimateLessThan {
                minutes: 30,
            }),
            RuleTemplate::EstimateGreaterThan => {
                RuleKind::Condition(Condition::EstimateGreaterThan { minutes: 30 })
            }
            RuleTemplate::EstimateRange => RuleKind::Condition(Condition::Estimate {
                min_minutes: Some(15),
                max_minutes: Some(60),
            }),
            RuleTemplate::HasDue => RuleKind::Condition(Condition::HasDate {
                field: DateField::Due,
                value: true,
            }),
            RuleTemplate::HasPlanned => RuleKind::Condition(Condition::HasDate {
                field: DateField::Planned,
                value: true,
            }),
            RuleTemplate::HasDefer => RuleKind::Condition(Condition::HasDate {
                field: DateField::Defer,
                value: true,
            }),
            RuleTemplate::DateInRange => RuleKind::Condition(Condition::DateRange {
                field: DateField::Due,
                range: DateRangeFilter::Today,
            }),
            RuleTemplate::Repeating => RuleKind::Condition(Condition::HasRepeat { value: true }),
            RuleTemplate::Untagged => RuleKind::Condition(Condition::IsUntagged { value: true }),
            RuleTemplate::TagStatus => RuleKind::Condition(Condition::TagStatus {
                value: TagStatusFilter::ActiveOrOnHold,
            }),
            RuleTemplate::TagsAny => RuleKind::Condition(Condition::TagsAny {
                ids: Vec::new(),
                include_children: true,
            }),
            RuleTemplate::TagsAll => RuleKind::Condition(Condition::TagsAll {
                ids: Vec::new(),
                include_children: true,
            }),
            RuleTemplate::ItemKind => RuleKind::Condition(Condition::ItemKind {
                value: ItemKindFilter::NotProjectOrGroup,
            }),
            RuleTemplate::Inbox => RuleKind::Condition(Condition::InInbox),
            RuleTemplate::SingleActions => {
                RuleKind::Condition(Condition::InSingleActionsList)
            }
            RuleTemplate::ProjectStatus => RuleKind::Condition(Condition::ProjectStatus {
                value: ProjectStatusFilter::Active,
            }),
            RuleTemplate::Within => RuleKind::Condition(Condition::Within {
                project: None,
                folder: None,
                include_children: true,
            }),
            RuleTemplate::Search => RuleKind::Condition(Condition::Search { terms: Vec::new() }),
            RuleTemplate::GroupAll => RuleKind::All(Vec::new()),
            RuleTemplate::GroupAny => RuleKind::Any(Vec::new()),
            RuleTemplate::GroupNone => RuleKind::None(Vec::new()),
            RuleTemplate::Comment => RuleKind::Comment,
        };
        RuleNode {
            id: fresh_id(next_id),
            enabled: true,
            kind,
            text: TextBuffer::empty(),
            text2: TextBuffer::empty(),
        }
    }
}

fn fresh_id(next_id: &mut u64) -> u64 {
    let id = *next_id;
    *next_id += 1;
    id
}

impl RuleNode {
    pub fn group(kind: RuleKind, next_id: &mut u64) -> Self {
        Self {
            id: fresh_id(next_id),
            enabled: true,
            kind,
            text: TextBuffer::empty(),
            text2: TextBuffer::empty(),
        }
    }

    pub fn from_filter(filter: &FilterNode, next_id: &mut u64) -> Self {
        let mut node = match filter {
            FilterNode::All(nodes) => Self {
                id: fresh_id(next_id),
                enabled: true,
                kind: RuleKind::All(
                    nodes
                        .iter()
                        .map(|node| Self::from_filter(node, next_id))
                        .collect(),
                ),
                text: TextBuffer::empty(),
                text2: TextBuffer::empty(),
            },
            FilterNode::Any(nodes) => Self {
                id: fresh_id(next_id),
                enabled: true,
                kind: RuleKind::Any(
                    nodes
                        .iter()
                        .map(|node| Self::from_filter(node, next_id))
                        .collect(),
                ),
                text: TextBuffer::empty(),
                text2: TextBuffer::empty(),
            },
            FilterNode::None(nodes) => Self {
                id: fresh_id(next_id),
                enabled: true,
                kind: RuleKind::None(
                    nodes
                        .iter()
                        .map(|node| Self::from_filter(node, next_id))
                        .collect(),
                ),
                text: TextBuffer::empty(),
                text2: TextBuffer::empty(),
            },
            // The editor expresses "not" as a None group; the two are the same
            // boolean operation and this keeps every rule editable.
            FilterNode::Not(inner) => Self {
                id: fresh_id(next_id),
                enabled: true,
                kind: RuleKind::None(vec![Self::from_filter(inner, next_id)]),
                text: TextBuffer::empty(),
                text2: TextBuffer::empty(),
            },
            FilterNode::Condition(condition) => {
                let (text, text2) = condition_text(condition);
                Self {
                    id: fresh_id(next_id),
                    enabled: true,
                    kind: RuleKind::Condition(condition.clone()),
                    text: TextBuffer::new(text),
                    text2: TextBuffer::new(text2),
                }
            }
            FilterNode::Comment(comment) => Self {
                id: fresh_id(next_id),
                enabled: true,
                kind: RuleKind::Comment,
                text: TextBuffer::new(comment.clone()),
                text2: TextBuffer::empty(),
            },
            FilterNode::Disabled(inner) => {
                let mut node = Self::from_filter(inner, next_id);
                node.enabled = false;
                node
            }
        };
        node.sync_text();
        node
    }

    /// A new perspective starts with OmniFocus's example rule.
    pub fn default_root(next_id: &mut u64) -> Self {
        let mut root = Self::group(RuleKind::All(Vec::new()), next_id);
        let rule = RuleTemplate::Availability.build(next_id);
        if let RuleKind::All(children) = &mut root.kind {
            children.push(rule);
        }
        root
    }

    pub fn to_filter(&self) -> FilterNode {
        let inner = match &self.kind {
            RuleKind::All(nodes) => {
                FilterNode::All(nodes.iter().map(RuleNode::to_filter).collect())
            }
            RuleKind::Any(nodes) => {
                FilterNode::Any(nodes.iter().map(RuleNode::to_filter).collect())
            }
            RuleKind::None(nodes) => {
                FilterNode::None(nodes.iter().map(RuleNode::to_filter).collect())
            }
            RuleKind::Comment => FilterNode::Comment(self.text.text().to_string()),
            RuleKind::Condition(_) => FilterNode::Condition(self.condition()),
        };
        if self.enabled {
            inner
        } else {
            FilterNode::Disabled(Box::new(inner))
        }
    }

    pub fn children(&self) -> &[RuleNode] {
        match &self.kind {
            RuleKind::All(nodes) | RuleKind::Any(nodes) | RuleKind::None(nodes) => nodes,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<RuleNode>> {
        match &mut self.kind {
            RuleKind::All(nodes) | RuleKind::Any(nodes) | RuleKind::None(nodes) => Some(nodes),
            _ => None,
        }
    }

    /// The condition with any editable text applied. Text, search terms,
    /// estimates and date ranges all live in `text`/`text2` while the editor is
    /// open, so a half-typed date never destroys the saved rule.
    pub fn condition(&self) -> Condition {
        let RuleKind::Condition(condition) = &self.kind else {
            return Condition::InInbox;
        };
        match condition {
            Condition::Text { .. } => Condition::Text {
                value: self.text.text().trim().to_string(),
            },
            Condition::Search { .. } => Condition::Search {
                terms: split_terms(&self.text.text()),
            },
            Condition::Estimate { .. } => Condition::Estimate {
                min_minutes: parse_u32(&self.text.text()),
                max_minutes: parse_u32(&self.text2.text()),
            },
            Condition::EstimateLessThan { minutes } => Condition::EstimateLessThan {
                minutes: parse_u32(&self.text.text()).unwrap_or(*minutes),
            },
            Condition::EstimateGreaterThan { minutes } => Condition::EstimateGreaterThan {
                minutes: parse_u32(&self.text.text()).unwrap_or(*minutes),
            },
            Condition::DateRange { field, range } => Condition::DateRange {
                field: *field,
                range: edited_range(range, &self.text.text(), &self.text2.text()),
            },
            other => other.clone(),
        }
    }

    pub fn sync_text(&mut self) {
        if let RuleKind::Condition(condition) = &self.kind {
            let (text, text2) = condition_text(condition);
            self.text.set(text);
            self.text2.set(text2);
        }
    }

    pub fn condition_ref(&self) -> Option<&Condition> {
        match &self.kind {
            RuleKind::Condition(condition) => Some(condition),
            _ => None,
        }
    }

    pub fn deep_clone(&self, next_id: &mut u64) -> Self {
        let kind = match &self.kind {
            RuleKind::All(nodes) => RuleKind::All(
                nodes.iter().map(|node| node.deep_clone(next_id)).collect(),
            ),
            RuleKind::Any(nodes) => RuleKind::Any(
                nodes.iter().map(|node| node.deep_clone(next_id)).collect(),
            ),
            RuleKind::None(nodes) => RuleKind::None(
                nodes.iter().map(|node| node.deep_clone(next_id)).collect(),
            ),
            RuleKind::Comment => RuleKind::Comment,
            RuleKind::Condition(condition) => RuleKind::Condition(condition.clone()),
        };
        Self {
            id: fresh_id(next_id),
            enabled: self.enabled,
            kind,
            text: TextBuffer::new(self.text.text().to_string()),
            text2: TextBuffer::new(self.text2.text().to_string()),
        }
    }

    pub fn has_conditions(&self) -> bool {
        match &self.kind {
            RuleKind::All(nodes) | RuleKind::Any(nodes) | RuleKind::None(nodes) => {
                nodes.iter().any(RuleNode::has_conditions)
            }
            RuleKind::Condition(_) => true,
            RuleKind::Comment => false,
        }
    }
}

fn split_terms(value: &str) -> Vec<String> {
    value
        .split([',', '\n'])
        .map(|term| term.trim().to_string())
        .filter(|term| !term.is_empty())
        .collect()
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse().ok()
}

fn condition_text(condition: &Condition) -> (String, String) {
    match condition {
        Condition::Text { value } => (value.clone(), String::new()),
        Condition::Search { terms } => (terms.join(", "), String::new()),
        Condition::Estimate {
            min_minutes,
            max_minutes,
        } => (
            min_minutes.map(|value| value.to_string()).unwrap_or_default(),
            max_minutes.map(|value| value.to_string()).unwrap_or_default(),
        ),
        Condition::EstimateLessThan { minutes } | Condition::EstimateGreaterThan { minutes } => {
            (minutes.to_string(), String::new())
        }
        Condition::DateRange { range, .. } => match range {
            DateRangeFilter::On { date } => (date.to_string(), String::new()),
            DateRangeFilter::Between { from, to } => (
                from.map(|date| date.to_string()).unwrap_or_default(),
                to.map(|date| date.to_string()).unwrap_or_default(),
            ),
            DateRangeFilter::InPast { amount, .. }
            | DateRangeFilter::InNext { amount, .. } => (amount.to_string(), String::new()),
            _ => (String::new(), String::new()),
        },
        _ => (String::new(), String::new()),
    }
}

fn parse_date_input(value: &str) -> Option<chrono::NaiveDate> {
    flowlist_core::parse::parse_date(value.trim(), flowlist_core::time::now()).ok()
}

fn edited_range(range: &DateRangeFilter, text: &str, text2: &str) -> DateRangeFilter {
    match range {
        DateRangeFilter::On { date } => DateRangeFilter::On {
            date: parse_date_input(text).unwrap_or(*date),
        },
        DateRangeFilter::Between { .. } => DateRangeFilter::Between {
            from: parse_date_input(text),
            to: parse_date_input(text2),
        },
        DateRangeFilter::InPast { amount, unit } => DateRangeFilter::InPast {
            amount: parse_u32(text).unwrap_or(*amount),
            unit: *unit,
        },
        DateRangeFilter::InNext { amount, unit } => DateRangeFilter::InNext {
            amount: parse_u32(text).unwrap_or(*amount),
            unit: *unit,
        },
        other => other.clone(),
    }
}

/// Everything the editor shows for one perspective.
#[derive(Clone, Debug)]
pub struct PerspectiveForm {
    pub id: Option<PerspectiveId>,
    pub name: TextBuffer,
    pub description: TextBuffer,
    pub position: u32,
    pub pinned: bool,
    pub icon: Option<IconSpec>,
    pub icon_open: bool,
    pub icon_tab: IconTab,
    pub icon_query: TextBuffer,
    pub group: GroupBy,
    pub sort: SortBy,
    pub project_group: GroupBy,
    pub project_sort: SortBy,
    pub show_completed: bool,
    pub tab: PerspectiveTab,
    // ---------------------------------------------------------- simple tab
    pub text: TextBuffer,
    pub flagged_only: bool,
    pub inbox_only: bool,
    pub status: TaskStatus,
    pub dues: BTreeSet<DueChoice>,
    pub planned: BTreeSet<DueChoice>,
    pub defers: BTreeSet<DeferChoice>,
    pub fullscreen: Option<bool>,
    pub projects: BTreeSet<ProjectId>,
    pub tags: BTreeSet<TagId>,
    pub priorities: BTreeSet<Priority>,
    pub tag_children: bool,
    pub estimate_min: TextBuffer,
    pub estimate_max: TextBuffer,
    /// Conditions the simple tab has no widget for.
    pub extra: Vec<Condition>,
    /// Non-condition roots kept intact while the simple tab is used.
    pub extra_roots: Vec<FilterNode>,
    // --------------------------------------------------- comprehensive tab
    pub root: RuleNode,
    /// The rule whose configuration card is open.
    pub editing: Option<u64>,
    /// The group whose Add a new rule menu is open.
    pub add_menu: Option<u64>,
    /// When set, a chosen rule is inserted after this row instead of at the
    /// end of the group.
    pub add_after: Option<u64>,
    /// The rule whose three-dot menu is open.
    pub row_menu: Option<u64>,
    /// Which layout pop-up is open, if any.
    pub layout_menu: Option<LayoutField>,
    next_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutField {
    Group,
    Sort,
}

impl Default for PerspectiveForm {
    fn default() -> Self {
        let mut next_id = 1;
        Self {
            id: None,
            name: TextBuffer::empty(),
            description: TextBuffer::empty(),
            position: 0,
            pinned: false,
            icon: None,
            icon_open: false,
            icon_tab: IconTab::Icons,
            icon_query: TextBuffer::empty(),
            group: GroupBy::Project,
            sort: SortBy::Due,
            project_group: GroupBy::None,
            project_sort: SortBy::Manual,
            show_completed: false,
            tab: PerspectiveTab::Simple,
            text: TextBuffer::empty(),
            flagged_only: false,
            inbox_only: false,
            status: TaskStatus::Any,
            dues: BTreeSet::new(),
            planned: BTreeSet::new(),
            defers: BTreeSet::new(),
            fullscreen: None,
            projects: BTreeSet::new(),
            tags: BTreeSet::new(),
            priorities: BTreeSet::new(),
            tag_children: true,
            estimate_min: TextBuffer::empty(),
            estimate_max: TextBuffer::empty(),
            extra: Vec::new(),
            extra_roots: Vec::new(),
            root: RuleNode::default_root(&mut next_id),
            editing: None,
            add_menu: None,
            add_after: None,
            row_menu: None,
            layout_menu: None,
            next_id,
        }
    }
}

impl PerspectiveForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn edit(perspective: &Perspective) -> Self {
        let mut next_id = 1;
        // The top of the editor is always an All/Any/None group. A single
        // condition or a legacy Not is wrapped without changing its meaning.
        let root = match &perspective.filter {
            FilterNode::All(_) | FilterNode::Any(_) | FilterNode::None(_) => {
                RuleNode::from_filter(&perspective.filter, &mut next_id)
            }
            other => {
                let mut group = RuleNode::group(RuleKind::All(Vec::new()), &mut next_id);
                let child = RuleNode::from_filter(other, &mut next_id);
                if let Some(children) = group.children_mut() {
                    children.push(child);
                }
                group
            }
        };
        let mut form = Self {
            id: Some(perspective.id.clone()),
            name: TextBuffer::new(perspective.name.clone()),
            description: TextBuffer::new(perspective.description.clone()),
            position: perspective.position,
            pinned: perspective.pinned,
            icon: perspective.icon.clone(),
            icon_open: false,
            icon_tab: IconTab::Icons,
            icon_query: TextBuffer::empty(),
            group: perspective.group_by,
            sort: perspective.sort,
            project_group: perspective.project_group_by,
            project_sort: perspective.project_sort,
            show_completed: perspective.show_completed,
            root,
            editing: None,
            add_menu: None,
            row_menu: None,
            layout_menu: None,
            next_id,
            ..Self::default()
        };
        form.read_simple_filter(&perspective.filter);
        // Simple is the first tab for views it can represent exactly. A view
        // with nested groups or advanced rules opens where it can be edited
        // without silently flattening it.
        form.tab = if form.extra_condition_count() == 0 {
            PerspectiveTab::Simple
        } else {
            PerspectiveTab::Comprehensive
        };
        form
    }

    pub fn set_tab(&mut self, tab: PerspectiveTab) {
        if tab == self.tab {
            return;
        }
        match tab {
            PerspectiveTab::Comprehensive => {
                let filter = self.simple_filter();
                // A brand-new comprehensive tab starts with Availability:
                // Remaining. Keep that starter card when adding the simple
                // tab's filters, so moving between tabs never silently drops
                // the example rule.
                if !filter.is_empty() {
                    if root_is_starter(&self.root) {
                        let mut merged = self.root.clone();
                        let mut added = Vec::new();
                        match &filter {
                            FilterNode::All(nodes) => {
                                for node in nodes {
                                    added.push(RuleNode::from_filter(node, &mut self.next_id));
                                }
                            }
                            other => {
                                added.push(RuleNode::from_filter(other, &mut self.next_id));
                            }
                        }
                        if let Some(children) = merged.children_mut() {
                            children.extend(added);
                        }
                        self.root = merged;
                    } else {
                        self.root = RuleNode::from_filter(&filter, &mut self.next_id);
                    }
                }
                self.editing = None;
                self.add_menu = None;
                self.add_after = None;
                self.row_menu = None;
                self.layout_menu = None;
            }
            PerspectiveTab::Simple => {
                let filter = self.root.to_filter();
                self.read_simple_filter(&filter);
            }
        }
        self.tab = tab;
    }

    pub fn to_perspective(&self) -> Perspective {
        let name = self.name.text().trim().to_string();
        let mut perspective = Perspective::new(name);
        if let Some(id) = &self.id {
            perspective.id = id.clone();
        }
        perspective.position = self.position;
        perspective.pinned = self.pinned;
        perspective.icon = self.icon.clone();
        perspective.description = self.description.text().trim().to_string();
        perspective.group_by = self.group;
        perspective.sort = self.sort;
        perspective.project_group_by = self.project_group;
        perspective.project_sort = self.project_sort;
        perspective.show_completed = self.show_completed;
        perspective.filter = self.filter();
        perspective
    }

    pub fn filter(&self) -> FilterNode {
        match self.tab {
            PerspectiveTab::Simple => self.simple_filter(),
            PerspectiveTab::Comprehensive => self.root.to_filter(),
        }
    }

    pub fn group_label(&self) -> &'static str {
        group_label(self.group)
    }

    pub fn sort_label(&self) -> &'static str {
        sort_label(self.sort)
    }

    pub fn project_group_label(&self) -> &'static str {
        group_label(self.project_group)
    }

    pub fn project_sort_label(&self) -> &'static str {
        sort_label(self.project_sort)
    }

    pub fn status_label(&self) -> &'static str {
        status_label(self.status)
    }

    /// True when the active tab contains a rule that can narrow the library.
    pub fn has_filters(&self) -> bool {
        match self.tab {
            PerspectiveTab::Simple => self.simple_has_filters(),
            PerspectiveTab::Comprehensive => self.root.has_conditions(),
        }
    }

    /// Drop every rule from both tabs. Layout choices are not filters, so they
    /// stay.
    pub fn clear_filters(&mut self) {
        self.root = RuleNode::group(RuleKind::All(Vec::new()), &mut self.next_id);
        self.clear_simple_filters();
        self.editing = None;
        self.add_menu = None;
        self.row_menu = None;
    }

    // ------------------------------------------------------------ simple tab

    fn read_simple_filter(&mut self, filter: &FilterNode) {
        self.clear_simple_filters();
        let FilterNode::All(nodes) = filter else {
            if !filter.is_empty() {
                self.extra_roots.push(filter.clone());
            }
            return;
        };
        for node in nodes {
            match node {
                FilterNode::Condition(condition) => {
                    if !self.absorb(condition) {
                        self.extra.push(condition.clone());
                    }
                }
                FilterNode::Any(nodes) if self.absorb_multi(nodes) => {}
                _ => self.extra_roots.push(node.clone()),
            }
        }
    }

    fn absorb(&mut self, condition: &Condition) -> bool {
        match condition {
            Condition::Flagged { value: true } => self.flagged_only = true,
            Condition::InInbox => self.inbox_only = true,
            Condition::Status { value } => self.status = *value,
            Condition::Due { window } => match DueChoice::from_window(*window) {
                Some(choice) => {
                    self.dues.insert(choice);
                }
                None => return false,
            },
            Condition::Planned { window } => match DueChoice::from_window(*window) {
                Some(choice) => {
                    self.planned.insert(choice);
                }
                None => return false,
            },
            Condition::Deferred { window } => match DeferChoice::from_window(*window) {
                Some(choice) => {
                    self.defers.insert(choice);
                }
                None => return false,
            },
            Condition::Project { id } => {
                self.projects.insert(id.clone());
            }
            Condition::Priority { value } => {
                self.priorities.insert(*value);
            }
            Condition::FullscreenReminder { value } => self.fullscreen = Some(*value),
            Condition::Text { value } => {
                self.text.set(value.clone());
            }
            Condition::Tag {
                id,
                include_children,
            } => {
                self.tags.insert(id.clone());
                self.tag_children = *include_children;
            }
            Condition::Estimate {
                min_minutes,
                max_minutes,
            } => {
                self.estimate_min.set(
                    min_minutes
                        .map(|minutes| minutes.to_string())
                        .unwrap_or_default(),
                );
                self.estimate_max.set(
                    max_minutes
                        .map(|minutes| minutes.to_string())
                        .unwrap_or_default(),
                );
            }
            _ => return false,
        }
        true
    }

    fn absorb_multi(&mut self, nodes: &[FilterNode]) -> bool {
        fn kind(condition: &Condition) -> Option<u8> {
            match condition {
                Condition::Project { .. } => Some(0),
                Condition::Tag { .. } => Some(1),
                Condition::Due { .. } => Some(2),
                Condition::Deferred { .. } => Some(3),
                Condition::Planned { .. } => Some(5),
                Condition::Priority { .. } => Some(4),
                _ => None,
            }
        }
        let conditions: Vec<&Condition> = nodes
            .iter()
            .filter_map(|node| match node {
                FilterNode::Condition(condition) => Some(condition),
                _ => None,
            })
            .collect();
        if conditions.len() != nodes.len() || conditions.len() < 2 {
            return false;
        }
        let Some(first) = kind(conditions[0]) else {
            return false;
        };
        if !conditions.iter().all(|condition| kind(condition) == Some(first)) {
            return false;
        }
        for condition in conditions {
            let _ = self.absorb(condition);
        }
        true
    }

    pub fn simple_filter(&self) -> FilterNode {
        let mut nodes = self.extra_roots.clone();
        let mut conditions = self.extra.clone();
        if self.flagged_only {
            conditions.push(Condition::Flagged { value: true });
        }
        if self.inbox_only {
            conditions.push(Condition::InInbox);
        }
        if self.status != TaskStatus::Any {
            conditions.push(Condition::Status { value: self.status });
        }
        if let Some(value) = self.fullscreen {
            conditions.push(Condition::FullscreenReminder { value });
        }
        let mut grouped: Vec<FilterNode> = Vec::new();
        let mut group = |conditions: Vec<Condition>| {
            let mut nodes: Vec<FilterNode> = conditions
                .into_iter()
                .map(FilterNode::Condition)
                .collect();
            match nodes.len() {
                0 => {}
                1 => grouped.push(nodes.remove(0)),
                _ => grouped.push(FilterNode::Any(nodes)),
            }
        };
        group(
            self.dues
                .iter()
                .map(|choice| Condition::Due {
                    window: choice.window().expect("stored choices always have a window"),
                })
                .collect(),
        );
        group(
            self.planned
                .iter()
                .map(|choice| Condition::Planned {
                    window: choice
                        .window()
                        .expect("stored choices always have a window"),
                })
                .collect(),
        );
        group(
            self.defers
                .iter()
                .map(|choice| Condition::Deferred {
                    window: choice.window().expect("stored choices always have a window"),
                })
                .collect(),
        );
        group(
            self.projects
                .iter()
                .map(|id| Condition::Project { id: id.clone() })
                .collect(),
        );
        group(
            self.tags
                .iter()
                .map(|id| Condition::Tag {
                    id: id.clone(),
                    include_children: self.tag_children,
                })
                .collect(),
        );
        group(
            self.priorities
                .iter()
                .map(|priority| Condition::Priority { value: *priority })
                .collect(),
        );
        let text = self.text.text().trim().to_string();
        if !text.is_empty() {
            conditions.push(Condition::Text { value: text });
        }
        let estimate_min = self.estimate_min.text().trim().parse::<u32>().ok();
        let estimate_max = self.estimate_max.text().trim().parse::<u32>().ok();
        if estimate_min.is_some() || estimate_max.is_some() {
            conditions.push(Condition::Estimate {
                min_minutes: estimate_min,
                max_minutes: estimate_max,
            });
        }
        nodes.extend(conditions.into_iter().map(FilterNode::Condition));
        nodes.extend(grouped);
        FilterNode::all(nodes)
    }

    pub fn toggle_project(&mut self, id: Option<ProjectId>) {
        match id {
            None => self.projects.clear(),
            Some(id) => {
                if !self.projects.remove(&id) {
                    self.projects.insert(id);
                }
            }
        }
        if !self.projects.is_empty() {
            self.inbox_only = false;
        }
    }

    pub fn toggle_tag(&mut self, id: Option<TagId>) {
        match id {
            None => self.tags.clear(),
            Some(id) => {
                if !self.tags.remove(&id) {
                    self.tags.insert(id);
                }
            }
        }
        if !self.tags.is_empty() {
            self.inbox_only = false;
        }
    }

    pub fn toggle_due(&mut self, choice: DueChoice) {
        if choice == DueChoice::Any {
            self.dues.clear();
        } else if !self.dues.remove(&choice) {
            self.dues.insert(choice);
        }
    }

    pub fn toggle_planned(&mut self, choice: DueChoice) {
        if choice == DueChoice::Any {
            self.planned.clear();
        } else if !self.planned.remove(&choice) {
            self.planned.insert(choice);
        }
    }

    pub fn toggle_defer(&mut self, choice: DeferChoice) {
        if choice == DeferChoice::Any {
            self.defers.clear();
        } else if !self.defers.remove(&choice) {
            self.defers.insert(choice);
        }
    }

    pub fn toggle_inbox(&mut self) {
        self.inbox_only = !self.inbox_only;
        if self.inbox_only {
            self.projects.clear();
            self.tags.clear();
        }
    }

    pub fn toggle_priority(&mut self, priority: Priority) {
        if !self.priorities.remove(&priority) {
            self.priorities.insert(priority);
        }
    }

    pub fn simple_has_filters(&self) -> bool {
        self.inbox_only
            || self.flagged_only
            || self.show_completed
            || self.status != TaskStatus::Any
            || self.fullscreen.is_some()
            || !self.dues.is_empty()
            || !self.planned.is_empty()
            || !self.defers.is_empty()
            || !self.projects.is_empty()
            || !self.tags.is_empty()
            || !self.priorities.is_empty()
            || !self.text.text().trim().is_empty()
            || !self.estimate_min.text().trim().is_empty()
            || !self.estimate_max.text().trim().is_empty()
            || !self.extra.is_empty()
            || !self.extra_roots.is_empty()
    }

    pub fn extra_condition_count(&self) -> usize {
        self.extra.len() + self.extra_roots.len()
    }

    fn clear_simple_filters(&mut self) {
        self.inbox_only = false;
        self.flagged_only = false;
        self.show_completed = false;
        self.status = TaskStatus::Any;
        self.fullscreen = None;
        self.dues.clear();
        self.planned.clear();
        self.defers.clear();
        self.projects.clear();
        self.tags.clear();
        self.priorities.clear();
        self.tag_children = true;
        self.text.clear();
        self.estimate_min.clear();
        self.estimate_max.clear();
        self.extra.clear();
        self.extra_roots.clear();
    }

    /// True when the open rule editor contains its own scrolling picker.
    pub fn editor_has_picker(&self) -> bool {
        self.editing
            .and_then(|id| self.rule(id))
            .and_then(RuleNode::condition_ref)
            .is_some_and(|condition| {
                matches!(
                    condition,
                    Condition::TagsAny { .. }
                        | Condition::TagsAll { .. }
                        | Condition::Within { .. }
                        | Condition::Project { .. }
                        | Condition::Folder { .. }
                        | Condition::Tag { .. }
                )
            })
    }

    pub fn is_rule_enabled(&self, id: u64) -> bool {
        find_rule(&self.root, id).is_some_and(|node| node.enabled)
    }

    pub fn rule(&self, id: u64) -> Option<&RuleNode> {
        find_rule(&self.root, id)
    }

    /// A flat, probe-friendly view of the tree.
    pub fn rows(&self, db: &Db) -> Vec<(u64, Option<u64>, String, bool, bool)> {
        fn walk(
            node: &RuleNode,
            parent: Option<u64>,
            db: &Db,
            out: &mut Vec<(u64, Option<u64>, String, bool, bool)>,
        ) {
            out.push((
                node.id,
                parent,
                rule_summary(db, node),
                node.enabled,
                node.kind.is_group(),
            ));
            for child in node.children() {
                walk(child, Some(node.id), db, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, None, db, &mut out);
        out
    }

    pub fn rule_mut(&mut self, id: u64) -> Option<&mut RuleNode> {
        find_rule_mut(&mut self.root, id)
    }

    pub fn add_rule(&mut self, parent: u64, template: RuleTemplate) {
        let rule = template.build(&mut self.next_id);
        if let Some(group) = find_rule_mut(&mut self.root, parent)
            && let Some(children) = group.children_mut()
        {
            children.push(rule);
            self.add_menu = None;
            self.add_after = None;
            self.editing = None;
        }
    }

    pub fn add_rule_after(&mut self, after: u64, template: RuleTemplate) {
        let rule = template.build(&mut self.next_id);
        let Some(children) = sibling_list_mut(&mut self.root, after) else {
            return;
        };
        if let Some(index) = children.iter().position(|child| child.id == after) {
            children.insert(index + 1, rule);
            self.add_menu = None;
            self.add_after = None;
            self.editing = None;
        }
    }

    pub fn delete_rule(&mut self, id: u64) {
        if id == self.root.id {
            return;
        }
        remove_rule(&mut self.root, id);
        if self.editing == Some(id) {
            self.editing = None;
        }
        if self.add_menu == Some(id) {
            self.add_menu = None;
        }
        if self.row_menu == Some(id) {
            self.row_menu = None;
        }
    }

    pub fn duplicate_rule(&mut self, id: u64) {
        let Some(original) = find_rule(&self.root, id).cloned() else {
            return;
        };
        let copy = original.deep_clone(&mut self.next_id);
        let Some(children) = sibling_list_mut(&mut self.root, id) else {
            return;
        };
        if let Some(index) = children.iter().position(|child| child.id == id) {
            children.insert(index + 1, copy);
        }
    }

    pub fn set_rule_enabled(&mut self, id: u64, enabled: bool) {
        if let Some(node) = find_rule_mut(&mut self.root, id) {
            node.enabled = enabled;
        }
    }

    pub fn move_rule(&mut self, id: u64, up: bool) -> bool {
        move_rule(&mut self.root, id, up)
    }

    /// Drag a rule into a group. A group cannot be dropped inside itself or
    /// one of its own descendants.
    pub fn move_into(&mut self, drag: u64, group: u64) -> bool {
        if drag == group || drag == self.root.id {
            return false;
        }
        let Some(dragged) = find_rule(&self.root, drag).cloned() else {
            return false;
        };
        if find_rule(&dragged, group).is_some() {
            return false;
        }
        if !remove_rule(&mut self.root, drag) {
            return false;
        }
        if let Some(target) = find_rule_mut(&mut self.root, group)
            && let Some(children) = target.children_mut()
        {
            children.push(dragged);
            return true;
        }
        false
    }

    /// Drag-and-drop reordering: put `drag` immediately before `target` in the
    /// target's sibling list. Dropping a group inside itself is ignored.
    pub fn move_before(&mut self, drag: u64, target: u64) -> bool {
        if drag == target || drag == self.root.id {
            return false;
        }
        let Some(dragged) = find_rule(&self.root, drag).cloned() else {
            return false;
        };
        if find_rule(&dragged, target).is_some() {
            return false;
        }
        if !remove_rule(&mut self.root, drag) {
            return false;
        }
        let Some(children) = sibling_list_mut(&mut self.root, target) else {
            return false;
        };
        if let Some(index) = children.iter().position(|child| child.id == target) {
            children.insert(index, dragged);
            true
        } else {
            false
        }
    }

    pub fn set_group_operator(&mut self, id: u64, operator: GroupOperator) {
        let Some(node) = find_rule_mut(&mut self.root, id) else {
            return;
        };
        let children = node.children().to_vec();
        node.kind = match operator {
            GroupOperator::All => RuleKind::All(children),
            GroupOperator::Any => RuleKind::Any(children),
            GroupOperator::None => RuleKind::None(children),
        };
    }

    /// Text buffer for the open rule editor. Index 0 is the primary value,
    /// index 1 is the optional second value.
    pub fn editing_buffer(&self, index: usize) -> Option<&TextBuffer> {
        let node = find_rule(&self.root, self.editing?)?;
        match index {
            0 => Some(&node.text),
            1 => Some(&node.text2),
            _ => None,
        }
    }

    pub fn editing_buffer_mut(&mut self, index: usize) -> Option<&mut TextBuffer> {
        let id = self.editing?;
        let node = find_rule_mut(&mut self.root, id)?;
        match index {
            0 => Some(&mut node.text),
            1 => Some(&mut node.text2),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupOperator {
    All,
    Any,
    None,
}

impl GroupOperator {
    pub fn label(self) -> &'static str {
        match self {
            GroupOperator::All => "All",
            GroupOperator::Any => "Any",
            GroupOperator::None => "None",
        }
    }
}

fn root_is_starter(node: &RuleNode) -> bool {
    matches!(
        &node.kind,
        RuleKind::All(children)
            if children.len() == 1
                && matches!(
                    &children[0].kind,
                    RuleKind::Condition(Condition::Availability {
                        value: AvailabilityFilter::Remaining
                    })
                )
    )
}

fn find_rule(node: &RuleNode, id: u64) -> Option<&RuleNode> {
    if node.id == id {
        return Some(node);
    }
    node.children()
        .iter()
        .find_map(|child| find_rule(child, id))
}

fn find_rule_mut(node: &mut RuleNode, id: u64) -> Option<&mut RuleNode> {
    if node.id == id {
        return Some(node);
    }
    node.children_mut()
        .and_then(|children| children.iter_mut().find_map(|child| find_rule_mut(child, id)))
}

fn remove_rule(node: &mut RuleNode, id: u64) -> bool {
    let Some(children) = node.children_mut() else {
        return false;
    };
    let before = children.len();
    children.retain(|child| child.id != id);
    if children.len() != before {
        return true;
    }
    children.iter_mut().any(|child| remove_rule(child, id))
}

fn sibling_list_mut(node: &mut RuleNode, target: u64) -> Option<&mut Vec<RuleNode>> {
    let Some(children) = node.children_mut() else {
        return None;
    };
    if children.iter().any(|child| child.id == target) {
        return Some(children);
    }
    children
        .iter_mut()
        .find_map(|child| sibling_list_mut(child, target))
}

fn move_rule(node: &mut RuleNode, id: u64, up: bool) -> bool {
    let Some(children) = node.children_mut() else {
        return false;
    };
    if let Some(index) = children.iter().position(|child| child.id == id) {
        let target = if up {
            index.checked_sub(1)
        } else if index + 1 < children.len() {
            Some(index + 1)
        } else {
            None
        };
        if let Some(target) = target {
            children.swap(index, target);
            return true;
        }
        return false;
    }
    children.iter_mut().any(|child| move_rule(child, id, up))
}

// ---------------------------------------------------------------- summaries

pub fn rule_summary(db: &Db, node: &RuleNode) -> String {
    match &node.kind {
        RuleKind::All(_) => "All of the following".to_string(),
        RuleKind::Any(_) => "Any of the following".to_string(),
        RuleKind::None(_) => "None of the following".to_string(),
        RuleKind::Comment => {
            let text = node.text.text().trim().to_string();
            if text.is_empty() {
                "Comment".to_string()
            } else {
                format!("Comment: {text}")
            }
        }
        RuleKind::Condition(_) => condition_summary(db, &node.condition()),
    }
}

pub fn condition_summary(db: &Db, condition: &Condition) -> String {
    match condition {
        Condition::Project { id } => format!("Is in project “{}”", db.project_path(id)),
        Condition::Folder { id, .. } => format!("Is in folder “{}”", db.folder_path(id)),
        Condition::Tag {
            id,
            include_children,
        } => {
            let name = db
                .tags
                .get(id)
                .map(|tag| tag.name.clone())
                .unwrap_or_else(|| "Unknown tag".to_string());
            if *include_children {
                format!("Has tag “#{name}” or a child tag")
            } else {
                format!("Has tag “#{name}”")
            }
        }
        Condition::InInbox => "Is in the Inbox".to_string(),
        Condition::Flagged { value: true } => "Is flagged".to_string(),
        Condition::Flagged { value: false } => "Is not flagged".to_string(),
        Condition::Priority { value } => format!("Priority is {}", value.label()),
        Condition::Status { value } => format!("Status is {}", task_status_label(*value)),
        Condition::Due { window } => format!("Due date is {}", window_label(*window)),
        Condition::Deferred { window } => format!("Defer date is {}", window_label(*window)),
        Condition::Planned { window } => format!("Planned date is {}", window_label(*window)),
        Condition::HasRepeat { value: true } => "Is repeating".to_string(),
        Condition::HasRepeat { value: false } => "Is not repeating".to_string(),
        Condition::FullscreenReminder { value: true } => {
            "Has a full-screen reminder".to_string()
        }
        Condition::FullscreenReminder { value: false } => {
            "Has no full-screen reminder".to_string()
        }
        Condition::Estimate {
            min_minutes,
            max_minutes,
        } => match (min_minutes, max_minutes) {
            (None, None) => "Has an estimated duration".to_string(),
            (Some(min), None) => format!("Estimate is at least {min} minutes"),
            (None, Some(max)) => format!("Estimate is at most {max} minutes"),
            (Some(min), Some(max)) => format!("Estimate is {min}–{max} minutes"),
        },
        Condition::Text { value } => format!("Contains “{value}”"),
        Condition::StatusKind { value } => format!("Status is {}", value.label()),
        Condition::Availability { value } => format!("Availability is {}", value.label()),
        Condition::HasEstimate { value: true } => "Has an estimated duration".to_string(),
        Condition::HasEstimate { value: false } => "Has no estimated duration".to_string(),
        Condition::EstimateLessThan { minutes } => {
            format!("Has an estimated duration less than {minutes} minutes")
        }
        Condition::EstimateGreaterThan { minutes } => {
            format!("Has an estimated duration greater than {minutes} minutes")
        }
        Condition::HasDate { field, value } => {
            let verb = if *value { "Has" } else { "Has no" };
            format!("{verb} {}", date_field_phrase(*field))
        }
        Condition::DateRange { field, range } => {
            format!("{} is {}", field.label(), range_label(range))
        }
        Condition::IsUntagged { value: true } => "Is untagged".to_string(),
        Condition::IsUntagged { value: false } => "Is tagged".to_string(),
        Condition::TagStatus { value } => format!("Has a tag that is {}", value.label()),
        Condition::TagsAny {
            ids,
            include_children,
        } => {
            let names = tag_names(db, ids);
            let suffix = if *include_children { " (or child tags)" } else { "" };
            if names.is_empty() {
                "Is tagged with any tag".to_string()
            } else {
                format!("Is tagged with any of: {names}{suffix}")
            }
        }
        Condition::TagsAll {
            ids,
            include_children,
        } => {
            let names = tag_names(db, ids);
            let suffix = if *include_children { " (or child tags)" } else { "" };
            if names.is_empty() {
                "Is tagged with all selected tags".to_string()
            } else {
                format!("Is tagged with all of: {names}{suffix}")
            }
        }
        Condition::ItemKind { value } => format!("Is {}", value.label()),
        Condition::InSingleActionsList => "Is in a single actions list".to_string(),
        Condition::ProjectStatus { value } => format!("Has a project that {}", value.label()),
        Condition::Within {
            project,
            folder,
            include_children,
        } => match (project, folder) {
            (Some(id), _) => format!("Is contained within project “{}”", db.project_path(id)),
            (None, Some(id)) => {
                let suffix = if *include_children {
                    " or a child folder"
                } else {
                    ""
                };
                format!("Is contained within folder “{}”{suffix}", db.folder_path(id))
            }
            (None, None) => "Is contained within any project or folder".to_string(),
        },
        Condition::Search { terms } => {
            if terms.is_empty() {
                "Matches search terms".to_string()
            } else {
                format!("Matches search terms: {}", terms.join(", "))
            }
        }
    }
}

fn tag_names(db: &Db, ids: &[TagId]) -> String {
    ids.iter()
        .filter_map(|id| db.tags.get(id))
        .map(|tag| format!("#{}", tag.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn date_field_phrase(field: DateField) -> &'static str {
    match field {
        DateField::Due => "a due date",
        DateField::Planned => "a planned date",
        DateField::Defer => "a defer date",
        DateField::Completed => "a completed date",
        DateField::Dropped => "a dropped date",
        DateField::Added => "an added date",
        DateField::Changed => "a changed date",
    }
}

fn range_label(range: &DateRangeFilter) -> String {
    match range {
        DateRangeFilter::On { date } => format!("On {date}"),
        DateRangeFilter::Yesterday => "Yesterday".to_string(),
        DateRangeFilter::Today => "Today".to_string(),
        DateRangeFilter::Tomorrow => "Tomorrow".to_string(),
        DateRangeFilter::InPast { amount, unit } => {
            format!("In the past {amount} {}", unit.label())
        }
        DateRangeFilter::InNext { amount, unit } => {
            format!("In the next {amount} {}", unit.label())
        }
        DateRangeFilter::Between { from, to } => match (from, to) {
            (Some(from), Some(to)) => format!("Between {from} and {to}"),
            (Some(from), None) => format!("On or after {from}"),
            (None, Some(to)) => format!("On or before {to}"),
            (None, None) => "Between any dates".to_string(),
        },
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Any => "Any",
        TaskStatus::Available => "Available",
        TaskStatus::Blocked => "Blocked",
        TaskStatus::Deferred => "Deferred",
        TaskStatus::Completed => "Completed",
    }
}

fn window_label(window: DueWindow) -> String {
    match window {
        DueWindow::Overdue => "Overdue".to_string(),
        DueWindow::Today => "Today".to_string(),
        DueWindow::Tomorrow => "Tomorrow".to_string(),
        DueWindow::Next7Days => "Next 7 days".to_string(),
        DueWindow::Next30Days => "Next 30 days".to_string(),
        DueWindow::Later => "Later".to_string(),
        DueWindow::NoDate => "No date".to_string(),
        DueWindow::On(date) => format!("On {date}"),
        DueWindow::Before(date) => format!("Before {date}"),
        DueWindow::After(date) => format!("After {date}"),
    }
}

fn status_slug(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Any => "any",
        TaskStatus::Available => "available",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Deferred => "deferred",
        TaskStatus::Completed => "completed",
    }
}

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Any => "Any",
        TaskStatus::Available => "Available",
        TaskStatus::Blocked => "Blocked",
        TaskStatus::Deferred => "Deferred",
        TaskStatus::Completed => "Completed",
    }
}

fn group_label(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "None",
        GroupBy::Project => "Project",
        GroupBy::Tag => "Tag",
        GroupBy::Folder => "Folder",
        GroupBy::DueDate => "Due",
        GroupBy::DeferDate => "Defer",
        GroupBy::PlannedDate => "Planned",
        GroupBy::CompletedDate => "Completed",
        GroupBy::DroppedDate => "Dropped",
        GroupBy::AddedDate => "Added",
        GroupBy::ChangedDate => "Changed",
        GroupBy::NextReview => "Next Review",
        GroupBy::TagsCombined => "Tags (combined)",
        GroupBy::Flagged => "Flagged",
        GroupBy::Estimate => "Estimate",
    }
}

fn sort_label(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Manual => "Manual",
        SortBy::Due => "Due",
        SortBy::Defer => "Defer",
        SortBy::Planned => "Planned",
        SortBy::Name => "Name",
        SortBy::Priority => "Priority",
        SortBy::Flagged => "Flagged",
        SortBy::Estimate => "Estimate",
        SortBy::Created => "Created",
        SortBy::CompletedAt => "Completed",
        SortBy::DueFlagged => "Due & Flagged",
        SortBy::Tag => "Tag",
        SortBy::AddedDate => "Added",
        SortBy::ChangedDate => "Changed",
        SortBy::DroppedDate => "Dropped",
        SortBy::NextReview => "Next Review",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conditions(filter: &FilterNode) -> Vec<Condition> {
        fn walk(node: &FilterNode, out: &mut Vec<Condition>) {
            match node {
                FilterNode::All(nodes)
                | FilterNode::Any(nodes)
                | FilterNode::None(nodes) => nodes.iter().for_each(|node| walk(node, out)),
                FilterNode::Not(inner) | FilterNode::Disabled(inner) => walk(inner, out),
                FilterNode::Condition(condition) => out.push(condition.clone()),
                FilterNode::Comment(_) => {}
            }
        }
        let mut out = Vec::new();
        walk(filter, &mut out);
        out
    }

    #[test]
    fn a_new_form_starts_with_the_example_rule() {
        let form = PerspectiveForm::new();
        assert_eq!(
            conditions(&RuleNode::default_root(&mut 1).to_filter()),
            vec![Condition::Availability {
                value: AvailabilityFilter::Remaining
            }]
        );
    }

    #[test]
    fn the_tree_round_trips_nested_groups_and_disabled_rules() {
        let mut form = PerspectiveForm::new();
        form.tab = PerspectiveTab::Comprehensive;
        form.clear_filters();
        let all = form.root.id;
        form.add_rule(all, RuleTemplate::GroupAny);
        let any = form.root.children()[0].id;
        form.add_rule(any, RuleTemplate::StatusFlagged);
        form.add_rule(any, RuleTemplate::Inbox);
        let flagged = form.root.children()[0].children()[0].id;
        form.set_rule_enabled(flagged, false);

        let saved = form.to_perspective();
        let edited = PerspectiveForm::edit(&saved);
        let filter = edited.filter();
        match filter {
            FilterNode::All(nodes) => match &nodes[0] {
                FilterNode::Any(children) => {
                    assert!(matches!(
                        &children[0],
                        FilterNode::Disabled(inner)
                            if matches!(**inner, FilterNode::Condition(Condition::StatusKind { .. }))
                    ));
                }
                other => panic!("expected nested any, got {other:?}"),
            },
            other => panic!("expected all root, got {other:?}"),
        }
    }

    #[test]
    fn none_groups_exclude_matching_items() {
        let mut form = PerspectiveForm::new();
        form.tab = PerspectiveTab::Comprehensive;
        form.clear_filters();
        let root = form.root.id;
        form.add_rule(root, RuleTemplate::GroupNone);
        let none = form.root.children()[0].id;
        form.add_rule(none, RuleTemplate::StatusFlagged);

        let filter = form.filter();
        let db = Db::new();
        let task = flowlist_core::model::Task::new("one");
        let at = flowlist_core::time::now();
        assert!(flowlist_core::query::perspective::matches(
            &db, &task, &filter, at
        ));
        let mut flagged = task.clone();
        flagged.flagged = true;
        assert!(!flowlist_core::query::perspective::matches(
            &db, &flagged, &filter, at
        ));
    }

    #[test]
    fn comments_never_match_against_an_any_group() {
        let mut form = PerspectiveForm::new();
        form.tab = PerspectiveTab::Comprehensive;
        form.clear_filters();
        let root = form.root.id;
        form.add_rule(root, RuleTemplate::GroupAny);
        let any = form.root.children()[0].id;
        form.add_rule(any, RuleTemplate::Comment);
        form.add_rule(any, RuleTemplate::Inbox);

        let db = Db::new();
        let task = flowlist_core::model::Task::new("one");
        let at = flowlist_core::time::now();
        assert!(flowlist_core::query::perspective::matches(
            &db,
            &task,
            &form.filter(),
            at
        ));
    }

    #[test]
    fn clearing_a_between_endpoint_makes_it_open_ended() {
        let date = flowlist_core::time::local_date(flowlist_core::time::now());
        let mut id = 1;
        let mut node = RuleNode::from_filter(
            &FilterNode::condition(Condition::DateRange {
                field: DateField::Due,
                range: DateRangeFilter::Between {
                    from: Some(date),
                    to: Some(date),
                },
            }),
            &mut id,
        );
        node.text2.set(String::new());
        assert_eq!(
            node.condition(),
            Condition::DateRange {
                field: DateField::Due,
                range: DateRangeFilter::Between {
                    from: Some(date),
                    to: None,
                },
            }
        );
    }

    #[test]
    fn text_and_estimate_editors_apply_their_buffers() {
        let mut form = PerspectiveForm::new();
        form.tab = PerspectiveTab::Comprehensive;
        form.clear_filters();
        let root = form.root.id;
        form.add_rule(root, RuleTemplate::Search);
        let search = form.root.children()[0].id;
        form.editing = Some(search);
        if let Some(buffer) = form.editing_buffer_mut(0) {
            buffer.set("deep, work");
        }
        let found = conditions(&form.filter());
        assert!(found.contains(&Condition::Search {
            terms: vec!["deep".to_string(), "work".to_string()]
        }));
    }
}
