//! Command-line interface. Every mutation goes through the same `Store` and
//! `Command` engine as the GUI, so the CLI is fully scriptable and undoable.

use std::io::{IsTerminal, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use serde_json::json;

use flowlist_core::command::{
    Command as CoreCommand, NewProject, ProjectPatch, SettingsPatch, TagPatch, TaskPatch,
    TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Db, Position};
use flowlist_core::ids::{
    ChecklistId, ContainerId, FolderId, PerspectiveId, ProjectId, TagId, TaskId,
};
use flowlist_core::model::{
    BuiltinPerspective, Condition, DueWindow, FilterNode, GroupBy, Perspective, Priority,
    ProjectKind, ProjectStatus, RepeatRule, SortBy, TagStatus, TaskStatus,
};
use flowlist_core::parse::{
    parse_duration, parse_quick, parse_repeat, parse_stamp, with_default_time,
};
use flowlist_core::paths::DataPaths;
use flowlist_core::persist;
use flowlist_core::query::{self, Availability, Group};
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, now};
use flowlist_core::{export, search, stats, taskpaper};

use crate::render;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "flowlist",
    version,
    about = "A fast, local-first task manager",
    long_about = "Flowlist keeps your tasks in one JSON library shared by the GUI, CLI and MCP server. \
Add tasks with quick-entry syntax, work in perspectives, and undo anything."
)]
pub struct Cli {
    /// Library directory (defaults to the platform data directory).
    #[arg(long, global = true, env = "FLOWLIST_HOME", value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommand {
    /// Add a task using quick-entry syntax.
    #[command(alias = "capture", alias = "a")]
    Add(AddArgs),
    /// List tasks from a perspective, project, tag or saved filter.
    #[command(alias = "ls")]
    List(ListArgs),
    /// Mark tasks complete (or incomplete with --undo).
    #[command(alias = "complete")]
    Done(DoneArgs),
    /// Edit a task's fields.
    Edit(EditArgs),
    /// Move tasks to Trash (or permanently delete them with `trash`).
    #[command(alias = "rm", alias = "delete")]
    Remove(RemoveArgs),
    /// Inspect Trash, restore tasks, delete them forever, or empty it.
    Trash(TrashArgs),
    /// Move tasks between the inbox, projects and parents.
    #[command(alias = "move")]
    Mv(MvArgs),
    /// Manage checklist items inside a task.
    #[command(alias = "check")]
    Checklist(ChecklistArgs),
    /// Undo the last change.
    Undo(UndoArgs),
    /// Redo an undone change.
    Redo(UndoArgs),
    /// Manage tags (contexts).
    Tag(TagArgs),
    /// List tags.
    Tags,
    /// Manage projects.
    #[command(alias = "proj")]
    Project(ProjectArgs),
    /// Manage folders.
    Folder(FolderArgs),
    /// Manage custom perspectives.
    Perspective(PerspectiveArgs),
    /// Review projects that are due.
    Review(ReviewArgs),
    /// Search tasks, projects, folders, tags and perspectives.
    Search(SearchArgs),
    /// Print library counters.
    Stats,
    /// Export the library as JSON or TaskPaper.
    Export(ExportArgs),
    /// Import a JSON or TaskPaper library.
    Import(ImportArgs),
    /// Diagnose (and optionally repair) the library.
    Doctor(DoctorArgs),
    /// Print the library location.
    Where,
    /// Run the MCP server on stdio.
    Mcp,
    /// Open the graphical application.
    Gui,
}

#[derive(Args, Debug, Clone, Default)]
pub struct AddArgs {
    /// Task text; supports `#tag`, `::Project`, `due:`, `est:`, `!` and `// note`.
    #[arg(value_name = "TEXT")]
    pub text: Vec<String>,
    /// File into a project (a path like `Work/Website` is created if needed).
    #[arg(long, short = 'p')]
    pub project: Option<String>,
    /// Attach a tag; repeat for several tags.
    #[arg(long = "tag", short = 't')]
    pub tags: Vec<String>,
    #[arg(long)]
    pub due: Option<String>,
    #[arg(long)]
    pub defer: Option<String>,
    #[arg(long)]
    pub estimate: Option<String>,
    #[arg(long)]
    pub repeat: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub flag: bool,
    /// low | medium | high.
    #[arg(long)]
    pub priority: Option<String>,
    /// Make this a subtask of an existing task.
    #[arg(long)]
    pub parent: Option<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ListArgs {
    #[arg(long)]
    pub inbox: bool,
    #[arg(long)]
    pub flagged: bool,
    #[arg(long)]
    pub forecast: bool,
    #[arg(long)]
    pub review: bool,
    #[arg(long)]
    pub completed: bool,
    /// List tasks currently in Trash.
    #[arg(long)]
    pub trash: bool,
    /// low | medium | high.
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub available: bool,
    #[arg(long = "tag", short = 't')]
    pub tag: Vec<String>,
    #[arg(long, short = 'p')]
    pub project: Option<String>,
    #[arg(long)]
    pub folder: Option<String>,
    #[arg(long, short = 'P')]
    pub perspective: Option<String>,
    #[arg(long, short = 's')]
    pub search: Option<String>,
    /// Filter by due window or date (`today`, `overdue`, `next7`, `2026-12-01`).
    #[arg(long)]
    pub due: Option<String>,
    /// manual | due | defer | name | flagged | estimate | created | completed
    #[arg(long, default_value = "manual")]
    pub sort: String,
    /// none | project | folder | tag | due | defer | flagged | estimate
    #[arg(long)]
    pub group: Option<String>,
    #[arg(long)]
    pub limit: Option<usize>,
    #[arg(long)]
    pub include_completed: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DoneArgs {
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
    /// Mark incomplete instead.
    #[arg(long)]
    pub undo: bool,
}

#[derive(Args, Debug, Clone)]
pub struct EditArgs {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub notes: Option<String>,
    /// A date phrase, or `none` to clear.
    #[arg(long)]
    pub due: Option<String>,
    #[arg(long)]
    pub defer: Option<String>,
    /// `30m`, `1h30m`, or `none`.
    #[arg(long)]
    pub estimate: Option<String>,
    #[arg(long)]
    pub flag: bool,
    #[arg(long)]
    pub unflag: bool,
    /// A repeat rule, or `none`.
    #[arg(long)]
    pub repeat: Option<String>,
    /// Project path, or `inbox` to clear.
    #[arg(long, short = 'p')]
    pub project: Option<String>,
    /// Parent task id, or `none` to clear.
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub sequential: bool,
    /// low | medium | high; `none` clears it.
    #[arg(long)]
    pub priority: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct TrashArgs {
    /// Restore these task ids (and their subtrees).
    #[arg(long, value_name = "ID")]
    pub restore: Vec<String>,
    /// Permanently delete these task ids (and their subtrees).
    #[arg(long, value_name = "ID")]
    pub delete: Vec<String>,
    /// Permanently delete every trashed task.
    #[arg(long)]
    pub empty: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RemoveArgs {
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct MvArgs {
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
    #[arg(long, short = 'p')]
    pub project: Option<String>,
    #[arg(long)]
    pub folder: Option<String>,
    #[arg(long)]
    pub parent: Option<String>,
    #[arg(long)]
    pub inbox: bool,
    #[arg(long)]
    pub before: Option<String>,
    #[arg(long)]
    pub after: Option<String>,
    #[arg(long)]
    pub start: bool,
}

#[derive(Args, Debug, Clone)]
pub struct UndoArgs {
    #[arg(long, default_value_t = 1)]
    pub steps: usize,
}

#[derive(Args, Debug, Clone)]
pub struct ChecklistArgs {
    #[command(subcommand)]
    pub command: ChecklistCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ChecklistCommand {
    /// List the checklist items of a task.
    #[command(alias = "ls")]
    List {
        #[arg(value_name = "TASK")]
        task: String,
        /// Hide completed items.
        #[arg(long)]
        hide_done: bool,
    },
    /// Append a checklist item (or insert it before an index).
    Add {
        #[arg(value_name = "TASK")]
        task: String,
        #[arg(value_name = "TITLE", required = true)]
        title: Vec<String>,
        /// Zero-based index to insert before.
        #[arg(long)]
        before: Option<usize>,
    },
    /// Rename an item or mark it complete/incomplete.
    #[command(alias = "edit")]
    Set {
        #[arg(value_name = "TASK")]
        task: String,
        #[arg(value_name = "ITEM")]
        item: String,
        #[arg(long)]
        title: Option<String>,
        /// Mark the item complete.
        #[arg(long)]
        done: bool,
        /// Mark the item incomplete.
        #[arg(long)]
        undone: bool,
    },
    /// Move an item to a zero-based index.
    #[command(alias = "mv")]
    Move {
        #[arg(value_name = "TASK")]
        task: String,
        #[arg(value_name = "ITEM")]
        item: String,
        #[arg(long)]
        to: usize,
    },
    /// Remove a checklist item.
    #[command(alias = "rm")]
    Remove {
        #[arg(value_name = "TASK")]
        task: String,
        #[arg(value_name = "ITEM")]
        item: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct TagArgs {
    #[command(subcommand)]
    pub command: TagCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TagCommand {
    /// Create a tag (nested paths are created as needed).
    Add(TagAddArgs),
    /// Delete a tag.
    #[command(alias = "rm")]
    Remove { id: String },
    /// Edit a tag.
    Set(TagSetArgs),
}

#[derive(Args, Debug, Clone)]
pub struct TagAddArgs {
    #[arg(value_name = "PATH")]
    pub path: String,
    #[arg(long)]
    pub parent: Option<String>,
    /// available | blocked | on-hold
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub lat: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    pub lon: Option<f64>,
}

#[derive(Args, Debug, Clone)]
pub struct TagSetArgs {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    /// Parent tag path, or `none`.
    #[arg(long)]
    pub parent: Option<String>,
    /// available | blocked | on-hold
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pub lat: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    pub lon: Option<f64>,
    #[arg(long)]
    pub clear_location: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub command: ProjectCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProjectCommand {
    Add(ProjectAddArgs),
    /// List projects.
    #[command(alias = "ls")]
    List {
        #[arg(long)]
        closed: bool,
    },
    /// Set active | on-hold | done | dropped.
    Status {
        id: String,
        status: String,
    },
    /// Mark a project reviewed.
    Review {
        id: String,
        #[arg(long)]
        on: Option<String>,
    },
    /// Edit project fields.
    Edit(ProjectEditArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ProjectAddArgs {
    #[arg(value_name = "NAME")]
    pub name: String,
    #[arg(long)]
    pub folder: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long)]
    pub due: Option<String>,
    #[arg(long)]
    pub defer: Option<String>,
    #[arg(long)]
    pub review_days: Option<u32>,
    #[arg(long)]
    pub sequential: bool,
    /// parallel | sequential | single_actions.
    #[arg(long)]
    pub kind: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectEditArgs {
    #[arg(value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    /// Folder path, or `none`.
    #[arg(long)]
    pub folder: Option<String>,
    #[arg(long)]
    pub due: Option<String>,
    #[arg(long)]
    pub defer: Option<String>,
    #[arg(long)]
    pub review_days: Option<u32>,
    #[arg(long)]
    pub next_review: Option<String>,
    #[arg(long)]
    pub sequential: bool,
    /// parallel | sequential | single_actions.
    #[arg(long)]
    pub kind: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct FolderArgs {
    #[command(subcommand)]
    pub command: FolderCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum FolderCommand {
    Add {
        path: String,
    },
    #[command(alias = "ls")]
    List,
    Rename {
        id: String,
        name: String,
    },
    /// Delete a folder; its projects move up a level.
    #[command(alias = "rm")]
    Remove {
        id: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct PerspectiveArgs {
    #[command(subcommand)]
    pub command: PerspectiveCommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum PerspectiveCommand {
    Add(PerspectiveAddArgs),
    #[command(alias = "ls")]
    List,
    #[command(alias = "rm")]
    Remove {
        id: String,
    },
}

#[derive(Args, Debug, Clone)]
pub struct PerspectiveAddArgs {
    #[arg(value_name = "NAME")]
    pub name: String,
    /// A JSON filter tree, e.g. '{"type":"condition","condition":{"type":"flagged","value":true}}'
    #[arg(long)]
    pub filter: Option<String>,
    /// none | project | folder | tag | due | defer | flagged | estimate
    #[arg(long)]
    pub group_by: Option<String>,
    #[arg(long)]
    pub sort: Option<String>,
    #[arg(long)]
    pub show_completed: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ReviewArgs {
    #[command(subcommand)]
    pub command: Option<ReviewCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ReviewCommand {
    Done {
        id: String,
        #[arg(long)]
        on: Option<String>,
    },
}

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    #[arg(value_name = "QUERY")]
    pub query: String,
    #[arg(long, default_value_t = 25)]
    pub limit: usize,
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    /// json | taskpaper
    #[arg(long, default_value = "taskpaper")]
    pub format: String,
    /// Write to a file instead of stdout.
    #[arg(long, short = 'o')]
    pub output: Option<PathBuf>,
    /// Write a timestamped copy of data.json into the backups directory.
    #[arg(long)]
    pub backup: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ImportArgs {
    #[arg(value_name = "FILE")]
    pub file: String,
    /// auto | json | taskpaper
    #[arg(long, default_value = "auto")]
    pub format: String,
    /// Merge into the current library instead of replacing it.
    #[arg(long)]
    pub merge: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DoctorArgs {
    /// Repair dangling references and re-index ordering arrays.
    #[arg(long)]
    pub fix: bool,
}

pub fn run() -> i32 {
    match try_run() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("\x1b[31merror:\x1b[0m {error:#}");
            1
        }
    }
}

fn try_run() -> Result<()> {
    let cli = Cli::parse();
    let paint = Paint::new(cli.json);
    let Some(command) = cli.command.clone() else {
        return start_gui(&cli);
    };

    match command {
        CliCommand::Add(args) => cmd_add(&cli, args, &paint),
        CliCommand::List(args) => cmd_list(&cli, args, &paint),
        CliCommand::Done(args) => cmd_done(&cli, args, &paint),
        CliCommand::Edit(args) => cmd_edit(&cli, args, &paint),
        CliCommand::Remove(args) => cmd_remove(&cli, args, &paint),
        CliCommand::Trash(args) => cmd_trash(&cli, args, &paint),
        CliCommand::Mv(args) => cmd_mv(&cli, args, &paint),
        CliCommand::Checklist(args) => cmd_checklist(&cli, args, &paint),
        CliCommand::Undo(args) => cmd_undo(&cli, args, false, &paint),
        CliCommand::Redo(args) => cmd_undo(&cli, args, true, &paint),
        CliCommand::Tag(args) => cmd_tag(&cli, args, &paint),
        CliCommand::Tags => cmd_tags(&cli, &paint),
        CliCommand::Project(args) => cmd_project(&cli, args, &paint),
        CliCommand::Folder(args) => cmd_folder(&cli, args, &paint),
        CliCommand::Perspective(args) => cmd_perspective(&cli, args, &paint),
        CliCommand::Review(args) => cmd_review(&cli, args, &paint),
        CliCommand::Search(args) => cmd_search(&cli, args, &paint),
        CliCommand::Stats => cmd_stats(&cli, &paint),
        CliCommand::Export(args) => cmd_export(&cli, args),
        CliCommand::Import(args) => cmd_import(&cli, args, &paint),
        CliCommand::Doctor(args) => cmd_doctor(&cli, args, &paint),
        CliCommand::Where => cmd_where(&cli, &paint),
        CliCommand::Mcp => {
            let paths = DataPaths::discover(cli.data_dir.clone())?;
            crate::mcp::serve(paths)
        }
        CliCommand::Gui => start_gui(&cli),
    }
}

fn start_gui(cli: &Cli) -> Result<()> {
    #[cfg(feature = "gui")]
    {
        let paths = DataPaths::discover(cli.data_dir.clone())?;
        crate::gui::run(paths)
    }
    #[cfg(not(feature = "gui"))]
    {
        let _ = cli;
        bail!(
            "this build was compiled without the GUI feature; use `cargo run` without --no-default-features"
        )
    }
}

fn open_store(cli: &Cli) -> Result<Store> {
    let paths = DataPaths::discover(cli.data_dir.clone()).context("locating the library")?;
    let store = Store::open(paths).context("opening the library")?;
    if let Some(warning) = store.warning() {
        eprintln!("\x1b[33mwarning:\x1b[0m {warning}");
    }
    Ok(store)
}

// ------------------------------------------------------------------ commands

fn cmd_add(cli: &Cli, args: AddArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let mut text = args.text.join(" ");
    if text.trim().is_empty() && !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        text = buffer;
    }
    if text.trim().is_empty() {
        bail!("no task text provided");
    }
    let reference = now();
    let mut entry = parse_quick(&text, reference)?;
    if let Some(project) = args.project {
        entry.project_path = Some(project);
    }
    entry.tag_paths.extend(args.tags);
    if let Some(due) = args.due {
        entry.due = Some(parse_stamp(&due, reference)?);
    }
    if let Some(defer) = args.defer {
        entry.defer = Some(parse_stamp(&defer, reference)?);
    }
    if let Some(estimate) = args.estimate {
        entry.estimate_min = Some(parse_duration(&estimate)?);
    }
    if let Some(repeat) = args.repeat {
        entry.repeat = Some(parse_repeat(&repeat, reference)?);
    }
    if args.flag {
        entry.flagged = true;
    }
    if let Some(note) = args.note {
        if !entry.notes.is_empty() {
            entry.notes.push('\n');
        }
        entry.notes.push_str(&note);
    }
    let settings = store.db().settings.clone();
    if let Some(stamp) = entry.due {
        entry.due = Some(with_default_time(stamp, settings.default_due_time));
    }
    if let Some(stamp) = entry.defer {
        entry.defer = Some(with_default_time(stamp, settings.default_defer_time));
    }

    let mut task = entry.into_new_task();
    if let Some(priority) = &args.priority {
        task.priority = Some(parse_priority(priority)?);
    }
    if let Some(parent) = args.parent {
        task.parent = Some(store.db().resolve_task(&parent)?);
    }
    let outcome = store.execute(CoreCommand::CreateTask(task))?;
    let created = outcome
        .created
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("the task was not created"))?;
    let id = store.db().resolve_task(&created)?;

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::task_value(store.db(), &id))?
        );
    } else {
        let task = &store.db().tasks[&id];
        println!(
            "{} {}  {}",
            paint.green("Added"),
            paint.cyan(task.id.short()),
            paint.bold(&task.title)
        );
    }
    Ok(())
}

fn cmd_list(cli: &Cli, args: ListArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    store.refresh()?;
    let at = now();
    let db = store.db();

    if args.review {
        let ids = query::review_projects(db, at);
        if cli.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&render::project_values(db, &ids))?
            );
        } else if ids.is_empty() {
            println!("{}", paint.dim("No projects are due for review."));
        } else {
            println!("{}", paint.bold("Review"));
            for id in &ids {
                let project = &db.projects[id];
                println!(
                    "  {}  {}  {}",
                    paint.cyan(project.id.short()),
                    project.name,
                    paint.dim(&project.next_review.format("%b %-d").to_string())
                );
            }
        }
        return Ok(());
    }

    let groups: Vec<Group> = if args.trash {
        query::view(db, BuiltinPerspective::Trash, at)
    } else if args.forecast {
        query::view(db, BuiltinPerspective::Forecast, at)
    } else if args.inbox
        && !args.include_completed
        && !args.completed
        && args.tag.is_empty()
        && args.project.is_none()
        && args.search.is_none()
    {
        query::view(db, BuiltinPerspective::Inbox, at)
    } else if let Some(perspective) = &args.perspective {
        let id = resolve_perspective(db, perspective)?;
        query::custom(db, &db.perspectives[&id], at)
    } else {
        let mut conditions: Vec<Condition> = Vec::new();
        let mut show_completed = args.include_completed;
        if args.inbox {
            conditions.push(Condition::InInbox);
        }
        if args.flagged {
            conditions.push(Condition::Flagged { value: true });
        }
        if args.completed {
            conditions.push(Condition::Status {
                value: TaskStatus::Completed,
            });
            show_completed = true;
        }
        for tag in &args.tag {
            let id = resolve_tag(db, tag)?;
            conditions.push(Condition::Tag {
                id,
                include_children: true,
            });
        }
        if let Some(project) = &args.project {
            let id = db
                .find_project_by_path(project)
                .or_else(|| db.find_project_by_name(project))
                .ok_or_else(|| anyhow!("project not found: {project}"))?;
            conditions.push(Condition::Project { id });
        }
        if let Some(folder) = &args.folder {
            let id = db
                .find_folder_path(folder)
                .ok_or_else(|| anyhow!("folder not found: {folder}"))?;
            conditions.push(Condition::Folder {
                id,
                include_children: true,
            });
        }
        if args.available {
            conditions.push(Condition::Status {
                value: TaskStatus::Available,
            });
        }
        if let Some(due) = &args.due {
            conditions.push(Condition::Due {
                window: parse_due_window(due, at)?,
            });
        }
        if let Some(priority) = &args.priority {
            conditions.push(Condition::Priority {
                value: parse_priority(priority)?,
            });
        }
        let filter = FilterNode::all_conditions(conditions);
        let sort = parse_sort(&args.sort)?;
        let group_by = match &args.group {
            Some(value) => parse_group(value)?,
            None if args.inbox => GroupBy::None,
            None => GroupBy::Project,
        };
        query::run(db, &filter, sort, group_by, show_completed, at)
    };

    let mut groups = groups;
    if let Some(query_text) = &args.search {
        let wanted: std::collections::HashSet<TaskId> =
            search::task_ids(db, query_text, 1000).into_iter().collect();
        groups = groups
            .into_iter()
            .map(|mut group| {
                group.rows.retain(|row| wanted.contains(&row.id));
                group
            })
            .filter(|group| !group.rows.is_empty())
            .collect();
    }
    if let Some(limit) = args.limit {
        let mut remaining = limit;
        groups = groups
            .into_iter()
            .filter_map(|mut group| {
                if remaining == 0 {
                    return None;
                }
                group.rows.truncate(remaining);
                remaining -= group.rows.len();
                (!group.rows.is_empty()).then_some(group)
            })
            .collect();
    }

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::groups_value(db, &groups))?
        );
        return Ok(());
    }

    let mut printed = false;
    for group in &groups {
        if group.rows.is_empty() {
            continue;
        }
        printed = true;
        println!("{}", paint.bold(&group.title));
        for row in &group.rows {
            if let Some(task) = db.tasks.get(&row.id) {
                println!("{}", format_task_line(db, task, row, at, paint));
            }
        }
        println!();
    }
    if !printed {
        println!("{}", paint.dim("No tasks."));
    }
    Ok(())
}

fn format_task_line(
    db: &Db,
    task: &flowlist_core::model::Task,
    row: &flowlist_core::query::TaskRow,
    at: chrono::DateTime<Utc>,
    paint: &Paint,
) -> String {
    let marker = if task.completed {
        paint.green("✓")
    } else {
        match row.availability {
            Availability::Available => paint.green("○"),
            Availability::Deferred(_) | Availability::ProjectDeferred(_, _) => paint.yellow("◷"),
            _ => paint.dim("⊘"),
        }
    };
    let mut line = format!(
        "{}{} {}",
        "  ".repeat(row.depth as usize),
        marker,
        if task.completed {
            paint.dim(&task.title)
        } else {
            task.title.clone()
        }
    );
    if task.flagged {
        line.push_str(&format!(" {}", paint.yellow("!")));
    }
    if let Some(priority) = task.priority {
        line.push_str(&format!(
            " {}",
            paint.cyan(&format!("[{}]", priority.label()))
        ));
    }
    if let Some(due) = task.due {
        let text = format!("due {}", due.label(at));
        line.push_str(&format!("  {}", paint.yellow(&text)));
    }
    if let Some(defer) = task.defer {
        line.push_str(&format!(
            "  {}",
            paint.dim(&format!("defer {}", defer.label(at)))
        ));
    }
    if let Some(estimate) = task.estimate_min {
        line.push_str(&format!(
            "  {}",
            paint.dim(&format!(
                "{}",
                flowlist_core::time::format_duration(estimate)
            ))
        ));
    }
    let (checklist_done, checklist_total) = task.checklist_progress();
    if checklist_total > 0 {
        line.push_str(&format!(
            "  {}",
            paint.dim(&format!("☑ {checklist_done}/{checklist_total}"))
        ));
    }
    for tag in &task.tags {
        if let Some(tag) = db.tags.get(tag) {
            line.push_str(&format!(
                "  {}",
                paint.cyan(&format!("#{}", db.tag_path(&tag.id)))
            ));
        }
    }
    line.push_str(&format!("  {}", paint.dim(task.id.short())));
    line
}

fn cmd_done(cli: &Cli, args: DoneArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let ids = resolve_tasks(store.db(), &args.ids)?;
    let completed = !args.undo;
    let outcome = store.execute(CoreCommand::CompleteTasks { ids, completed })?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "label": outcome.label,
                "created": outcome.created,
                "affected": outcome.affected,
            }))?
        );
    } else {
        println!(
            "{} {}",
            paint.green(if completed { "Completed" } else { "Reopened" }),
            outcome.label
        );
        if let Some(message) = outcome.message {
            println!("  {}", paint.dim(&message));
        }
    }
    Ok(())
}

fn cmd_edit(cli: &Cli, args: EditArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let reference = now();
    let id = store.db().resolve_task(&args.id)?;
    if args.project.is_some() && args.parent.is_some() {
        bail!("choose either --project or --parent, not both");
    }
    let mut patch = TaskPatch::default();
    if let Some(title) = args.title {
        patch.title = Some(title);
    }
    if let Some(notes) = args.notes {
        patch.notes = Some(notes);
    }
    patch.due = optional_stamp(&args.due, reference)?;
    patch.defer = optional_stamp(&args.defer, reference)?;
    if let Some(estimate) = &args.estimate {
        patch.estimate_min = if estimate.eq_ignore_ascii_case("none") {
            Some(None)
        } else {
            Some(Some(parse_duration(estimate)?))
        };
    }
    if args.flag {
        patch.flagged = Some(true);
    }
    if args.unflag {
        patch.flagged = Some(false);
    }
    if let Some(priority) = &args.priority {
        patch.priority = if priority.eq_ignore_ascii_case("none") {
            Some(None)
        } else {
            Some(Some(parse_priority(priority)?))
        };
    }
    if let Some(repeat) = &args.repeat {
        patch.repeat = if repeat.eq_ignore_ascii_case("none") {
            Some(None)
        } else {
            Some(Some(parse_repeat(repeat, reference)?))
        };
    }
    if let Some(project) = &args.project {
        patch.project = if project.eq_ignore_ascii_case("inbox") {
            Some(None)
        } else {
            Some(Some(
                store
                    .db()
                    .find_project_by_path(project)
                    .or_else(|| store.db().find_project_by_name(project))
                    .ok_or_else(|| anyhow!("project not found: {project}"))?,
            ))
        };
    }
    if let Some(parent) = &args.parent {
        patch.parent = if parent.eq_ignore_ascii_case("none") {
            Some(None)
        } else {
            Some(Some(store.db().resolve_task(parent)?))
        };
    }
    if args.sequential {
        patch.sequential = Some(true);
    }
    store.execute(CoreCommand::UpdateTask {
        id: id.clone(),
        patch,
    })?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::task_value(store.db(), &id))?
        );
    } else {
        println!("{} {}", paint.green("Updated"), paint.cyan(id.short()));
    }
    Ok(())
}

fn cmd_remove(cli: &Cli, args: RemoveArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let ids = resolve_tasks(store.db(), &args.ids)?;
    let count = ids.len();
    store.execute(CoreCommand::DeleteTasks { ids })?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"deleted": count}))?
        );
    } else {
        println!("{} {} task(s) to Trash", paint.red("Moved"), count);
    }
    Ok(())
}

fn cmd_trash(cli: &Cli, args: TrashArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let mut acted = false;
    if !args.restore.is_empty() {
        let ids = resolve_tasks(store.db(), &args.restore)?;
        let outcome = store.execute(CoreCommand::RestoreTasks { ids })?;
        if !cli.json {
            println!(
                "{}",
                paint.green(&outcome.message.unwrap_or_else(|| "Restored".to_string()))
            );
        }
        acted = true;
    }
    if !args.delete.is_empty() {
        let ids = resolve_tasks(store.db(), &args.delete)?;
        let outcome = store.execute(CoreCommand::PurgeTasks { ids })?;
        if !cli.json {
            println!(
                "{}",
                paint.red(&outcome.message.unwrap_or_else(|| "Deleted".to_string()))
            );
        }
        acted = true;
    }
    if args.empty {
        let outcome = store.execute(CoreCommand::PurgeExpiredTrash { before: now() })?;
        if !cli.json {
            println!(
                "{}",
                paint.red(
                    &outcome
                        .message
                        .unwrap_or_else(|| "Emptied Trash".to_string())
                )
            );
        }
        acted = true;
    }
    if acted {
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&json!({"ok": true}))?);
        }
        return Ok(());
    }

    store.refresh()?;
    let db = store.db();
    let at = now();
    let groups = query::view(db, BuiltinPerspective::Trash, at);
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::groups_value(db, &groups))?
        );
        return Ok(());
    }
    let mut printed = false;
    for group in &groups {
        for row in &group.rows {
            if let Some(task) = db.tasks.get(&row.id) {
                printed = true;
                println!("{}", format_task_line(db, task, row, at, paint));
            }
        }
    }
    if !printed {
        println!("{}", paint.dim("Trash is empty."));
    } else {
        println!(
            "{}",
            paint.dim("restore with `flowlist trash --restore ID` · empty with `--empty`")
        );
    }
    Ok(())
}

fn cmd_mv(cli: &Cli, args: MvArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let ids = resolve_tasks(store.db(), &args.ids)?;
    let targets = [args.project.is_some(), args.parent.is_some(), args.inbox]
        .into_iter()
        .filter(|value| *value)
        .count();
    if targets != 1 {
        bail!("choose exactly one destination: --project, --parent or --inbox");
    }
    let position = match (&args.before, &args.after, args.start) {
        (Some(before), None, false) => Position::Before(store.db().resolve_task(before)?),
        (None, Some(after), false) => Position::After(store.db().resolve_task(after)?),
        (None, None, true) => Position::Start,
        (None, None, false) => Position::End,
        _ => bail!("choose at most one of --before, --after or --start"),
    };
    let placement = if let Some(project) = &args.project {
        let project = store
            .db()
            .find_project_by_path(project)
            .or_else(|| store.db().find_project_by_name(project))
            .ok_or_else(|| anyhow!("project not found: {project}"))?;
        TaskPlacement::Project { project, position }
    } else if let Some(parent) = &args.parent {
        TaskPlacement::Parent {
            parent: store.db().resolve_task(parent)?,
            position,
        }
    } else {
        TaskPlacement::Inbox(position)
    };
    store.execute(CoreCommand::MoveTasks { ids, placement })?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"moved": args.ids.len()}))?
        );
    } else {
        println!("{} {} task(s)", paint.green("Moved"), args.ids.len());
    }
    Ok(())
}

fn cmd_checklist(cli: &Cli, args: ChecklistArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        ChecklistCommand::List { task, hide_done } => {
            store.refresh()?;
            let db = store.db();
            let task_id = db.resolve_task(&task)?;
            let task = db.tasks.get(&task_id).expect("resolved task");
            if cli.json {
                let items: Vec<serde_json::Value> = task
                    .checklist
                    .iter()
                    .filter(|item| !hide_done || !item.completed)
                    .map(|item| {
                        json!({
                            "id": item.id.to_string(),
                            "short_id": item.id.short(),
                            "title": item.title,
                            "completed": item.completed,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "task": render::task_value(db, &task_id),
                        "items": items,
                    }))?
                );
            } else {
                let (done, total) = task.checklist_progress();
                println!(
                    "{}  {}  {}",
                    paint.cyan(task.id.short()),
                    paint.bold(&task.title),
                    paint.dim(&format!("{done}/{total}"))
                );
                for (index, item) in task.checklist.iter().enumerate() {
                    if hide_done && item.completed {
                        continue;
                    }
                    println!(
                        "  {} {}  {}  {}",
                        paint.dim(&index.to_string()),
                        if item.completed {
                            paint.green("✓")
                        } else {
                            paint.dim("○")
                        },
                        if item.completed {
                            paint.dim(&item.title)
                        } else {
                            item.title.clone()
                        },
                        paint.dim(item.id.short())
                    );
                }
                if task.checklist.is_empty() {
                    println!("  {}", paint.dim("No checklist items."));
                }
            }
        }
        ChecklistCommand::Add {
            task,
            title,
            before,
        } => {
            let task_id = store.db().resolve_task(&task)?;
            let title = title.join(" ");
            let outcome = store.execute(CoreCommand::AddChecklistItem {
                task: task_id.clone(),
                title: title.trim().to_string(),
                position: before,
            })?;
            let created = outcome.created.first().cloned().unwrap_or_default();
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::task_value(store.db(), &task_id))?
                );
            } else {
                let short = created
                    .get(created.len().saturating_sub(8)..)
                    .unwrap_or(&created);
                println!(
                    "{} {}",
                    paint.green("Added checklist item"),
                    paint.cyan(short)
                );
            }
        }
        ChecklistCommand::Set {
            task,
            item,
            title,
            done,
            undone,
        } => {
            if done && undone {
                bail!("choose at most one of --done or --undone");
            }
            let (task_id, item_id) = resolve_checklist(store.db(), &task, &item)?;
            let title = title
                .map(|value| value.trim().to_string())
                .or_else(|| {
                    store
                        .db()
                        .tasks
                        .get(&task_id)
                        .and_then(|task| task.checklist_item(&item_id))
                        .map(|item| item.title.clone())
                })
                .unwrap_or_default();
            store.execute(CoreCommand::UpdateChecklistItem {
                task: task_id.clone(),
                item: item_id.clone(),
                title,
                completed: if done {
                    Some(true)
                } else if undone {
                    Some(false)
                } else {
                    None
                },
            })?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::task_value(store.db(), &task_id))?
                );
            } else {
                println!(
                    "{} {}",
                    paint.green("Updated checklist item"),
                    paint.cyan(item_id.short())
                );
            }
        }
        ChecklistCommand::Move { task, item, to } => {
            let (task_id, item_id) = resolve_checklist(store.db(), &task, &item)?;
            store.execute(CoreCommand::MoveChecklistItem {
                task: task_id.clone(),
                item: item_id.clone(),
                to,
            })?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::task_value(store.db(), &task_id))?
                );
            } else {
                println!(
                    "{} {} → {}",
                    paint.green("Moved checklist item"),
                    paint.cyan(item_id.short()),
                    to
                );
            }
        }
        ChecklistCommand::Remove { task, item } => {
            let (task_id, item_id) = resolve_checklist(store.db(), &task, &item)?;
            store.execute(CoreCommand::DeleteChecklistItem {
                task: task_id.clone(),
                item: item_id.clone(),
            })?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::task_value(store.db(), &task_id))?
                );
            } else {
                println!(
                    "{} {}",
                    paint.red("Deleted checklist item"),
                    paint.cyan(item_id.short())
                );
            }
        }
    }
    Ok(())
}

fn resolve_checklist(db: &Db, task_spec: &str, item_spec: &str) -> Result<(TaskId, ChecklistId)> {
    let task_id = db.resolve_task(task_spec)?;
    let task = db
        .tasks
        .get(&task_id)
        .ok_or_else(|| anyhow!("task not found: {task_spec}"))?;
    let matches: Vec<&ChecklistId> = task
        .checklist
        .iter()
        .filter(|item| item.id.matches_spec(item_spec))
        .map(|item| &item.id)
        .collect();
    match matches.as_slice() {
        [] => bail!("checklist item not found: {item_spec}"),
        [item] => Ok((task_id, (*item).clone())),
        _ => bail!("checklist item id is ambiguous: {item_spec}"),
    }
}

fn cmd_undo(cli: &Cli, args: UndoArgs, redo: bool, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let mut labels = Vec::new();
    for _ in 0..args.steps.max(1) {
        let outcome = if redo { store.redo()? } else { store.undo()? };
        labels.push(outcome.label);
    }
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"steps": labels}))?
        );
    } else {
        for label in labels {
            println!(
                "{} {}",
                paint.green(if redo { "Redo:" } else { "Undo:" }),
                label
            );
        }
    }
    Ok(())
}

fn cmd_tag(cli: &Cli, args: TagArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        TagCommand::Add(args) => {
            let (parent, name) = split_parent_path(&args.path, args.parent.as_deref());
            let parent_id = match parent {
                Some(path) => Some(ensure_tag_chain(&mut store, &path)?),
                None => None,
            };
            let status = args
                .status
                .as_deref()
                .map(parse_tag_status)
                .transpose()?
                .unwrap_or(TagStatus::Available);
            let location = match (args.lat, args.lon) {
                (Some(lat), Some(lon)) => Some(flowlist_core::model::Geo { lat, lon }),
                (None, None) => None,
                _ => bail!("--lat and --lon must be given together"),
            };
            let outcome = store.execute(CoreCommand::CreateTag {
                name,
                parent: parent_id,
                status,
                location,
                position: Position::End,
            })?;
            let created = outcome.created.first().cloned().unwrap_or_default();
            let id = store.db().resolve_tag(&created)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::tag_value(store.db(), &id))?
                );
            } else {
                println!(
                    "{} {}  {}",
                    paint.green("Added tag"),
                    paint.cyan(id.short()),
                    store.db().tag_path(&id)
                );
            }
        }
        TagCommand::Remove { id } => {
            let id = resolve_tag(store.db(), &id)?;
            store.execute(CoreCommand::DeleteTag { id: id.clone() })?;
            if !cli.json {
                println!("{} {}", paint.red("Deleted tag"), paint.cyan(id.short()));
            }
        }
        TagCommand::Set(args) => {
            let id = resolve_tag(store.db(), &args.id)?;
            let mut patch = TagPatch::default();
            if let Some(name) = args.name {
                patch.name = Some(name);
            }
            if let Some(parent) = &args.parent {
                patch.parent = if parent.eq_ignore_ascii_case("none") {
                    Some(None)
                } else {
                    Some(Some(resolve_tag(store.db(), parent)?))
                };
            }
            if let Some(status) = &args.status {
                patch.status = Some(parse_tag_status(status)?);
            }
            if args.clear_location {
                patch.location = Some(None);
            } else if let (Some(lat), Some(lon)) = (args.lat, args.lon) {
                patch.location = Some(Some(flowlist_core::model::Geo { lat, lon }));
            } else if args.lat.is_some() || args.lon.is_some() {
                bail!("--lat and --lon must be given together");
            }
            store.execute(CoreCommand::UpdateTag {
                id: id.clone(),
                patch,
            })?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::tag_value(store.db(), &id))?
                );
            } else {
                println!("{} {}", paint.green("Updated tag"), paint.cyan(id.short()));
            }
        }
    }
    Ok(())
}

fn cmd_tags(cli: &Cli, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    store.refresh()?;
    let db = store.db();
    let rows = db.flatten_tags();
    if cli.json {
        let ids: Vec<TagId> = rows.iter().map(|row| row.tag.clone()).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&render::tag_values(db, &ids))?
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("{}", paint.dim("No tags."));
        return Ok(());
    }
    for row in rows {
        let tag = &db.tags[&row.tag];
        println!(
            "{}{}  {}  {}",
            "  ".repeat(row.depth as usize),
            db.tag_path(&row.tag),
            paint.dim(tag.status.label()),
            paint.dim(&format!("{} task(s)", db.tag_usage_count(&row.tag, false)))
        );
    }
    Ok(())
}

fn cmd_project(cli: &Cli, args: ProjectArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        ProjectCommand::Add(args) => {
            let reference = now();
            let (folder_path, name) = match args.name.rsplit_once('/') {
                Some((folder, name)) => (Some(folder.to_string()), name.trim().to_string()),
                None => (args.folder.clone(), args.name.trim().to_string()),
            };
            let folder = match folder_path {
                Some(path) => Some(ensure_folder(&mut store, &path)?),
                None => None,
            };
            let mut new_project = NewProject::new(name);
            new_project.folder = folder;
            new_project.notes = args.note.unwrap_or_default();
            new_project.due = optional_stamp_value(&args.due, reference)?;
            new_project.defer = optional_stamp_value(&args.defer, reference)?;
            new_project.review_interval_days = args.review_days;
            new_project.kind = args
                .kind
                .as_deref()
                .map(parse_project_kind)
                .transpose()?
                .unwrap_or(ProjectKind::Parallel);
            new_project.sequential = args.sequential || new_project.kind == ProjectKind::Sequential;
            let outcome = store.execute(CoreCommand::CreateProject(new_project))?;
            let created = outcome.created.first().cloned().unwrap_or_default();
            let id = store.db().resolve_project(&created)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::project_value(store.db(), &id))?
                );
            } else {
                println!(
                    "{} {}  {}",
                    paint.green("Added project"),
                    paint.cyan(id.short()),
                    store.db().project_path(&id)
                );
            }
        }
        ProjectCommand::List { closed } => {
            store.refresh()?;
            let db = store.db();
            let ids: Vec<ProjectId> = db
                .flatten_containers()
                .into_iter()
                .filter_map(|row| match row.container {
                    ContainerId::Project(id) => db
                        .projects
                        .get(&id)
                        .filter(|project| closed || project.status.is_open())
                        .map(|_| id),
                    ContainerId::Folder(_) => None,
                })
                .collect();
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::project_values(db, &ids))?
                );
            } else if ids.is_empty() {
                println!("{}", paint.dim("No projects."));
            } else {
                for id in &ids {
                    let project = &db.projects[id];
                    let (done, total) = db.project_progress(id);
                    println!(
                        "{}  {}  {}",
                        paint.cyan(project.id.short()),
                        db.project_path(id),
                        paint.dim(&format!("{}  {done}/{total}", project.status.label()))
                    );
                }
            }
        }
        ProjectCommand::Status { id, status } => {
            let id = resolve_project(store.db(), &id)?;
            let status = parse_project_status(&status)?;
            store.execute(CoreCommand::SetProjectStatus {
                id: id.clone(),
                status,
            })?;
            if !cli.json {
                println!(
                    "{} {} → {}",
                    paint.green("Project"),
                    paint.cyan(id.short()),
                    status.label()
                );
            }
        }
        ProjectCommand::Review { id, on } => {
            let id = resolve_project(store.db(), &id)?;
            let reviewed_on = match on {
                Some(value) => flowlist_core::parse::parse_date(&value, now())?,
                None => flowlist_core::time::local_date(now()),
            };
            store.execute(CoreCommand::CompleteReview {
                id: id.clone(),
                reviewed_on,
            })?;
            if !cli.json {
                let project = &store.db().projects[&id];
                println!(
                    "{} {}  next review {}",
                    paint.green("Reviewed"),
                    paint.cyan(id.short()),
                    project.next_review.format("%b %-d")
                );
            }
        }
        ProjectCommand::Edit(args) => {
            let id = resolve_project(store.db(), &args.id)?;
            let reference = now();
            let mut patch = ProjectPatch::default();
            if let Some(name) = args.name {
                patch.name = Some(name);
            }
            if let Some(note) = args.note {
                patch.notes = Some(note);
            }
            if let Some(folder) = &args.folder {
                patch.folder = if folder.eq_ignore_ascii_case("none") {
                    Some(None)
                } else {
                    Some(Some(ensure_folder(&mut store, folder)?))
                };
            }
            patch.due = optional_stamp(&args.due, reference)?;
            patch.defer = optional_stamp(&args.defer, reference)?;
            if let Some(days) = args.review_days {
                patch.review_interval_days = Some(days);
            }
            if let Some(next) = &args.next_review {
                patch.next_review = Some(flowlist_core::parse::parse_date(next, reference)?);
            }
            if args.sequential {
                patch.sequential = Some(true);
            }
            if let Some(kind) = &args.kind {
                patch.kind = Some(parse_project_kind(kind)?);
            }
            store.execute(CoreCommand::UpdateProject {
                id: id.clone(),
                patch,
            })?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::project_value(store.db(), &id))?
                );
            } else {
                println!(
                    "{} {}",
                    paint.green("Updated project"),
                    paint.cyan(id.short())
                );
            }
        }
    }
    Ok(())
}

fn cmd_folder(cli: &Cli, args: FolderArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        FolderCommand::Add { path } => {
            let id = ensure_folder(&mut store, &path)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "id": id.to_string(),
                        "short_id": id.short(),
                        "path": store.db().folder_path(&id),
                    }))?
                );
            } else {
                println!(
                    "{} {}  {}",
                    paint.green("Added folder"),
                    paint.cyan(id.short()),
                    store.db().folder_path(&id)
                );
            }
        }
        FolderCommand::List => {
            store.refresh()?;
            let db = store.db();
            let rows = db.flatten_containers();
            if cli.json {
                let folders: Vec<serde_json::Value> = rows
                    .iter()
                    .filter_map(|row| match &row.container {
                        ContainerId::Folder(id) => Some(json!({
                            "id": id.to_string(),
                            "short_id": id.short(),
                            "path": db.folder_path(id),
                            "depth": row.depth,
                            "projects": db.project_count(id),
                        })),
                        ContainerId::Project(_) => None,
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json!(folders))?);
            } else if rows.is_empty() {
                println!("{}", paint.dim("No folders."));
            } else {
                for row in rows {
                    match row.container {
                        ContainerId::Folder(id) => println!(
                            "{}{}  {}",
                            "  ".repeat(row.depth as usize),
                            db.folder_path(&id),
                            paint.dim(&format!("{} project(s)", db.project_count(&id)))
                        ),
                        ContainerId::Project(id) => println!(
                            "{}• {}",
                            "  ".repeat(row.depth as usize),
                            db.project_path(&id)
                        ),
                    }
                }
            }
        }
        FolderCommand::Rename { id, name } => {
            let id = store.db().resolve_folder(&id)?;
            store.execute(CoreCommand::RenameFolder {
                id: id.clone(),
                name,
            })?;
            if !cli.json {
                println!(
                    "{} {}",
                    paint.green("Renamed folder"),
                    paint.cyan(id.short())
                );
            }
        }
        FolderCommand::Remove { id } => {
            let id = store.db().resolve_folder(&id)?;
            store.execute(CoreCommand::DeleteFolder { id: id.clone() })?;
            if !cli.json {
                println!("{} {}", paint.red("Deleted folder"), paint.cyan(id.short()));
            }
        }
    }
    Ok(())
}

fn cmd_perspective(cli: &Cli, args: PerspectiveArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        PerspectiveCommand::Add(args) => {
            let mut perspective = Perspective::new(args.name);
            if let Some(filter) = args.filter {
                perspective.filter =
                    serde_json::from_str(&filter).context("--filter must be a JSON filter tree")?;
            }
            if let Some(group_by) = args.group_by {
                perspective.group_by = parse_group(&group_by)?;
            }
            if let Some(sort) = args.sort {
                perspective.sort = parse_sort(&sort)?;
            }
            perspective.show_completed = args.show_completed;
            let outcome = store.execute(CoreCommand::SavePerspective {
                perspective: perspective.clone(),
            })?;
            // Saving reports the id under `affected`; `created` is only used by
            // constructors whose id the caller does not know yet.
            let saved = outcome
                .affected
                .first()
                .cloned()
                .unwrap_or_else(|| perspective.id.to_string());
            let id = store.db().resolve_perspective(&saved)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "id": id.to_string(),
                        "short_id": id.short(),
                        "name": perspective.name,
                    }))?
                );
            } else {
                println!(
                    "{} {}  {}",
                    paint.green("Saved perspective"),
                    paint.cyan(id.short()),
                    perspective.name
                );
            }
        }
        PerspectiveCommand::List => {
            store.refresh()?;
            let db = store.db();
            let mut perspectives: Vec<&Perspective> = db.perspectives.values().collect();
            perspectives.sort_by_key(|perspective| perspective.position);
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!(
                        perspectives
                            .iter()
                            .map(|perspective| json!({
                                "id": perspective.id.to_string(),
                                "short_id": perspective.id.short(),
                                "name": perspective.name,
                                "group_by": format!("{:?}", perspective.group_by),
                                "sort": format!("{:?}", perspective.sort),
                                "show_completed": perspective.show_completed,
                            }))
                            .collect::<Vec<_>>()
                    ))?
                );
            } else if perspectives.is_empty() {
                println!("{}", paint.dim("No custom perspectives."));
            } else {
                for perspective in perspectives {
                    println!(
                        "{}  {}",
                        paint.cyan(perspective.id.short()),
                        perspective.name
                    );
                }
            }
        }
        PerspectiveCommand::Remove { id } => {
            let id = resolve_perspective(store.db(), &id)?;
            store.execute(CoreCommand::DeletePerspective { id: id.clone() })?;
            if !cli.json {
                println!(
                    "{} {}",
                    paint.red("Deleted perspective"),
                    paint.cyan(id.short())
                );
            }
        }
    }
    Ok(())
}

fn cmd_review(cli: &Cli, args: ReviewArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    match args.command {
        Some(ReviewCommand::Done { id, on }) => {
            let id = resolve_project(store.db(), &id)?;
            let reviewed_on = match on {
                Some(value) => flowlist_core::parse::parse_date(&value, now())?,
                None => flowlist_core::time::local_date(now()),
            };
            store.execute(CoreCommand::CompleteReview {
                id: id.clone(),
                reviewed_on,
            })?;
            if !cli.json {
                let project = &store.db().projects[&id];
                println!(
                    "{} {}  next {}",
                    paint.green("Reviewed"),
                    paint.cyan(id.short()),
                    project.next_review.format("%b %-d")
                );
            }
        }
        None => {
            store.refresh()?;
            let db = store.db();
            let ids = query::review_projects(db, now());
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render::project_values(db, &ids))?
                );
            } else if ids.is_empty() {
                println!(
                    "{}",
                    paint.dim("Nothing to review. Inbox zero for projects.")
                );
            } else {
                for id in ids {
                    let project = &db.projects[&id];
                    println!(
                        "{}  {}  {}",
                        paint.cyan(project.id.short()),
                        project.name,
                        paint.dim(&format!("review {}", project.next_review.format("%b %-d")))
                    );
                }
            }
        }
    }
    Ok(())
}

fn cmd_search(cli: &Cli, args: SearchArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    store.refresh()?;
    let db = store.db();
    let hits = search::search(db, &args.query, args.limit);
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::search_value(db, &hits))?
        );
        return Ok(());
    }
    if hits.is_empty() {
        println!("{}", paint.dim("No matches."));
        return Ok(());
    }
    for hit in hits {
        println!(
            "{:<11} {}  {:<44} {}",
            paint.dim(hit.kind.label()),
            paint.cyan(hit.short_id()),
            hit.title,
            paint.dim(&hit.subtitle)
        );
    }
    Ok(())
}

fn cmd_stats(cli: &Cli, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    store.refresh()?;
    let db = store.db();
    let counts = stats::compute(db, now());
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&render::stats_value(&counts))?
        );
        return Ok(());
    }
    println!("{}", paint.bold("Flowlist"));
    println!("  Open tasks        {}", counts.open_tasks);
    println!("  Available         {}", counts.available);
    println!("  Flagged           {}", counts.flagged);
    println!("  Due today         {}", counts.due_today);
    println!("  Overdue           {}", counts.overdue);
    println!("  Inbox             {}", counts.inbox);
    println!("  Completed today   {}", counts.completed_today);
    println!(
        "  Projects          {} active, {} on hold, {} to review",
        counts.active_projects, counts.on_hold_projects, counts.projects_due_review
    );
    println!("  Tags / folders    {} / {}", counts.tags, counts.folders);
    Ok(())
}

fn cmd_export(cli: &Cli, args: ExportArgs) -> Result<()> {
    let mut store = open_store(cli)?;
    if args.backup {
        store.refresh()?;
        let dir = store.paths().ensure_backups()?;
        let stamp = now().format("%Y%m%d-%H%M%S");
        let destination = dir.join(format!("flowlist-backup-{stamp}.json"));
        persist::atomic::atomic_write(&destination, &serde_json::to_vec_pretty(store.data())?)?;
        println!("Backup written to {}", destination.display());
        if args.output.is_none() {
            return Ok(());
        }
    }
    let contents = match args.format.to_lowercase().as_str() {
        "json" => export::to_json(store.db())?,
        "taskpaper" | "tp" => taskpaper::export(store.db()),
        other => bail!("unknown export format: {other} (use json or taskpaper)"),
    };
    match args.output {
        Some(path) => {
            std::fs::write(&path, contents)
                .with_context(|| format!("writing {}", path.display()))?;
            println!("Wrote {}", path.display());
        }
        None => print!("{contents}"),
    }
    Ok(())
}

fn cmd_import(cli: &Cli, args: ImportArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    let text = if args.file == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        std::fs::read_to_string(&args.file).with_context(|| format!("reading {}", args.file))?
    };
    let normalised = args.format.to_lowercase();
    let format = match normalised.as_str() {
        "auto" => {
            if text.trim_start().starts_with('{') {
                "json"
            } else {
                "taskpaper"
            }
        }
        other => other,
    };
    let db = match format {
        "json" => export::from_json(&text)?,
        "taskpaper" | "tp" => taskpaper::import(&text)?,
        other => bail!("unknown import format: {other}"),
    };
    let mode = if args.merge {
        flowlist_core::command::ImportMode::Merge
    } else {
        flowlist_core::command::ImportMode::Replace
    };
    let outcome = store.execute(CoreCommand::Import { db, mode })?;
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"label": outcome.label}))?
        );
    } else {
        println!(
            "{} {}",
            paint.green("Imported"),
            outcome.message.unwrap_or_else(|| outcome.label)
        );
    }
    Ok(())
}

fn cmd_doctor(cli: &Cli, args: DoctorArgs, paint: &Paint) -> Result<()> {
    let mut store = open_store(cli)?;
    store.refresh()?;
    let db = store.db().clone();
    let counts = stats::compute(&db, now());
    let mut fixes = Vec::new();
    if args.fix {
        let mut repaired = db.clone();
        fixes = repaired.repair();
        if !fixes.is_empty() {
            repaired.validate()?;
            let mut data = store.data().clone();
            data.db = repaired;
            data.revision = data.revision.saturating_add(1);
            data.updated_at = now();
            persist::save_data(store.paths(), &data)?;
            let mut history = persist::HistoryFile::new(&data.db);
            history.entries.clear();
            history.entries.push(persist::HistoryEntry {
                label: "Repaired library".to_string(),
                at: now(),
                state: data.db.clone(),
            });
            persist::save_history(store.paths(), &history)?;
        }
    }
    let paths = store.paths();
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "root": paths.root.display().to_string(),
                "data": paths.data().display().to_string(),
                "schema_version": persist::SCHEMA_VERSION,
                "revision": store.revision(),
                "stats": render::stats_value(&counts),
                "validation": db.validate().err().map(|error| error.to_string()),
                "history_entries": store.history().entries.len(),
                "fixes": fixes,
            }))?
        );
        return Ok(());
    }
    println!("{}", paint.bold("Flowlist doctor"));
    println!("  Data directory  {}", paths.root.display());
    println!("  Data file       {}", paths.data().display());
    println!(
        "  Schema          v{} (revision {})",
        persist::SCHEMA_VERSION,
        store.revision()
    );
    println!(
        "  Tasks           {} total, {} open, {} completed",
        counts.total_tasks, counts.open_tasks, counts.completed_tasks
    );
    println!(
        "  Projects        {} active, {} on hold",
        counts.active_projects, counts.on_hold_projects
    );
    println!(
        "  History         {} state(s), undo {}",
        store.history().entries.len(),
        if store.can_undo() {
            "available"
        } else {
            "empty"
        }
    );
    match db.validate() {
        Ok(()) => println!("  Validation      {}", paint.green("ok")),
        Err(error) => println!("  Validation      {} {error}", paint.red("failed:")),
    }
    if args.fix {
        if fixes.is_empty() {
            println!("  Repairs         {}", paint.dim("nothing to fix"));
        } else {
            println!("  Repairs         {}", paint.green(&fixes.join(", ")));
        }
    }
    Ok(())
}

fn cmd_where(cli: &Cli, paint: &Paint) -> Result<()> {
    let paths = DataPaths::discover(cli.data_dir.clone())?;
    let entries = [
        ("data directory", paths.root.clone()),
        ("data", paths.data()),
        ("backup", paths.data_backup()),
        ("history", paths.history()),
        ("session", paths.session()),
        ("lock", paths.lock()),
    ];
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "root": paths.root.display().to_string(),
                "data": paths.data().display().to_string(),
                "exists": paths.data().exists(),
            }))?
        );
        return Ok(());
    }
    for (label, path) in entries {
        let exists = if path.exists() {
            paint.green("exists")
        } else {
            paint.dim("missing")
        };
        println!("{:<16} {}  {}", label, path.display(), exists);
    }
    Ok(())
}

// -------------------------------------------------------------------- helpers

fn resolve_tasks(db: &Db, specs: &[String]) -> Result<Vec<TaskId>> {
    if specs.is_empty() {
        bail!("no task ids given");
    }
    specs
        .iter()
        .map(|spec| db.resolve_task(spec).map_err(|error| anyhow!("{error}")))
        .collect()
}

fn resolve_project(db: &Db, spec: &str) -> Result<ProjectId> {
    if let Ok(id) = db.resolve_project(spec) {
        return Ok(id);
    }
    db.find_project_by_path(spec)
        .or_else(|| db.find_project_by_name(spec))
        .ok_or_else(|| anyhow!("project not found: {spec}"))
}

fn resolve_tag(db: &Db, spec: &str) -> Result<TagId> {
    if let Some(id) = db.find_tag_path(spec) {
        return Ok(id);
    }
    db.resolve_tag(spec).map_err(|error| anyhow!("{error}"))
}

fn resolve_perspective(db: &Db, spec: &str) -> Result<PerspectiveId> {
    if let Some(perspective) = db
        .perspectives
        .values()
        .find(|perspective| perspective.name.eq_ignore_ascii_case(spec))
    {
        return Ok(perspective.id.clone());
    }
    db.resolve_perspective(spec)
        .map_err(|error| anyhow!("{error}"))
}

fn parse_sort(value: &str) -> Result<SortBy> {
    match value.to_lowercase().as_str() {
        "manual" | "none" => Ok(SortBy::Manual),
        "due" => Ok(SortBy::Due),
        "defer" => Ok(SortBy::Defer),
        "name" | "title" => Ok(SortBy::Name),
        "flagged" | "flag" => Ok(SortBy::Flagged),
        "estimate" => Ok(SortBy::Estimate),
        "created" => Ok(SortBy::Created),
        "completed" | "completed-at" => Ok(SortBy::CompletedAt),
        other => bail!("unknown sort: {other}"),
    }
}

fn parse_group(value: &str) -> Result<GroupBy> {
    match value.to_lowercase().as_str() {
        "none" | "manual" => Ok(GroupBy::None),
        "project" => Ok(GroupBy::Project),
        "folder" => Ok(GroupBy::Folder),
        "tag" | "context" => Ok(GroupBy::Tag),
        "due" | "due-date" => Ok(GroupBy::DueDate),
        "defer" | "defer-date" => Ok(GroupBy::DeferDate),
        "flagged" | "flag" => Ok(GroupBy::Flagged),
        "estimate" => Ok(GroupBy::Estimate),
        other => bail!("unknown group: {other}"),
    }
}

fn parse_due_window(value: &str, at: chrono::DateTime<Utc>) -> Result<DueWindow> {
    let today = flowlist_core::time::local_date(at);
    if let Some(window) = query::perspective::window_from_keyword(value, today) {
        return Ok(window);
    }
    Ok(DueWindow::On(parse_stamp(value, at)?.local_date()))
}

fn parse_priority(value: &str) -> Result<Priority> {
    Priority::from_slug(&value.trim().to_lowercase())
        .or_else(|| {
            Priority::ALL
                .into_iter()
                .find(|priority| priority.label().eq_ignore_ascii_case(value.trim()))
        })
        .ok_or_else(|| anyhow!("priority must be low, medium or high"))
}

fn parse_project_kind(value: &str) -> Result<ProjectKind> {
    ProjectKind::from_slug(value.trim())
        .ok_or_else(|| anyhow!("project kind must be parallel, sequential or single_actions"))
}

fn parse_project_status(value: &str) -> Result<ProjectStatus> {
    match value.to_lowercase().as_str() {
        "active" | "open" => Ok(ProjectStatus::Active),
        "on-hold" | "on_hold" | "hold" => Ok(ProjectStatus::OnHold),
        "done" | "complete" | "completed" => Ok(ProjectStatus::Done),
        "dropped" | "drop" => Ok(ProjectStatus::Dropped),
        other => bail!("unknown project status: {other}"),
    }
}

fn parse_tag_status(value: &str) -> Result<TagStatus> {
    match value.to_lowercase().as_str() {
        "available" | "active" => Ok(TagStatus::Available),
        "blocked" => Ok(TagStatus::Blocked),
        "on-hold" | "on_hold" | "hold" => Ok(TagStatus::OnHold),
        other => bail!("unknown tag status: {other}"),
    }
}

fn optional_stamp(
    value: &Option<String>,
    reference: chrono::DateTime<Utc>,
) -> Result<Option<Option<Stamp>>> {
    match value {
        None => Ok(None),
        Some(value) if value.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(value) => Ok(Some(Some(parse_stamp(value, reference)?))),
    }
}

fn optional_stamp_value(
    value: &Option<String>,
    reference: chrono::DateTime<Utc>,
) -> Result<Option<Stamp>> {
    match value {
        None => Ok(None),
        Some(value) => Ok(Some(parse_stamp(value, reference)?)),
    }
}

fn split_parent_path(path: &str, explicit_parent: Option<&str>) -> (Option<String>, String) {
    if let Some(parent) = explicit_parent {
        return (Some(parent.to_string()), path.trim().to_string());
    }
    match path.rsplit_once('/') {
        Some((parent, name)) => (Some(parent.to_string()), name.trim().to_string()),
        None => (None, path.trim().to_string()),
    }
}

fn ensure_folder(store: &mut Store, path: &str) -> Result<FolderId> {
    let mut parent: Option<FolderId> = None;
    for part in path
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let siblings = store.db().container_children(parent.clone()).to_vec();
        let existing = siblings.into_iter().find_map(|container| match container {
            ContainerId::Folder(id) => store
                .db()
                .folders
                .get(&id)
                .filter(|folder| folder.name.eq_ignore_ascii_case(part))
                .map(|_| id),
            ContainerId::Project(_) => None,
        });
        parent = Some(match existing {
            Some(id) => id,
            None => {
                let outcome = store.execute(CoreCommand::CreateFolder {
                    name: part.to_string(),
                    parent: parent.clone(),
                    position: ContainerPosition::End,
                })?;
                let created = outcome
                    .created
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow!("folder was not created"))?;
                store.db().resolve_folder(&created)?
            }
        });
    }
    parent.ok_or_else(|| anyhow!("empty folder path"))
}

fn ensure_tag_chain(store: &mut Store, path: &str) -> Result<TagId> {
    let mut parent: Option<TagId> = None;
    for part in path
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let siblings: Vec<TagId> = match &parent {
            Some(id) => store.db().tag_children(id).to_vec(),
            None => store.db().tag_roots().to_vec(),
        };
        let existing = siblings.into_iter().find(|id| {
            store
                .db()
                .tags
                .get(id)
                .is_some_and(|tag| tag.name.eq_ignore_ascii_case(part))
        });
        parent = Some(match existing {
            Some(id) => id,
            None => {
                let outcome = store.execute(CoreCommand::CreateTag {
                    name: part.to_string(),
                    parent: parent.clone(),
                    status: TagStatus::Available,
                    location: None,
                    position: Position::End,
                })?;
                let created = outcome.created.first().cloned().unwrap_or_default();
                store.db().resolve_tag(&created)?
            }
        });
    }
    parent.ok_or_else(|| anyhow!("empty tag path"))
}

fn _settings_patch_example() -> SettingsPatch {
    SettingsPatch::default()
}

fn _repeat_example() -> Option<RepeatRule> {
    None
}

/// ANSI styling that respects `--json` and non-terminal output.
pub struct Paint {
    enabled: bool,
}

impl Paint {
    pub fn new(json: bool) -> Self {
        Self {
            enabled: !json && std::io::stdout().is_terminal(),
        }
    }

    fn wrap(&self, text: &str, code: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(&self, text: &str) -> String {
        self.wrap(text, "1")
    }

    pub fn dim(&self, text: &str) -> String {
        self.wrap(text, "2")
    }

    pub fn red(&self, text: &str) -> String {
        self.wrap(text, "31")
    }

    pub fn green(&self, text: &str) -> String {
        self.wrap(text, "32")
    }

    pub fn yellow(&self, text: &str) -> String {
        self.wrap(text, "33")
    }

    pub fn cyan(&self, text: &str) -> String {
        self.wrap(text, "36")
    }
}
