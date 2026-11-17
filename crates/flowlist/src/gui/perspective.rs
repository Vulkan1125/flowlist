//! The custom-perspective editor.
//!
//! A perspective is a saved query: a filter tree plus grouping and sort. The
//! GUI exposes the common conditions as buttons, keeps any conditions it does
//! not understand intact, and rebuilds the tree on save. That way editing a
//! CLI-authored perspective never silently drops a condition.

use flowlist_core::ids::{PerspectiveId, ProjectId, TagId};
use flowlist_core::model::{
    Condition, DueWindow, FilterNode, GroupBy, Perspective, SortBy, TaskStatus,
};

use super::text::TextBuffer;

/// Due windows offered by the editor. The CLI and MCP server can still write
/// the date-based variants; they survive editing untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueChoice {
    Any,
    Overdue,
    Today,
    Next7Days,
    NoDate,
}

impl DueChoice {
    pub fn label(self) -> &'static str {
        match self {
            DueChoice::Any => "Any",
            DueChoice::Overdue => "Overdue",
            DueChoice::Today => "Today",
            DueChoice::Next7Days => "Next 7 days",
            DueChoice::NoDate => "No date",
        }
    }

    pub fn window(self) -> Option<DueWindow> {
        match self {
            DueChoice::Any => None,
            DueChoice::Overdue => Some(DueWindow::Overdue),
            DueChoice::Today => Some(DueWindow::Today),
            DueChoice::Next7Days => Some(DueWindow::Next7Days),
            DueChoice::NoDate => Some(DueWindow::NoDate),
        }
    }

    fn from_window(window: DueWindow) -> Option<Self> {
        match window {
            DueWindow::Overdue => Some(DueChoice::Overdue),
            DueWindow::Today => Some(DueChoice::Today),
            DueWindow::Next7Days => Some(DueChoice::Next7Days),
            DueWindow::NoDate => Some(DueChoice::NoDate),
            _ => None,
        }
    }
}

/// Deferred tasks are never overdue, so the editor leaves that window out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferChoice {
    Any,
    Today,
    Next7Days,
    NoDate,
}

impl DeferChoice {
    pub fn label(self) -> &'static str {
        match self {
            DeferChoice::Any => "Any",
            DeferChoice::Today => "Today",
            DeferChoice::Next7Days => "Next 7 days",
            DeferChoice::NoDate => "No date",
        }
    }

    pub fn window(self) -> Option<DueWindow> {
        match self {
            DeferChoice::Any => None,
            DeferChoice::Today => Some(DueWindow::Today),
            DeferChoice::Next7Days => Some(DueWindow::Next7Days),
            DeferChoice::NoDate => Some(DueWindow::NoDate),
        }
    }

    fn from_window(window: DueWindow) -> Option<Self> {
        match window {
            DueWindow::Today => Some(DeferChoice::Today),
            DueWindow::Next7Days => Some(DeferChoice::Next7Days),
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
    Defer,
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
            ViewField::Defer => "defer",
            ViewField::Group => "group",
            ViewField::Sort => "sort",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewField::Project => "Project",
            ViewField::Tag => "Tag",
            ViewField::Status => "Status",
            ViewField::Due => "Due",
            ViewField::Defer => "Defer",
            ViewField::Group => "Group by",
            ViewField::Sort => "Sort by",
        }
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
    Defer(DeferChoice),
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
                DueChoice::Next7Days => "next7",
                DueChoice::NoDate => "none",
            }
            .to_string(),
            ViewChoice::Defer(choice) => match choice {
                DeferChoice::Any => "any",
                DeferChoice::Today => "today",
                DeferChoice::Next7Days => "next7",
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
                GroupBy::Flagged => "flagged",
                GroupBy::Estimate => "estimate",
            }
            .to_string(),
            ViewChoice::Sort(sort) => match sort {
                SortBy::Manual => "manual",
                SortBy::Due => "due",
                SortBy::Defer => "defer",
                SortBy::Name => "name",
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
        GroupBy::Flagged => "Flagged",
        GroupBy::Estimate => "Estimate",
    }
}

fn sort_label(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Manual => "Manual",
        SortBy::Due => "Due",
        SortBy::Defer => "Defer",
        SortBy::Name => "Name",
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
    pub position: u32,
    pub group: GroupBy,
    pub sort: SortBy,
    pub show_completed: bool,
    pub flagged_only: bool,
    pub inbox_only: bool,
    pub status: TaskStatus,
    pub due: DueChoice,
    pub defer: DeferChoice,
    pub project: Option<ProjectId>,
    pub tag: Option<TagId>,
    pub tag_children: bool,
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
            position: 0,
            group: GroupBy::Project,
            sort: SortBy::Due,
            show_completed: false,
            flagged_only: false,
            inbox_only: false,
            status: TaskStatus::Any,
            due: DueChoice::Any,
            defer: DeferChoice::Any,
            project: None,
            tag: None,
            tag_children: true,
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
            position: perspective.position,
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
            let FilterNode::Condition(condition) = node else {
                self.extra_roots.push(node.clone());
                continue;
            };
            if !self.absorb(condition) {
                self.extra.push(condition.clone());
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
                Some(choice) => self.due = choice,
                None => return false,
            },
            Condition::Deferred { window } => match DeferChoice::from_window(*window) {
                Some(choice) => self.defer = choice,
                None => return false,
            },
            Condition::Project { id } => self.project = Some(id.clone()),
            Condition::Tag {
                id,
                include_children,
            } => {
                self.tag = Some(id.clone());
                self.tag_children = *include_children;
            }
            _ => return false,
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
        if let Some(window) = self.due.window() {
            conditions.push(Condition::Due { window });
        }
        if let Some(window) = self.defer.window() {
            conditions.push(Condition::Deferred { window });
        }
        if let Some(id) = &self.project {
            conditions.push(Condition::Project { id: id.clone() });
        }
        if let Some(id) = &self.tag {
            conditions.push(Condition::Tag {
                id: id.clone(),
                include_children: self.tag_children,
            });
        }
        nodes.extend(conditions.into_iter().map(FilterNode::Condition));
        FilterNode::all(nodes)
    }

    pub fn to_perspective(&self) -> Perspective {
        let name = self.name.text().trim().to_string();
        let mut perspective = Perspective::new(name);
        if let Some(id) = &self.id {
            perspective.id = id.clone();
        }
        perspective.position = self.position;
        perspective.group_by = self.group;
        perspective.sort = self.sort;
        perspective.show_completed = self.show_completed;
        perspective.filter = self.filter();
        perspective
    }

    // ------------------------------------------------------------ controls

    /// Picking a project means "not the inbox"; the same applies to tags.
    pub fn set_project(&mut self, id: Option<ProjectId>) {
        self.project = id;
        if self.project.is_some() {
            self.inbox_only = false;
        }
    }

    pub fn set_tag(&mut self, id: Option<TagId>) {
        self.tag = id;
        if self.tag.is_some() {
            self.inbox_only = false;
        }
    }

    pub fn toggle_inbox(&mut self) {
        self.inbox_only = !self.inbox_only;
        if self.inbox_only {
            self.project = None;
            self.tag = None;
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
            || self.due.window().is_some()
            || self.defer.window().is_some()
            || self.project.is_some()
            || self.tag.is_some()
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
        self.due = DueChoice::Any;
        self.defer = DeferChoice::Any;
        self.project = None;
        self.tag = None;
        self.tag_children = true;
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
        form.due = DueChoice::Today;
        form.tag_children = false;
        let tag = TagId::new();
        form.set_tag(Some(tag.clone()));

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
        ]);

        let form = PerspectiveForm::edit(&original);
        assert!(form.flagged_only);
        // Unknown conditions are preserved.
        assert_eq!(form.extra.len(), 2);

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
        ]);
        let mut form = PerspectiveForm::edit(&original);
        assert!(form.has_filters());
        assert_eq!(form.extra_condition_count(), 2);

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
    fn has_filters_ignores_layout_only_forms() {
        let mut form = PerspectiveForm::new();
        assert!(!form.has_filters());
        form.group = GroupBy::Tag;
        form.sort = SortBy::Name;
        assert!(!form.has_filters());
        form.defer = DeferChoice::Today;
        assert!(form.has_filters());
    }

    #[test]
    fn choosing_inbox_clears_project_and_tag() {
        let mut form = PerspectiveForm::new();
        form.set_project(Some(ProjectId::new()));
        form.toggle_inbox();
        assert!(form.inbox_only);
        assert!(form.project.is_none());
        form.set_tag(Some(TagId::new()));
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
}
