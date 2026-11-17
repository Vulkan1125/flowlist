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
- [x] Inline subtasks: children sit indented under their parent in every list,
      expanded by default, with a disclosure that collapses one branch.
      `za`/`zo`/`zc` fold the selected branch from the keyboard
- [x] Project types: parallel, sequential and single actions
- [x] Optional High / Medium / Low task priorities
- [x] Task and project attachments stored as referenced file paths
- [x] Fixed project icon set (project, folder, star, briefcase, home, heart,
      book, code, rocket, coffee); new projects start with the project icon
- [x] Restorable Trash with configurable auto-delete
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
- [x] Today (due today plus overdue; deferred-to-today work joins the day, and
      a Hide overdue chip drops the overdue group. A clear day shows a calm
      “Nothing due today” message)
- [x] Projects
- [x] Forecast (overdue / today / tomorrow / later)
- [x] Flagged
- [x] Review
- [x] Tags
- [x] Completed
- [x] Changed (recently edited and still open, configurable window)
- [x] Attachments (open tasks with file paths)
- [x] Nearby (location + radius)
- [x] Trash (restore, delete forever, auto-empty)
- [x] Priority rows and a global priority filter
- [x] Custom saved perspectives with JSON filter trees
- [x] Availability: available, deferred, blocked, completed
- [x] Sorting: manual, due, defer, name, flagged, estimate, created, completed
- [x] Grouping: project, folder, tag, due, defer, flagged, estimate
- [x] Sort and Group menus that list every option instead of cycling
- [x] Priority menu with All priorities / High / Medium / Low, matching the
      Sort and Group chips
- [x] First Available filter (only the next actionable row)
- [x] Per-perspective sort, grouping, completed and availability memory

## CLI

- [x] `add`, `list`, `edit`, `done`, `remove`, `mv`, `convert`
- [x] `trash` list / `--restore` / `--delete` / `--empty`
- [x] `--priority` on add/edit/list, `--kind`/`--icon` on projects and
      `--attach`/`--clear-attachments` on tasks and projects
- [x] `--changed` and `--attachments` on `list`
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
- [x] Task tools: capture/add/get/update/complete/delete/move/convert/tags/list
- [x] Trash tools: list/restore/purge/empty
- [x] Priority and attachment fields on task tools; `kind` and `icon` on
      project tools
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

- [x] Full-height glass window with a blended header, persistent sidebar
      and optional inspector. The macOS traffic lights are hidden; the
      header shows the app name, a search field and an **Add task** button with
      a plus icon, which focuses capture from any view.
- [x] Content header layout: the view title and Sort/Group/Filter chips share
      one row, while the subtitle sits on its own row. A view name that cannot
      fit beside the controls is omitted instead of clipped. Search lives in
      the title bar at every window width, centered on the content pane, with
      a × that clears the query. The title-bar theme (palette) icon is gone
      while its keyboard shortcuts remain.
- [x] Sidebar rows give their full width to the name until hovered. Only the
      hovered row makes room for the pencil and ×, and its long name fades out
      at the edge without an ellipsis. At rest, long sidebar and review-card
      names end with an ellipsis, and the inspector wraps the full project name
      instead of truncating it. Library project rows carry the same boxed count
      as perspectives, aligned with the folder icon and the name. Once a
      project has work but nothing open, that count becomes a tick; a new
      project keeps its 0 until it has a task.
- [x] Sidebar folders indent what they contain: subfolders and projects step
      right once per nesting level, and an empty folder shows “Empty” aligned
      with the names. Root-level projects keep their usual position. Clicking
      a folder row opens it and unfolds its tree; clicking the same open
      folder again folds the tree without leaving the folder view, and a third
      click unfolds it. The chevron still toggles the tree on its own.
- [x] Forecast month calendar above the list: Monday-first grid, previous /
      Today / next controls and an accent bar under dates with a task. A wide
      pane shows two consecutive months side by side so the grid keeps
      calendar-sized cells instead of stretching. Clicking a date filters the
      list to that day; click again or press `\` to clear, with `[`/`]` day
      steps and `{`/`}` month steps. A Months chip cycles Auto / one / two
      months and a width chip toggles calendar-sized cells and the full pane;
      both are saved library settings. Alternating date cells carry a faint
      checker tint mixed from the theme's own text colour, so the grid reads
      as a checkerboard in every family without competing with the numbers.
- [x] Sidebar perspectives, Trash, priority rows, custom perspectives and
      project/folder/tag trees. Badges count shown tasks only, so Trash never
      inflates another perspective.
- [x] Task rows describe themselves with the drawn icon set instead of emoji:
      a clock for a future defer date, a tick for checklist progress and the
      repeat glyph for a recurrence, so the metadata matches the rest of the
      chrome.
- [x] Content list with grouped rows, progress indicators and inline status
- [x] Task capture with quick-entry syntax
- [x] Multi-select, range select, keyboard navigation
- [x] Complete/reopen, flag/unflag, delete, undo/redo and post-delete focus
- [x] Trash restore / delete forever, with retention set in Settings, plus a
      **Select all** button and `⌘A` for one-step restore or purge
- [x] Priority chips in the inspector and a priority filter in every view
- [x] Inline title/notes/tag/project/date/estimate/repeat editing
- [x] Rename sheets select the whole name when they open, so typing replaces
      it and one Backspace clears it. Clicks and drags inside sheet fields
      place the caret or select a range with the mouse.
- [x] Calendar icons on the inspector's Due and Defer fields open a two-step
      picker. The first step is a month calendar: a clicked day is applied
      through the normal command layer (so it is undoable), an existing time
      of day survives a date change, and Today, Clear, month arrows, arrow
      keys, Enter and Esc all work. An All day / At time switch decides whether
      the chosen day carries a time; an all-day pick commits at once, while a
      timed one opens the second step. That step is an Android-style clock
      face: the hour comes first and picking one advances to minutes, the
      digital hour/minute readout and the AM/PM chips switch what is being set,
      and Back returns to the calendar. Done (or Enter) writes the timed stamp.
- [x] Inspector with checklist add/toggle/rename/reorder/delete and progress
- [x] Task subtitle type chips (parallel/sequential action groups)
- [x] Attachments in the task and project inspectors: add paths, open,
      remove, plus a paperclip count badge on rows
- [x] Convert a task and its subtasks into a project in one undo step
- [x] Project icon picker collapsed behind the current icon button in the
      inspector and New Project sheet, with the chosen icon in the sidebar
      and dashboard
- [x] Project and tag autocomplete: prefix-first `Folder / Project` and
      `Parent / Child` paths, `↑`/`↓` to highlight, `Tab` or click to accept.
      The project list opens only after typing; an unknown path asks before
      creating the project
- [x] System-wide capture (`⌃⌥Space`) works while another app is in front;
      Flowlist comes forward with capture focused and switches to Inbox from
      perspectives that have no capture bar
- [x] Task progress badge for checklists
- [x] Drag/drop tasks into projects, tags, parents and sibling positions
- [x] Drag/drop projects into folders; drop one on the LIBRARY heading or on
      empty space under the tree to move it back to the root. The row or
      heading under the pointer highlights during the drag, and the list
      repaints as soon as the drop lands. The move is undoable like any other
      command; dropping a project where it already sits does nothing
- [x] Project dashboard with progress, status, review info and project type
- [x] New project/folder/tag/perspective modals (`⇧⌘N`, `⌥⌘N`,
      `⌃⌘N`, `⇧⌘P`)
- [x] Rich custom-perspective editor laid out like settings: labelled filter
      and layout rows that each open a menu of every value, a live match count,
      an explicit Clear filters, and unknown CLI conditions kept and counted
- [x] Rename project/folder/tag/perspective (`⌘R` or hover pencil);
      hover each sidebar row for its edit and delete buttons
- [x] Right-click any sidebar row for open/rename/delete, project status,
      tag location, new child items and Empty Trash
- [x] Tag locations from the sidebar (`⌖`) or `⇧L`, with validation and clear
- [x] Nearby ranks tasks by the distance between the library and tag locations
- [x] Delete folder/tag/perspective/project from the sidebar (`⌘⌫` or `dd`);
      deleting a project moves its tasks to Trash and is undoable
- [x] Deleting a folder asks first: its projects move up to the parent, the
      sheet names the folder, and Undo restores it. The × button, the
      right-click menu and the sidebar's `⌘⌫` / `dd` share the confirmation.
- [x] Project status management (active/on hold/done/dropped)
- [x] Import modal (JSON or TaskPaper, merge or replace)
- [x] TaskPaper clipboard export (`⇧⌘E`)
- [x] Settings and tag location modals
- [x] Session persistence: view, selection, inspector, sort, grouping,
      per-view options, per-view priority filter, First Available, theme,
      follow-system, vim mode and window size
- [x] Ten theme families — Paper, Mist, Catppuccin, Nord, Gruvbox, Dracula,
      Tokyo Night, Vesper, Graphite and Midnight — each with a light and a dark
      palette, chosen in Settings or with the title-bar moon/sun button,
      `⌘T` / `⇧⌘T` / `⌥⌘T` (`⇧T` in vim mode). Clicking a swatch or appearance
      chip previews it on the live window; Cancel or Esc puts back the theme
      the sheet opened with, and Save keeps the new one.
- [x] Follow-system appearance: keep the theme family, take light/dark from
      the macOS window appearance. A manual light/dark switch turns this off so
      the chosen mode is not reverted on the next frame.
- [x] Blurred sidebar, inspector, header and status bar over an opaque reading
      surface. The window's `NSVisualEffectView` uses the
      `.underWindowBackground` material and is pinned to the palette's
      light/dark appearance, so a dark theme never inherits a light material
      from macOS. The sidebar and inspector run a vertical wash between two
      tint stops; the header and status bar use the flat glass tint so the
      chrome does not ramp from left to right.
- [x] Embedded SVG icon set with a shared grid, stroke and alignment
- [x] Keyboard help (`?`)
- [x] Toasts retire after a few seconds and dismiss on click

## Packaging and quality

- [x] `scripts/package.sh` builds `dist/Flowlist.app` with an icon,
      validated `Info.plist` and an ad-hoc signature
- [x] `scripts/make-icon.sh` draws and packs the app icon
- [x] `scripts/perf-check.sh` generates large libraries and times the CLI
      query paths
- [x] Cross-process tests spawn the real binary and verify reload in both
      directions
- [x] `scripts/gui-smoke.sh` drives the mouse-only New project and perspective
      pencil flows with CoreGraphics events
- [x] Accessibility pass: tooltips on every icon-only control, keyboard routes
      for hover-only actions, documented GPUI limitation
- [x] Packaging, performance, accessibility and GUI automation docs

## Deliberate design decisions

- Checklist items are not tasks. They have no project, dates or tags and do not
  appear in perspectives.
- Project deletion is expressed as the `Dropped` status, matching the CLI and
  MCP model; folder and tag deletion are explicit and undoable.
- Ids are ULIDs, not sequential integers, so they remain stable across sync and
  merge imports.
- The GUI, CLI and MCP server never maintain separate copies of the library.

## Interaction model

### Vim keybindings across the whole app

Vim mode is a first-class input layer for the entire GUI, not a per-screen
shortcut set.

- Normal mode is the default outside text fields. Text fields keep native
  editing; `Esc` returns to normal mode.
- Motions work in the content list, sidebar, inspector, forecast, review,
  nearby, completed and custom perspectives: `h`/`j`/`k`/`l`, `gg`/`G`,
  `Ctrl-d`/`Ctrl-u`, and numeric counts such as `3j`.
- Actions share one grammar everywhere: `x` complete, `o`/`O` new task below or
  above, `dd` delete, `yy` yank, `p` paste, `u`/`Ctrl-r` undo/redo, `r` rename,
  `i` inspector, `/` search with `n`/`N` match cycling, `Enter` open or edit.
- Sidebar: `j`/`k` move between perspectives and entities, `l` expand, `h`
  collapse, `Enter` open, `r` rename or edit, `⇧L` set a location. Inspector
  and checklist panes use the same motions.
- Existing macOS shortcuts stay available, and the help card plus README list
  both maps. A settings toggle disables vim mode for users who prefer arrows.

### Delayed tooltips for icon-only controls

- One reusable tooltip component that stays hidden for roughly five seconds
  after hover begins. Clicking the button mutes its tooltip until the pointer
  leaves; hovering again later shows it normally.
- Applied to all icon-only controls: sidebar pencil and ×, completion circles,
  flag toggles, checklist add/toggle/reorder/delete, project status chips,
  disclosure arrows, toolbar and modal close buttons.
- Tooltips name the action and show its shortcut where one exists, for example
  "Rename (⌘R)" and "Delete (⌘⌫)".
- Tooltips never cover the row they describe. Mouse-only controls also have
  keyboard routes (`r` and `⇧L` in the sidebar).

## Next refinements

Outstanding work lives in [backlog.md](backlog.md) and the durable
`flowlist.md` list in the project root. The short version:

- Visual QA of the glass surfaces against busy wallpapers.
- Optional accent-colour variants on top of the ten base families.
- Additional automation coverage for mouse-only entity management flows.
