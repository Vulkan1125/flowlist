//! The custom-perspective editor.
//!
//! OmniFocus exposes one hierarchical contents tree: every rule sits beneath a
//! top-level All, Any or None group, groups nest, rules can be turned off, and
//! comments annotate the tree without affecting the result. This module mirrors
//! that shape. The saved form is still the shared recursive `FilterNode`, so
//! CLI- and MCP-authored perspectives round-trip without loss.

use flowlist_core::db::Db;
use flowlist_core::ids::{PerspectiveId, TagId};
use flowlist_core::model::{
    AvailabilityFilter, Condition, DateField, DateRangeFilter, DueWindow, FilterNode, FilterStatus,
    GroupBy, IconSpec, ItemKindFilter, Perspective, ProjectStatusFilter, SortBy, TagStatusFilter,
    TaskStatus,
};

use super::text::TextBuffer;

/// Which half of the icon card is showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconTab {
    #[default]
    Icons,
    Emoji,
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
    pub const ALL: [RuleTemplate; 24] = [
        RuleTemplate::StatusDueSoon,
        RuleTemplate::StatusFlagged,
        RuleTemplate::Availability,
        RuleTemplate::HasEstimate,
        RuleTemplate::EstimateLessThan,
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
        DateRangeFilter::Between { from, to } => DateRangeFilter::Between {
            from: parse_date_input(text).or(*from),
            to: parse_date_input(text2).or(*to),
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
    pub show_completed: bool,
    pub root: RuleNode,
    /// The rule whose configuration card is open.
    pub editing: Option<u64>,
    /// The group whose Add a new rule menu is open.
    pub add_menu: Option<u64>,
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
            show_completed: false,
            root: RuleNode::default_root(&mut next_id),
            editing: None,
            add_menu: None,
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
        Self {
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
            show_completed: perspective.show_completed,
            root,
            editing: None,
            add_menu: None,
            row_menu: None,
            layout_menu: None,
            next_id,
        }
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
        perspective.show_completed = self.show_completed;
        perspective.filter = self.filter();
        perspective
    }

    pub fn filter(&self) -> FilterNode {
        self.root.to_filter()
    }

    pub fn group_label(&self) -> &'static str {
        group_label(self.group)
    }

    pub fn sort_label(&self) -> &'static str {
        sort_label(self.sort)
    }

    /// True when the tree contains a rule that can narrow the library.
    pub fn has_filters(&self) -> bool {
        self.root.has_conditions()
    }

    /// Drop every rule except the top-level group, which OmniFocus never
    /// deletes. Layout choices are not filters, so they stay.
    pub fn clear_filters(&mut self) {
        self.root = RuleNode::group(
            RuleKind::All(Vec::new()),
            &mut self.next_id,
        );
        self.editing = None;
        self.add_menu = None;
        self.row_menu = None;
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

fn group_label(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "None",
        GroupBy::Project => "Project",
        GroupBy::Tag => "Tag",
        GroupBy::Folder => "Folder",
        GroupBy::DueDate => "Due",
        GroupBy::DeferDate => "Defer",
        GroupBy::PlannedDate => "Planned",
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
            conditions(&form.filter()),
            vec![Condition::Availability {
                value: AvailabilityFilter::Remaining
            }]
        );
    }

    #[test]
    fn the_tree_round_trips_nested_groups_and_disabled_rules() {
        let mut form = PerspectiveForm::new();
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
    fn text_and_estimate_editors_apply_their_buffers() {
        let mut form = PerspectiveForm::new();
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
