# Flowlist architecture

Flowlist is split into a headless domain crate and one binary that exposes the
domain through three front ends.

```
flowlist-core                       flowlist
┌─────────────────────────┐         ┌─────────────────────────────┐
│ model    ids/time       │         │ GPUI app (gui feature)      │
│ db       command        │◄────────│ CLI (clap)                  │
│ store    persistence    │         │ MCP server (JSON-RPC stdio) │
│ query    search         │         │ shared JSON renderer        │
│ import   export         │         └─────────────────────────────┘
└─────────────────────────┘
```

`flowlist-core` has no GPUI dependency. The `gui` Cargo feature is on by
default; `--no-default-features` builds a headless CLI and MCP server.

## Domain model

All entities use ULIDs (`crates/flowlist-core/src/ids.rs`). ULIDs sort by
creation time and are exposed in a compact eight-character suffix by the CLI,
GUI and MCP output. Every id lookup accepts a full id, an unambiguous prefix or
the short suffix.

Entities:

| Entity | Notes |
| --- | --- |
| `Task` | title, notes, project, parent, tags, dates, estimate, flag, repeat rule, explicit child order |
| `ChecklistItem` | per-task step: id, title, completed; never a task |
| `Project` | name, folder, notes, tags, dates, status, review interval, task order |
| `Folder` | nested container for projects |
| `Tag` | nested context with availability status and optional location |
| `Perspective` | saved filter tree, grouping and sort |
| `Settings` | review/completed windows, location, radius |

The database (`db.rs`) stores maps plus explicit ordering arrays
(`Project::task_order`, `Task::child_order`, `Folder::child_order`, `Db::root`).
Reparenting is therefore an array splice rather than an implicit sort key.

Dates use `Stamp { at: DateTime<Utc>, has_time: bool }`. Everything is stored in
UTC and bucketed in local time, so `due:today` means the user's local day.
Date-only stamps render without a time.

`Task::sequential` turns a task's children into an ordered list: only the first
incomplete child is available.

## Command engine

Every mutation is a serializable `Command` variant in
`crates/flowlist-core/src/command.rs`:

```rust
Command::CreateTask(NewTask)
Command::UpdateTask { id, patch }
Command::CompleteTasks { ids, completed }
Command::AddChecklistItem { task, title, position }
Command::MoveChecklistItem { task, item, to }
Command::SetProjectStatus { id, status }
Command::Import { db, mode }
// ...
```

`Store::execute` wraps a command in a full-state snapshot:

1. acquire the cross-process lock;
2. reload the latest data if another process wrote it;
3. apply the command;
4. validate the resulting database;
5. append the previous state to `history.json`;
6. write `data.json` atomically.

Undo and redo restore snapshots; `HISTORY_LIMIT` keeps 80 states (40 undo plus
redo capacity). Because history is persisted, undo works across GUI and CLI
sessions and survives a restart.

Repeat rules are applied inside `CompleteTasks`. The next occurrence is computed
from due/defer dates and the repeat mode/schedule, then a new task is inserted
after the completed one. Its checklist is copied with every item unchecked, its
repeat counter advanced, and the last number in its title incremented. Every
occurrence carries the first task's id in `repeat_series`, so `DeleteTaskSeries`
can find the whole chain and trash it in one snapshot; a task without a
recorded series is treated as its own series.

Checklist commands are deliberately separate from task commands: they edit
`Task::checklist` in place, participate in undo, and never create rows in
perspectives.

## Persistence and concurrency

`persist/mod.rs`, `persist/atomic.rs` and `persist/lock.rs` implement:

- `data.json` as `{schema_version, revision, updated_at, db}`;
- atomic write → fsync → rename, with the previous file kept as `data.json.bak`;
- `data.lock` via `fs2` for cross-process serialization;
- schema migration on open (`persist/migrate.rs`);
- revision polling so long-running processes reload after external writes;
- `doctor` validation and repair for dangling ids/order entries.

The GUI polls `Store::reload_if_changed` once per second. The CLI refreshes at the
start of a command. The MCP server does the same, so all three stay coherent
without a daemon.

## Query engine

`query/` evaluates built-in perspectives and saved filters:

- `query::view(db, BuiltinPerspective, at)` returns grouped rows.
- `query::custom(db, perspective, at)` applies a `FilterNode` tree.
- `query/availability.rs` combines task, project and tag status plus defer
  dates to classify rows as available, deferred, blocked or completed.
- `query/forecast.rs` buckets due tasks into overdue, today, tomorrow, later.
- `query/sort.rs` supports manual order, due, defer, name, flag, estimate,
  created and completed.
- `query/regroup` powers ad-hoc grouping in the GUI (`--group` in the CLI).

Filters are hand-written serde types so the JSON shape stays stable across
versions and can be authored from the CLI or MCP.

## Import and export

`taskpaper.rs` implements a TaskPaper-compatible outline with fields for dates,
estimates, flags, tags and repeat rules. `export.rs` reads and writes the full
JSON library. Both are exposed through the CLI and MCP server.

Merge imports reconcile tags by path rather than id, so importing a library
that was exported elsewhere does not duplicate `Work/Website` under a new id.
Folder and project paths are matched the same way.

Search (`search.rs`) scores title, notes, tag paths and checklist item titles,
then returns typed hits for tasks, projects, folders, tags and perspectives.

## GUI

The GUI lives in `crates/flowlist/src/gui/`:

| File | Responsibility |
| --- | --- |
| `mod.rs` | app state, layout, keyboard routing, capture, commands, session |
| `sidebar.rs` | perspectives and project/folder/tag trees |
| `content.rs` | task list, project dashboards, forecast, review, nearby |
| `inspector.rs` | task/project editing and checklist management |
| `modals.rs` | new project/folder/tag/perspective, rename, import, settings, help |
| `drag.rs` | task drag payload and drop targets |
| `text.rs` | small cursor/selection-aware text buffer |
| `theme.rs` | light/dark palettes |
| `widgets.rs` | native GPUI controls (buttons, fields, chips, checkboxes, progress) |

There is one root focus handle. `FocusArea` decides whether keystrokes go to the
content list, capture field, search field, inspector field, checklist input,
tag input or a modal. Text editing is handled directly from `on_key_down`, which
keeps the same behaviour for hardware keyboards without relying on OS IME.

The window uses GPUI's transparent titlebar and blurred background appearance.
The native traffic lights sit over the header by default (Settings → Appearance
can hide them), and the header shows only the app name; session state,
including the window-buttons choice, is written to `session.json` and restored
on launch.

## Binary front ends

`crates/flowlist/src/main.rs` dispatches to:

- `gui::run` when the `gui` feature is enabled and no subcommand is given;
- `cli::run` for clap commands;
- `mcp::run` for the stdio JSON-RPC server.

All three call the same `Store` and `Command` APIs. `render.rs` defines the JSON
shape used by `--json` and MCP `structuredContent`, so scripts and agents see
identical field names.

## Tests

- Unit tests cover time math, repeat rules, parsing, TaskPaper round-trips,
  migrations and query behaviour.
- `crates/flowlist-core/tests/core_flow.rs` exercises create/edit/complete,
  undo/redo, persistence across stores, merge import, repeat spawning and
  checklist CRUD/reordering.
- `cargo test --workspace` runs the full suite.
