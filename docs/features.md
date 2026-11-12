# Feature checklist

Flowlist is built feature-first; visual refinement is an ongoing pass on top of a
working domain and GUI. This page tracks what is implemented and verified.

## Core and data

- [x] Rust workspace with a headless `flowlist-core` crate
- [x] ULID ids with prefix and short-suffix lookup
- [x] Normalized task/project/folder/tag/perspective model
- [x] Explicit sibling ordering for tasks, projects and folders
- [x] Nested projects inside folders, nested tags
- [x] UTC storage with local-time date buckets and date-only stamps
- [x] Sequential task children
- [x] Repeat rules: fixed/after-completion, due-again/defer-another, end date,
      end count, completion counter
- [x] Checklist items with add/edit/reorder/delete and progress counts
- [x] Checklist copied unchecked when a repeating task spawns
- [x] JSON persistence with atomic writes, backups and schema migration
- [x] Cross-process lock, revision polling and reload
- [x] Persisted undo/redo snapshots (80 states)
- [x] Validation and `doctor --fix`
- [x] TaskPaper and full-JSON import/export
- [x] Merge import reconciles tags/containers by path
- [x] Search across tasks, notes, checklist steps, projects, folders, tags and
      perspectives
- [x] Library statistics and review scheduling

## Perspectives and queries

- [x] Inbox
- [x] Projects
- [x] Forecast (overdue / today / tomorrow / later)
- [x] Flagged
- [x] Review
- [x] Tags
- [x] Completed
- [x] Nearby (location + radius)
- [x] Custom saved perspectives with JSON filter trees
- [x] Availability: available, deferred, blocked, completed
- [x] Sorting: manual, due, defer, name, flagged, estimate, created, completed
- [x] Grouping: project, folder, tag, due, defer, flagged, estimate

## CLI

- [x] `add`, `list`, `edit`, `done`, `remove`, `mv`
- [x] `checklist list|add|set|move|remove`
- [x] `tag add|remove|set`, `tags`
- [x] `project add|list|status|review|edit`
- [x] `folder add|list|rename|remove`
- [x] `perspective add|list|remove`
- [x] `review`, `review done`
- [x] `search`, `stats`, `where`
- [x] `export`, `import`, `doctor`
- [x] `undo`, `redo`
- [x] `--json` for every command
- [x] `FLOWLIST_HOME` / `--data-dir`
- [x] Headless build with `--no-default-features`

## MCP server

- [x] Stdio JSON-RPC transport
- [x] Modern `2026-07-28` lifecycle and legacy `initialize`
- [x] Task tools: capture/add/get/update/complete/delete/move/tags/list
- [x] Checklist tools: list/add/update/delete/move
- [x] Project tools: list/create/update/status
- [x] Tag tools: list/create/delete
- [x] Folder tools: list/create
- [x] Review list and complete
- [x] Search, stats, library info
- [x] Undo/redo
- [x] JSON/TaskPaper import/export
- [x] `structuredContent` and read-only library resource

## GUI

- [x] Frameless window with transparent titlebar and blurred background
- [x] Sidebar perspectives, custom perspectives and project/folder/tag trees
- [x] Content list with grouped rows, progress indicators and inline status
- [x] Task capture with quick-entry syntax
- [x] Multi-select, range select, keyboard navigation
- [x] Complete/reopen, flag/unflag, delete and undo/redo
- [x] Inline title/notes/tag/project/date/estimate/repeat editing
- [x] Inspector with checklist add/toggle/rename/reorder/delete and progress
- [x] Task progress badge for checklists
- [x] Drag/drop tasks into projects, tags, parents and sibling positions
- [x] Project dashboard with progress, status chips and review info
- [x] New project/folder/tag/perspective modals
- [x] Rename project/folder/tag/perspective (`⌘R` or hover pencil)
- [x] Delete folder/tag/perspective from the sidebar
- [x] Project status management (active/on hold/done/dropped)
- [x] Import modal (JSON or TaskPaper, merge or replace)
- [x] TaskPaper clipboard export (`⇧⌘E`)
- [x] Settings and tag location modals
- [x] Session persistence: view, selection, inspector, sort, grouping, theme
- [x] Light/dark theme toggle
- [x] Keyboard help (`?`)

## Deliberate design decisions

- Checklist items are not tasks. They have no project, dates or tags and do not
  appear in perspectives.
- Project deletion is expressed as the `Dropped` status, matching the CLI and
  MCP model; folder and tag deletion are explicit and undoable.
- Ids are ULIDs, not sequential integers, so they remain stable across sync and
  merge imports.
- The GUI, CLI and MCP server never maintain separate copies of the library.

## Next refinements

- Further visual polish of the macOS-native theme, typography and spacing.
- Richer custom-perspective editor in the GUI (the CLI/MCP already accept the
  full filter tree).
- Additional automation coverage for mouse-only entity management flows.
