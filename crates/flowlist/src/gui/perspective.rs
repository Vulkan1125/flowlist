//! The custom-perspective editor.
//!
//! A perspective is a saved query: a filter tree plus grouping and sort. The
//! GUI exposes the common conditions as buttons, keeps any conditions it does
//! not understand intact, and rebuilds the tree on save. That way editing a
//! CLI-authored perspective never silently drops a condition.

use std::collections::BTreeSet;

use flowlist_core::ids::{PerspectiveId, ProjectId, TagId};
use flowlist_core::model::{
    Condition, DueWindow, FilterNode, GroupBy, IconSpec, Perspective, Priority, SortBy,
    TaskStatus,
};

use super::text::TextBuffer;

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

/// Which half of the icon card is showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconTab {
    #[default]
    Icons,
    Emoji,
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
            ViewChoice::Group(group) => match group {
                GroupBy::None => "none",
                GroupBy::Project => "project",
                GroupBy::Folder => "folder",
                GroupBy::Tag => "tag",
                GroupBy::DueDate => "due",
                GroupBy::DeferDate => "defer",
                GroupBy::PlannedDate => "planned",
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
            }
            .to_string(),
        }
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

/// Everything the editor shows for one perspective.
#[derive(Clone, Debug)]
pub struct PerspectiveForm {
    pub id: Option<PerspectiveId>,
    pub name: TextBuffer,
    pub description: TextBuffer,
    pub text: TextBuffer,
    pub position: u32,
    pub pinned: bool,
    pub icon: Option<IconSpec>,
    pub icon_open: bool,
    pub icon_tab: IconTab,
    pub icon_query: TextBuffer,
    pub group: GroupBy,
    pub sort: SortBy,
    pub show_completed: bool,
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
    /// Conditions the editor has no widget for. They are written back as-is.
    pub extra: Vec<Condition>,
    /// Non-condition roots (nested `any`/`not` trees) kept intact.
    pub extra_roots: Vec<FilterNode>,
}

impl Default for PerspectiveForm {
    fn default() -> Self {
        Self {
            id: None,
            name: TextBuffer::empty(),
            description: TextBuffer::empty(),
            text: TextBuffer::empty(),
            position: 0,
            pinned: false,
            icon: None,
            icon_open: false,
            icon_tab: IconTab::Icons,
            icon_query: TextBuffer::empty(),
            group: GroupBy::Project,
            sort: SortBy::Due,
            show_completed: false,
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
        }
    }
}

impl PerspectiveForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn edit(perspective: &Perspective) -> Self {
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
            show_completed: perspective.show_completed,
            ..Self::default()
        };
        form.read_filter(&perspective.filter);
        form
    }

    fn read_filter(&mut self, filter: &FilterNode) {
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

    /// Read one condition into a widget. Returns false when the editor has no
    /// control for it, so the caller keeps it in `extra`.
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

    /// Absorb an `any` group made only of the multi-select condition kinds
    /// the editor understands. Mixed or unknown groups stay verbatim.
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

    pub fn filter(&self) -> FilterNode {
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

    // ------------------------------------------------------------ controls

    /// Picking a project means "not the inbox"; the same applies to tags.
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

    pub fn group_label(&self) -> &'static str {
        group_label(self.group)
    }

    pub fn sort_label(&self) -> &'static str {
        sort_label(self.sort)
    }

    pub fn status_label(&self) -> &'static str {
        status_label(self.status)
    }

    /// True when the filter would narrow the library at all, so the editor can
    /// offer Clear filters only when there is something to clear.
    pub fn has_filters(&self) -> bool {
        self.inbox_only
            || self.flagged_only
            || self.show_completed
            || self.status != TaskStatus::Any
            || self.fullscreen.is_some()
            || !self.dues.is_empty()
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

    /// Conditions the editor cannot show. They are kept on save, and named in
    /// the sheet so an imported view never looks simpler than it is.
    pub fn extra_condition_count(&self) -> usize {
        self.extra.len() + self.extra_roots.len()
    }

    /// Drop every condition, including the CLI-only ones. This is an explicit
    /// action in the editor, never an implicit side effect of saving.
    pub fn clear_filters(&mut self) {
        self.inbox_only = false;
        self.flagged_only = false;
        self.show_completed = false;
        self.status = TaskStatus::Any;
        self.fullscreen = None;
        self.dues.clear();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition_set(filter: &FilterNode) -> Vec<Condition> {
        match filter {
            FilterNode::All(nodes) => nodes
                .iter()
                .filter_map(|node| match node {
                    FilterNode::Condition(condition) => Some(condition.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn a_new_form_saves_the_chosen_conditions() {
        let mut form = PerspectiveForm::new();
        form.name = TextBuffer::new("Focus");
        form.flagged_only = true;
        form.status = TaskStatus::Available;
        form.dues.insert(DueChoice::Today);
        form.text.set("focus");
        form.toggle_priority(Priority::High);
        form.tag_children = false;
        let tag = TagId::new();
        form.toggle_tag(Some(tag.clone()));

        let perspective = form.to_perspective();
        assert_eq!(perspective.name, "Focus");
        let conditions = condition_set(&perspective.filter);
        assert!(conditions.contains(&Condition::Flagged { value: true }));
        assert!(conditions.contains(&Condition::Status {
            value: TaskStatus::Available
        }));
        assert!(conditions.contains(&Condition::Due {
            window: DueWindow::Today
        }));
        assert!(conditions.contains(&Condition::Tag {
            id: tag,
            include_children: false,
        }));
        assert!(conditions.contains(&Condition::Text {
            value: "focus".to_string()
        }));
        assert!(conditions.contains(&Condition::Priority {
            value: Priority::High
        }));
    }

    #[test]
    fn fullscreen_filter_round_trips() {
        let mut form = PerspectiveForm::new();
        form.name = TextBuffer::new("Alerts");
        form.fullscreen = Some(true);
        let perspective = form.to_perspective();
        assert_eq!(
            condition_set(&perspective.filter),
            vec![Condition::FullscreenReminder { value: true }]
        );

        let edited = PerspectiveForm::edit(&perspective);
        assert_eq!(edited.fullscreen, Some(true));
        assert_eq!(edited.extra_condition_count(), 0);
    }

    #[test]
    fn editing_round_trips_known_and_unknown_conditions() {
        let mut original = Perspective::new("Deep work");
        original.filter = FilterNode::all_conditions(vec![
            Condition::Flagged { value: true },
            Condition::Text {
                value: "deep".to_string(),
            },
            Condition::Estimate {
                min_minutes: Some(30),
                max_minutes: None,
            },
            Condition::HasRepeat { value: true },
        ]);

        let form = PerspectiveForm::edit(&original);
        assert!(form.flagged_only);
        assert_eq!(form.text.text(), "deep");
        assert_eq!(form.estimate_min.text(), "30");
        assert!(form.estimate_max.text().is_empty());
        // Unknown conditions are preserved.
        assert_eq!(form.extra.len(), 1);

        let saved = form.to_perspective();
        assert_eq!(saved.id, original.id);
        assert_eq!(saved.name, original.name);
        let conditions = condition_set(&saved.filter);
        assert!(conditions.contains(&Condition::Flagged { value: true }));
        assert!(conditions.contains(&Condition::Text {
            value: "deep".to_string()
        }));
        assert!(conditions.contains(&Condition::Estimate {
            min_minutes: Some(30),
            max_minutes: None
        }));
        assert!(conditions.contains(&Condition::HasRepeat { value: true }));
    }

    #[test]
    fn clear_filters_drops_known_and_unknown_conditions() {
        let mut original = Perspective::new("Mixed");
        original.filter = FilterNode::all_conditions(vec![
            Condition::Flagged { value: true },
            Condition::Status {
                value: TaskStatus::Available,
            },
            Condition::Text {
                value: "deep".to_string(),
            },
            Condition::Estimate {
                min_minutes: Some(30),
                max_minutes: None,
            },
            Condition::HasRepeat { value: true },
        ]);
        let mut form = PerspectiveForm::edit(&original);
        assert!(form.has_filters());
        assert_eq!(form.extra_condition_count(), 1);

        form.clear_filters();
        assert!(!form.has_filters());
        assert_eq!(form.extra_condition_count(), 0);
        assert_eq!(form.status, TaskStatus::Any);
        // Layout choices are not filters, so clearing keeps them.
        assert_eq!(form.group, original.group_by);
        assert_eq!(form.sort, original.sort);
        assert!(condition_set(&form.to_perspective().filter).is_empty());
    }

    #[test]
    fn estimate_max_becomes_a_filter() {
        let mut form = PerspectiveForm::new();
        form.estimate_max.set("15");
        let conditions = condition_set(&form.to_perspective().filter);
        assert!(conditions.contains(&Condition::Estimate {
            min_minutes: None,
            max_minutes: Some(15),
        }));
    }

    #[test]
    fn multi_select_groups_round_trip() {
        let mut form = PerspectiveForm::new();
        let first = ProjectId::new();
        let second = ProjectId::new();
        form.toggle_project(Some(first.clone()));
        form.toggle_project(Some(second.clone()));
        form.toggle_due(DueChoice::Today);
        form.toggle_due(DueChoice::Next7Days);

        let saved = form.to_perspective();
        let edited = PerspectiveForm::edit(&saved);
        assert_eq!(
            edited.projects,
            BTreeSet::from([first, second])
        );
        assert!(edited.dues.contains(&DueChoice::Today));
        assert!(edited.dues.contains(&DueChoice::Next7Days));
    }

    #[test]
    fn has_filters_ignores_layout_only_forms() {
        let mut form = PerspectiveForm::new();
        assert!(!form.has_filters());
        form.group = GroupBy::Tag;
        form.sort = SortBy::Name;
        assert!(!form.has_filters());
        form.defers.insert(DeferChoice::Today);
        assert!(form.has_filters());
    }

    #[test]
    fn choosing_inbox_clears_project_and_tag() {
        let mut form = PerspectiveForm::new();
        form.toggle_project(Some(ProjectId::new()));
        form.toggle_inbox();
        assert!(form.inbox_only);
        assert!(form.projects.is_empty());
        form.toggle_tag(Some(TagId::new()));
        assert!(!form.inbox_only);
    }

    #[test]
    fn nested_filters_survive_editing() {
        let mut original = Perspective::new("Nested");
        let nested = FilterNode::any(vec![
            FilterNode::condition(Condition::Flagged { value: true }),
            FilterNode::condition(Condition::InInbox),
        ]);
        original.filter = FilterNode::all(vec![nested.clone()]);

        let form = PerspectiveForm::edit(&original);
        let saved = form.to_perspective();
        match &saved.filter {
            FilterNode::All(nodes) => assert!(nodes.contains(&nested)),
            other => panic!("expected an all-root, got {other:?}"),
        }
    }

    #[test]
    fn planned_windows_round_trip_through_the_editor() {
        let mut form = PerspectiveForm::new();
        form.name.set("Planned today");
        form.toggle_planned(DueChoice::Today);
        let perspective = form.to_perspective();
        assert!(matches!(
            &perspective.filter,
            FilterNode::All(nodes) if nodes.iter().any(|node| matches!(
                node,
                FilterNode::Condition(Condition::Planned { .. })
            ))
        ));
        let rebuilt = PerspectiveForm::edit(&perspective);
        assert!(rebuilt.planned.contains(&DueChoice::Today));
    }

    #[test]
    fn editing_a_pinned_view_keeps_it_pinned_and_keeps_its_icon() {
        let mut original = Perspective::new("Running");
        original.pinned = true;
        original.icon = Some(IconSpec::builtin(flowlist_core::model::ProjectIcon::Star));

        let form = PerspectiveForm::edit(&original);
        assert!(form.pinned);
        let saved = form.to_perspective();
        assert!(saved.pinned);
        assert_eq!(
            saved.icon,
            Some(IconSpec::builtin(flowlist_core::model::ProjectIcon::Star))
        );
    }
}
