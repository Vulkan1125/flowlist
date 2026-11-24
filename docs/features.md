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
      collapsed by default, with a disclosure that opens one branch.
      `za`/`zo`/`zc` fold the selected branch from the keyboard
- [x] Project types: parallel, sequential and single actions
- [x] Optional High / Medium / Low task priorities
- [x] Task and project attachments stored as referenced file paths
- [x] Fixed 54-icon project set covering work, home, people, health, travel,
      food, study, sport, nature, media and tools; new projects start with the
      project icon
- [x] Restorable Trash with configurable auto-delete
- [x] Repeat rules: fixed/after-completion, due-again/defer-another, end date,
      end count, completion counter
- [x] Auto-incrementing repeat titles: the last number in a title moves on
      with each occurrence (“Week 3” becomes “Week 4”), stopping at the rule's
      end date or count
- [x] Series-aware deletion: deleting a repeating task asks whether this
      occurrence or the whole series goes to Trash, in one undoable step
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
- [x] Fuzzy list search with a **This view** / **Library** scope (`⌘⇧F`),
      ranked by relevance
- [x] Library statistics and review scheduling
- [x] Settings → About checks the latest GitHub release and can download,
      stage and install it beside the running bundle, then relaunch.

## Perspectives and queries

- [x] Inbox (tasks with no project and no schedule; a due or defer date
      files the task into Today or Forecast instead)
- [x] Today (due today plus overdue; deferred-to-today work joins the day, and
      a Hide overdue chip drops the overdue group. A clear day shows a calm
      “Nothing due today” message)
- [x] Projects
- [x] Forecast (overdue / today / tomorrow / later)
- [x] Flagged
- [x] Review
- [x] Tags
- [x] Completed
- [x] Archive (finished and dropped projects in folder sections, with Reopen
      and a guarded Delete that Undo can restore)
- [x] Changed (recently edited and still open, configurable window)
- [x] Attachments (open tasks with file paths)
- [x] Nearby (location + radius)
- [x] Trash (restore, delete forever, auto-empty)
- [x] Priority rows and a global priority filter
- [x] Custom saved perspectives with JSON filter trees, including a
      full-screen reminder filter that follows project and folder marks
- [x] Hide empty custom views option: a saved view leaves the sidebar until
      its filter matches again
- [x] Availability: available, deferred, blocked, completed
- [x] Sorting: manual, due, defer, name, flagged, estimate, created, completed
- [x] Grouping: project, folder, tag, due, defer, flagged, estimate
- [x] Sort and Group menus that list every option instead of cycling
- [x] Priority menu with All priorities / High / Medium / Low, matching the
      Sort and Group chips
- [x] First Available filter (only the next actionable row)
- [x] Per-perspective sort, grouping, completed and availability memory
- [x] Separate light and dark main-window backgrounds with a shared opacity

## CLI

- [x] `add`, `list`, `edit`, `done`, `remove`, `mv`, `convert`
- [x] `trash` list / `--restore` / `--delete` / `--empty`
- [x] `--priority` on add/edit/list, `--kind`/`--icon` on projects and
      `--attach`/`--clear-attachments` on tasks and projects
- [x] `--changed` and `--attachments` on `list`
- [x] `--archive` on `list` for finished and dropped projects
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
      and optional inspector. The macOS traffic lights show over the header by
      default and Settings → Appearance can hide them (`W` toggles them while
      that section is open); the header reserves room for them when they are
      visible. It shows the app name, a search field and an **Add task** button
      with a plus icon, which focuses capture from any view.
- [x] Back and forward arrows sit beside the app name in the title bar and
      walk the views the window came through (`⌘[` / `⌘]`). Going from the
      Archive to a project and pressing back returns to the Archive; a new
      view drops the forward trail, and a vanished project is skipped instead
      of leaving a dead arrow. The arrows dim at the ends of the history.
- [x] Content header layout: the view title and Sort/Group/Filter chips share
      one row, while the subtitle sits on its own row. A view name that cannot
      fit beside the controls is omitted instead of clipped. Search lives in
      the title bar at every window width, centered on the content pane with
      the inspector's column always reserved (so it stays put when the
      inspector toggles), and a × that clears the query. The title-bar theme (palette) icon is gone
      while its keyboard shortcuts remain.
- [x] Sidebar rows give their full width to the name. Rename, delete, project
      status and tag location live in the right-click menu, so no pencil or ×
      ever competes with the name. Long sidebar and review-card names end with
      an ellipsis, and the inspector wraps the full project name instead of
      truncating it. Library project rows carry the same boxed count as
      perspectives, aligned with the folder icon and the name. Once a project
      has work but nothing open, that count becomes a bare tick with no chip
      behind it; a new
      project keeps its 0 until it has a task.
- [x] Sidebar tags sort by live work, busiest first, then name ignoring case
      with the id as a stable tie-break. Nested tags follow the same rule inside
      their parent, and completed tasks do not lift a tag up the list.
- [x] Sidebar folders indent what they contain: subfolders and projects step
      right once per nesting level, and an empty folder shows “Empty” aligned
      with the names. Root-level projects keep their usual position. Folders
      launch folded, so a fresh window is quiet; clicking a folder row opens
      it and unfolds its tree, clicking the same open folder again folds the
      tree without leaving the folder view, and a third click unfolds it. The
      chevron keeps toggling just the tree.
- [x] A folder's badge counts the projects under it that still want work: an
      open project whose tasks are all ticked, or a closed project, no longer
      inflates the number. Nested folders roll their live projects into their
      parent's badge, so the count reflects active work at any depth. Rows show
      their own name, with indentation carrying the path.
- [x] Rail sections fold from their headings. Perspectives stay as the fixed
      backbone, while Priorities, Library, Saved and Tags collapse without
      touching the view. The heading's chevron sits at the right edge and
      only appears while the pointer is on that heading, so a resting rail
      reads as calm labels. Keyboard focus and Settings both respect a folded
      or hidden section.
- [x] Forecast month calendar above the list: Monday-first grid, previous /
      Today / next controls, an accent circle for today, tinted event pills,
      and `+N more` when a day overflows. One full-width month is shown, so
      the cells never stretch into empty slabs. Clicking a date filters the
      list to that day; click again or press `\` to clear, with `[`/`]` day
      steps and `{`/`}` month steps. A chip in the content header hides and
      restores the calendar, remembering whether it was the week strip or the
      full month; hiding clears any picked day so the list returns to the full
      agenda. While a day is picked, the header count names it — “1 task for
      23 Nov” — and returns to the plain “3 tasks” when cleared. A chevron
      beside the month name folds the full month to a
      Monday-first current-week strip (forward-only paging, day counts, a
      selected day that clears on a second click), and the strip's chevron
      opens the full month again.
      Dragging the full grid's bottom grip (or `-` / `=`) changes its height;
      the visibility, mode, week and height are saved with the session. Chips
      drag onto a day to reschedule through the ordinary command, so the move
      is undoable. The Forecast list keeps the normal task routes.
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
- [x] Inline title/notes/tag/project/date/estimate/repeat editing. `Tab` and
      `Enter` save the current field and move to the next; a failed validation
      keeps the editor open, and `Enter` on the last field ends the session.
      A first-time task stays a draft (with its Delete/Complete actions hidden)
      until that whole field session ends.
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
- [x] Attachments in the task and project inspectors: add file paths and
      website links, open either, remove, plus a paperclip (or link) count
      badge on rows and the shared Attachments perspective
- [x] Convert a task and its subtasks into a project in one undo step
- [x] Project icon picker collapsed behind the current icon button in the
      inspector and New Project sheet, with the chosen icon in the sidebar
      and dashboard
- [x] Project and tag autocomplete: prefix-first `Folder / Project` and
      `Parent / Child` paths, `↑`/`↓` to highlight, `Tab` or click to accept.
      The project list opens only after typing; an unknown path asks before
      creating the project
- [x] Things 3-style quick capture (`⌃⌥Space`): a compact square-cornered
      floating panel opens in the middle of the screen over whatever is in
      front, without moving the main window's view, selection or scroll. It
      previews the parsed title and destination, files plain text into the
      Inbox, files an explicit `::Project` into that project, and schedules a
      `due:` / `defer:` capture into Today or Forecast. The panel stays above
      normal windows when another app comes forward, closes on `Enter`, stays
      open on `⇧⏎`, reveals the new task on `⌘⏎`, and every capture is undoable
      from the main window
- [x] Task progress badge for checklists
- [x] Drag/drop tasks into projects, tags, parents and sibling positions
- [x] Drag/drop projects into folders; drop one on the LIBRARY heading or on
      empty space under the tree to move it back to the root. The row or
      heading under the pointer highlights during the drag, and the list
      repaints as soon as the drop lands. The move is undoable like any other
      command; dropping a project where it already sits does nothing
- [x] Project dashboard with progress, status, review info and project type
- [x] New project/folder/tag/perspective modals (`⇧⌘N`, `⌥⌘N`,
      `⌃⌘N`, `⇧⌘P`). The New Project sheet carries a Review every (days)
      field pre-filled from the library default, so each project can keep its
      own cadence; the review line in the project banner and inspector opens
      the same value for later edits.
- [x] Removing an attachment asks first and leaves the file on disk; Undo
      restores the row.
- [x] Image and PDF attachments preview inside the window, with PDF page
      stepping, and `o` still opens the file externally.
- [x] Due reminders through macOS UserNotifications, with date-only tasks at
      09:00 and a 30-day horizon. The permission is requested on enable; an
      ad-hoc build shows the signing requirement, while an Apple Development or
      Developer ID signature enables delivery.
- [x] Full-screen reminders are opt-in per task, project or folder. Marking a
      task (or a parent, project or folder above it) opens a borderless
      screen-sized reminder when it comes due while Flowlist is running, with
      Complete, Snooze, Open task and Dismiss. Quick capture accepts
      `fsr:1h30m` or `fsr:2d` (and bare `fsr` to follow due), and the
      project/folder sheets carry the mark at creation. Multiple reminders
      due together offer **View all**, which opens the Full-screen view. A
      mark without a due or explicit reminder time is ignored.
- [x] Mac backgrounds: a background sits behind the main window with one
      opacity choice; a Settings toggle reveals separate light and dark
      pickers. The reminder background has the same optional split. The
      full-screen reminder has its own backdrop: six
      theme-synced abstract styles (Aurora, Dunes, Halo, Mesh, Tide, Ember)
      with visual previews, a Match-theme default, or a separate background with
      its own opacity.
- [x] Full-screen reminders can follow the due date and time or carry their
      own reminder date and time. The inspector (Mac) and task editor
      (Android) expose both choices, the picker is disabled while matching
      due, and “Send full-screen test” opens the reminder screen directly.
      Snooze length is 15, 30 or 60 minutes. On the Mac reminder, **S**
      cycles the length, **Enter** snoozes and **⌘Enter** completes.
      Reminder timestamps earlier than now are refused.
- [x] Releases ship a drag-to-Applications DMG beside the updater's zip.
- [x] The plus opens the capture panel over the window (the global shortcut
      stays screen-centered), the toast follows the visible middle pane, and
      inspector boxes sit on a solid surface with visible rounded borders.
- [x] Rich custom-perspective editor laid out like settings: labelled filter
      and layout rows that each open a menu of every value, a live match count,
      an explicit Clear filters, and unknown CLI conditions kept and counted
- [x] Rename project/folder/tag/perspective (`⌘R`, or Rename in the row's
      right-click menu); the menu also carries delete, status and location
- [x] Right-click any sidebar row for open/rename/delete, project status,
      tag location, new child items and Empty Trash
- [x] Tag locations from the sidebar (`⌖`) or `⇧L`, with validation and clear
- [x] Nearby ranks tasks by the distance between the library and tag locations
- [x] Delete folder/tag/perspective/project from the sidebar (`⌘⌫` or `dd`);
      deleting a project moves its tasks to Trash and is undoable
- [x] Deleting a folder asks first: nested folders, their projects and every
      task inside move to Trash with it, the sheet names the folder and counts
      its projects, and one Undo restores the whole tree. The × button, the
      right-click menu and the sidebar's `⌘⌫` / `dd` share the confirmation.
- [x] Folder rows drag like projects: drop one on another folder to nest it,
      or on the LIBRARY heading or empty rail space to move it back to the
      root. A folder cannot be dropped into itself or its own subtree, and a
      drop is one undoable move.
- [x] Project status management (active/on hold/done/dropped)
- [x] Import modal (JSON or TaskPaper, merge or replace)
- [x] TaskPaper clipboard export (`⇧⌘E`)
- [x] Settings is a sectioned sheet: Appearance, Sidebar, Workflow, Nearby,
      Trash, Mobile and About sit in a fixed rail beside one scrolling pane, with `⌘1`–`⌘7`
      as the keyboard route. The Sidebar section chooses which rail sections
      appear and which ones carry count badges; the quiet default shows every
      section but only numbers the Library (and Today, which always carries its
      day count), so a one-task library does not look busy. The choice is
      library-wide and survives a relaunch.
- [x] Plain accent colours, kept to the Plain family. Nine curated preset
      pairs carry a light and a dark tone, so one choice keeps its character
      in both modes. A saturation/brightness square and hue strip edit the
      appearance on screen separately, keys `1`–`9` pick a preset, and a
      system eyedropper samples any pixel on screen (the background included).
      Every custom tone is luminance-clamped to preserve the 4.5:1 contract
      for accent text and primary-button labels. Cancel restores the accent
      pair the sheet opened with; Save persists both tones in the library
      session.
- [x] Session persistence: view, selection, inspector, sort, grouping,
      per-view options, per-view priority filter, First Available, theme,
      Plain's accent pair, follow-system, vim mode and window size
- [x] Eleven theme families — Plain, Paper, Mist, Catppuccin, Nord, Gruvbox,
      Dracula, Tokyo Night, Vesper, Graphite and Midnight — each with a light
      and a dark palette, chosen in Settings or with the title-bar moon/sun
      button, `⌘T` / `⇧⌘T` / `⌥⌘T` (`⇧T` in vim mode). Plain opens on a pure
      white reading surface in light mode and a near-black one in dark mode,
      and is the first-run default. Clicking a swatch or appearance chip
      previews it on the live window; Cancel or Esc puts back the theme the
      sheet opened with, and Save keeps the new one. Plain also carries the
      accent presets, custom picker and eyedropper; the other families keep
      their own accent as part of their identity.
- [x] Android backgrounds: the main window can show a custom background with
      its own opacity. Twelve ambient gradients, one tuned for each theme
      family, live on the full-screen reminder instead, with **Match theme**
      and free choice regardless of theme. The reminder can also use the
      background with a separate opacity.
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
      context-menu flow with CoreGraphics events, and `scripts/gui/forecast-probe.sh`
      covers the Forecast calendar's fold, day filter and drag-to-resize
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

- One reusable tooltip component that stays hidden for three seconds
  after hover begins. Clicking the button mutes its tooltip until the pointer
  leaves; hovering again later shows it normally.
- Applied to all icon-only controls: completion circles,
  flag toggles, checklist add/toggle/reorder/delete, project status chips,
  disclosure arrows, toolbar and modal close buttons.
- Tooltips name the action and show its shortcut where one exists, for example
  "Rename (⌘R)" and "Delete (⌘⌫)".
- Tooltips never cover the row they describe. Mouse-only controls also have
  keyboard routes (`r` and `⇧L` in the sidebar).
- [x] Visual design pass: row actions stay hidden until hover, every empty
      view shares one illustrated halo, counts read as prose (`1 task`,
      `6 tasks`), menus, the toast and sheets fade in over 120 ms, and theme
      swatches preview each family as a miniature window. Plain uses a lighter
      Dia-style translucent wash on the sidebar, inspector and header, so the
      desktop shows through without tinting the glass.

## Next refinements

Outstanding work lives in [backlog.md](backlog.md) and the durable
`flowlist.md` list in the project root. The short version:

- Additional automation coverage for mouse-only entity management flows.
