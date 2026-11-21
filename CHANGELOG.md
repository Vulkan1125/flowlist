# Changelog

## Unreleased

### Interface

- Settings is now a sectioned sheet: a fixed rail (Appearance, Sidebar,
  Workflow, Nearby, Trash) beside one scrolling pane, so library-wide
  preferences have a home instead of lengthening one nameless list. `⌘1`–`⌘5`
  jump between sections, and `⇥` only walks the fields in the section on
  screen.
- The title-bar search keeps the inspector's column reserved when the
  inspector is closed, so it no longer moves between inspector states or slides
  under the **Add task** cluster.
- The macOS close, minimise and zoom buttons now sit over the header by
  default instead of being hidden, and Settings → Appearance → Window controls
  hides them again (`W` while the section is open). The header reserves the
  buttons' space while they show, the choice previews live, Cancel restores the
  state the sheet opened with, and the choice is saved with the session.
- The Settings sheet sizes itself to the window instead of a fixed height, so
  the theme grid, accent picker and window controls fit at once on a tall
  display and the pane scrolls on a short one.
- Rounded and bordered quads now snap to the device pixel grid when painted,
  so borders that used to land on half pixels render as solid one-pixel lines
  instead of two washed-out rows.
- Plain gains accent colours. Nine curated pairs each carry a light and a dark
  tone (Ink, Blue, Teal, Green, Amber, Orange, Rose, Pink, Violet), and a
  saturation/brightness square plus hue strip tune a separate custom accent
  for each appearance. Keys `1`–`9` choose a preset while Appearance is open.
  A system eyedropper samples any pixel — wallpaper included — and the result
  is luminance-clamped so accent text and primary-button labels stay at least
  4.5:1 on Plain's chrome. Accents are saved per appearance with the session;
  Cancel restores the pair the sheet opened with.
- Sidebar rows no longer show a pencil or × on hover. Rename, delete, project
  status and tag location live in the right-click menu, so names keep the full
  row width. A finished project's tick no longer paints a chip behind it.
- Visual design pass: task checkboxes stay visible on a white surface, row
  actions wait for hover, every empty view shares one illustrated halo,
  counts read as prose (`1 task`, `6 tasks`), menus, the toast and sheets fade
  in over 120 ms, and theme swatches preview each family as a miniature
  window. The status bar keeps the revision and library path in a tooltip.
- A new Plain theme family adds a pure white light palette and a near-black
  dark palette with a neutral ink accent. Its sidebar, inspector and header
  use a lighter Dia-style translucent wash, so the desktop shows through
  without tinting the glass. It is the first-run default for a library with no
  saved theme; existing saved themes are untouched.
- `⌃⌥Space` now opens a Things 3-style floating capture panel in the middle
  of the screen instead of bringing the whole window forward. The panel is a
  compact, square-cornered card that stays above normal windows when another
  app comes forward. It previews the parsed title and destination, always files
  a plain capture into the Inbox, honours an explicit project or due/defer
  date (so scheduled tasks leave the Inbox), closes on `Enter`, stays open on
  `⇧⏎`, selects the new task on `⌘⏎`, and every capture is undoable from the
  main window. `Enter` and `Esc` hand focus back to the app that was in front
  before capture.
- Inbox now means unscheduled: a task with a due or defer date leaves the
  Inbox for Today or Forecast, and clearing its dates files it back. Captures
  no longer inherit the last view's project, tag, priority or forecast date.
- The inspector's inline fields now save on `Tab` and `Enter` and move to the
  next field. A validation error keeps the current field open, and `Enter` on
  the last field finishes editing. A first-time task stays a draft until the
  whole session ends, so Delete and Complete no longer appear as soon as the
  title is saved.
- The New Project sheet has a Review every (days) field, pre-filled from the
  library default, so each project can keep its own review cadence. A project
  keeps that cadence editable afterwards: its review line in the banner and in
  the inspector is a button that opens a **Review interval** sheet, so changing
  the rhythm no longer means recreating the project. Undo restores the previous
  interval and the next review date moves with it.
- Task rows read their right-edge metadata in one fixed order — attachments,
  project, tags, estimate, due, deferred, repeat, checklist — instead of the
  order the fields happened to be added. Tags now print with a leading `#`, so
  a tag chip can never be mistaken for a project name.
- The × on an attachment row now asks first, naming the file or link and
  noting that nothing on disk is touched. Confirming unlinks it, and Undo puts
  the attachment back.
- The content header no longer repeats the last action and its Undo sentence;
  the status bar still carries both. The Settings sheet drops its
  library-wide subtitle, since the section rail already names what it holds.
- Icon tooltips now wait three seconds instead of about five.
- Nested folder rows show just the folder's own name; indentation carries the
  path instead of printing `Parent / Child` on every row.
- Completing a repeating task now increments the last number in its title, so
  “Week 3 review” spawns “Week 4 review”. The rule's end date or count still
  stops the series.
- Deleting a repeating task asks whether to trash just this occurrence or the
  whole series. The sheet defaults to this occurrence, Tab or the arrow keys
  switch choices, and one Undo restores every trashed occurrence. `remove
  --series` and the MCP `delete_task` tool's `series` flag do the same.
- Attachments now accept http(s) website links next to file paths. The
  inspector's **Add link…** button takes a pasted address, a link row uses the
  chain icon, and a task whose attachments are all links shows a link badge
  instead of the paperclip. `--attach` and the MCP attachment fields take
  links too.
- Deleting a folder now takes its nested folders, their projects and every
  task inside to Trash as one undoable step. The sheet names the folder and
  says how many projects go with it.
- Folder rows drag like project rows. Drop one on another folder to nest it,
  or on the LIBRARY heading or empty rail space to move it back to the root;
  a folder never lands inside itself or its own subtree.
- Sidebar tags now sort by live task count, busiest first, then by name
  ignoring case. Nested tags follow the same order inside their parent, and
  completed tasks no longer lift a tag up the list.
- A folder's count now reports live projects instead of everything filed under
  it. A project whose tasks are all ticked, or a closed project, no longer
  inflates the badge; nested folders roll their live projects up.
- The title bar now has back and forward arrows beside the app name, with
  `⌘[` and `⌘]` as their keyboard routes. They walk the views the window
  came through, so Archive → project → back returns to the Archive; a new view
  drops the forward trail and deleted projects are skipped.
- Rail sections are calmer. Folders start folded on every launch, Priorities,
  Library, Saved and Tags fold from their headings, and a section's chevron
  waits at the right edge until the pointer rests on that heading.
  Perspectives stay fixed as the backbone.
- Settings has a Sidebar group that chooses which rail sections appear and
  which carry count badges. Counts default quiet outside the Library, while
  Today always keeps its day count. The choice is saved with the library.
- Archive rows can now be deleted, not just reopened. Delete asks in the usual
  confirmation sheet and runs through the normal command layer, so Undo
  restores the project and its folder group.
- The Settings sheet puts the Sidebar controls with Appearance so the new
  choices are visible without scrolling past the numeric defaults.
- Added an Archive to the sidebar, after Completed. Finished and dropped
  projects wait in collapsible folder sections with a project count, each row
  opens the project, and a Reopen button sets it back to Active through the
  ordinary command, so Undo puts it straight back. An empty Archive shows a
  calm “No finished projects” state.
- The bottom View sheet is now a settings editor instead of a row of cycling
  chips. Every condition has a labelled row that opens a menu of all its
  values, so Status, Due, Defer, Group and Sort are one click each, and the
  open menu takes `j`/`k`, arrows, Home/End, Enter and Esc. The sheet previews
  how many tasks the view matches as it is edited, Clear filters drops every
  condition, and conditions written by the CLI are named and kept on save.
- Forecast and Calendar are now one perspective. The rich month calendar
  moved above the Forecast list: one full-width month with a continuous grid,
  an accent circle for today, tinted event pills, `+N more` overflow and
  drag-to-reschedule. The duplicate Calendar perspective is gone.
- A Calendar chip in the content header folds the Forecast month grid away,
  and the grip under it resizes the calendar with the mouse (`-` / `=` from
  the keyboard). Both choices are saved with the session, and the grip is
  clamped so the day cells stay readable and the list keeps its room.
- Added a Today perspective, second in the sidebar: overdue work in its own
  group, then everything due today plus tasks deferred to today. A Hide
  overdue chip (remembered with the view) drops the overdue group, and a clear
  day shows “Nothing due today” instead of an empty list.
- Theme and appearance choices in Settings now preview on the whole window as
  soon as they are clicked, including `⌘T` inside the sheet. Cancel or Esc
  restores the theme the sheet opened with, and Save keeps the new one.
- Deleting a folder now asks first, in a sheet that names it and explains that
  its projects move up a level. The × button, the right-click menu and the
  sidebar's `⌘⌫` / `dd` all use the same confirmation, and Undo still restores
  the folder in one step.
- Task rows use the app's drawn icons for their metadata instead of emoji: a
  clock marks a future defer date, a tick marks checklist progress, and the
  repeat glyph marks a recurrence. The defer date no longer shows an
  hourglass that sat apart from every other mark.
- Clicking a sidebar folder opens it and unfolds its tree; clicking the same,
  already-open folder folds the tree again without leaving the folder view.
  The chevron keeps toggling just the tree, and `Enter` on the row matches the
  mouse.
- The Due and Defer date pickers are now a two-step surface. A proper month
  calendar replaces the cramped date-plus-stepper card, and choosing a timed
  day opens an Android-style clock face: pick the hour, it moves on to
  minutes, then Done writes the stamp. All-day picks still commit in one
  click, so a plain date stays quick. The clock shows the date being set, an
  AM/PM pair and a Back button, and every control has a tooltip and a keyboard
  route.
- The inspector no longer repeats the completion circle beside the task status;
  the Complete / Reopen button at the bottom of the panel is the one control
  for that. The status text and flag stay at the top.
- Subtasks now sit indented under their parent in every list instead of being
  hidden until the first click. The disclosure chevron collapses a whole
  branch in one click (the first click used to do nothing), and `za`, `zo` and
  `zc` fold the selected task from the keyboard.
- Projects can now be dragged between folders in the sidebar. The folder row
  or LIBRARY heading under the pointer highlights before the drop, empty
  sidebar space under the tree also returns a project to the root, and the
  list repaints immediately instead of waiting for the toast. Dropping a
  project where it already sits is a silent no-op, so it adds no undo entry.
  Every real move goes through the normal move command, so `⌘Z` restores the
  previous placement, and task drags onto projects keep working.
- Folder contents now indent under their folder, one step per nesting level,
  including projects and subfolders. An empty folder says “Empty” instead of
  the root’s “No projects yet”, and both notes line up with the names.
- Changed no longer lists completed tasks; completing a task removes it from
  the view and lowers the sidebar count, and undoing the completion puts it
  back. The filter also excludes tasks in Trash as before.
- New projects and tasks converted to projects use the Project icon by default
  again instead of the Folder glyph; Folder is still one of the ten choices.
- A sidebar project with tasks but nothing left open now shows a tick in place
  of its count. A brand-new project still shows 0, and adding any open task
  brings the number back.
- Removed the macOS close/miniaturize/zoom buttons from the app window.
- Reduced the title bar to the app name. The app badge and the
  `Flowlist / view` suffix are gone.
- Replaced the inspector toggle's sidebar glyph with an info (`i`) icon, in
  both the header and the inspector panel.
- Added a moon/sun button to the title bar, left of the inspector toggle. It
  switches light/dark in the current theme family and shows the mode it will
  switch to; `⌘T` still does the same. A manual switch turns off follow-system
  appearance so the choice sticks.
- Added a Monday-first month calendar to the top of Forecast, with previous /
  Today / next controls, an accent circle for today and event pills on dates
  that carry a task. The existing forecast buckets sit below it unchanged, and
  the header chip folds the grid away when the list needs the whole pane.
- Rename sheets now open with the whole name selected: typing replaces it,
  one Backspace or Delete clears it, and arrows or a click place the caret.
  Sheet text fields also accept a click or drag, so a range of a name can be
  selected with the mouse instead of erased key by key.
- The inspector's Due and Defer fields gained a calendar icon that opens a
  small month picker right under the field. Clicking a day applies it as a
  normal undoable edit, keeping an existing time of day; Today, Clear, month
  arrows and keyboard navigation are included, and Esc closes without change.
- The picker's Time row adds or removes a time of day. All day toggles between
  a date-only stamp and the library's default due/defer time, −/+ step 15
  minutes (an hour with `{`/`}`), and the chosen time is written with the day.
- Header and status chrome now use a flat glass tint; only the sidebar and
  inspector keep the vertical tint wash. This removes the unintended
  left-to-right gradient ramp under the title bar.
- Added `docs/backlog.md` and `AGENTS.md` so outstanding work survives a
  context reset; `flowlist.md` is now the durable pending list.
- Deleting tasks now selects the row that took their place, so `j`/`k` keep
  working and the list never loses the cursor.
- Clicking a Forecast calendar date filters the list to that day. Click again
  or press `\` to clear; `[`/`]` step a day and `{`/`}` step a month.
- Project types: parallel, sequential and single-action projects, chosen in the
  New Project sheet and cycled from the project banner.
- Sort and Group header chips now open a list of every option instead of
  cycling blindly. `Esc` closes the menu.
- The Priority header chip now opens the same kind of menu (All priorities,
  High, Medium, Low) instead of cycling through the filter.
- Project creation sheets no longer show a second close control in the top
  corner; their footer already has Cancel.
- Tooltips wait about five seconds before appearing, and clicking a button
  keeps its tooltip quiet until the pointer leaves the button. Hovering again
  later restores the normal behaviour.
- The **Add task** action moved to the title bar as a labelled plus button,
  so it is available in every view and the content header keeps its full width
  for the view controls.
- The content header keeps only the view title and the Sort/Group/Filter chips
  on its first row, with the subtitle on its own row so it is never squeezed.
  If the view name cannot fit beside the controls it is left out entirely
  instead of being clipped to a few letters.
- Search moved into the title bar as a full field at every window width, so it
  no longer competes with the view title and chips. The field is centered on
  the content pane, not on the whole window, and a × at its right edge clears
  the query in one click. The title-bar theme (palette) icon is gone; `⌘T` and
  `⇧⌘T` / `⌥⌘T` still change the theme.
- Sidebar rows spend their width on the name until the pointer arrives. Only
  the hovered row makes room for its pencil and ×, and its long name then fades
  out at the edge instead of adding an ellipsis. At rest long project, folder,
  tag, perspective and review-card names still end with an ellipsis (…). The
  inspector shows the project name in full and wraps it instead of cutting it.
- Dropdown menus and sheets now block the window underneath, so clicking a
  Sort, Group or Priority option no longer also selects or highlights the row
  beneath the menu, and a dimmed sheet backdrop swallows clicks instead of
  passing them to the list.
- The Priority filter is remembered per view. Opening a sidebar priority row
  no longer leaves that filter on the next perspective, and each perspective
  keeps its own choice.
- Sidebar Forecast, Flagged and Completed badges, plus the status bar's open
  and available totals, no longer count tasks sitting in Trash.
- Project rows in the Library use the same small boxed count as perspectives,
  and their names line up with the folder icon and count instead of sitting a
  couple of pixels low.
- Sort and Group menus open directly under their chip, stay above the task
  list, and close on any click outside. The outside-click scrim now closes on
  mouse-up, so the menu item under the pointer still receives its click.
- Right-clicking any sidebar row opens a menu for that row: open, rename,
  delete, new child, project status, tag location and Empty Trash where they
  apply. The menu opens under the pointer and `Esc` closes it.
- Sidebar rows reveal their edit and delete buttons on hover, including the
  selected row, which no longer pins them visible. Project rows gained a
  delete button.
- Project deletion is an undoable command in the GUI, CLI (`project delete`)
  and MCP (`delete_project`). Deleting a project moves its tasks to Trash and
  detaches the project from its folder. The sidebar's × now opens a Delete
  project sheet that names the project, so one click cannot remove it by
  accident; Enter confirms and Esc cancels.
- Typing an unknown `Folder / Project` path in the task inspector now asks
  before creating the project. Dismissing the prompt leaves the task alone.
- The inspector's project picker only opens once you start typing, instead of
  covering the panel with every project as soon as the field is focused.
- New projects default to the folder icon instead of the generic project
  glyph.
- Added a **First Available** header toggle that hides blocked rows.
- Sort, grouping, completed visibility and availability are remembered per
  perspective.
- Task inspector: parallel/sequential chips for action groups, and an
  attachments section that can add file paths, open them, and remove them.
- **Convert to project** turns a task and its top-level subtasks into a project
  in one undoable step, then opens that project. Attachments move to the
  project and leftover checklist items become tasks, so nothing is lost.
- Project icons: a fixed set of ten glyphs, chosen in New Project and the
  project inspector, rendered in the sidebar and dashboard. The grid stays
  folded behind the current icon until it is clicked.
- Project and tag fields complete paths as you type: prefix matches first,
  `↑`/`↓` to highlight, `Tab` or click to accept. Spaces around `/` are
  optional.
- The system-wide `⌃⌥Space` shortcut opens capture while another app is in
  front. Flowlist activates, switches to Inbox if the current view has no
  capture bar, and selects any previous capture text so typing starts fresh.
- Follow-system appearance keeps the chosen theme family and takes light/dark
  from macOS.
- Changing the theme now shows a toast naming the family and appearance.
- Notes wrap in the inspector instead of clipping.
- Review uses a coffee-cup icon, and Mark reviewed sits on the title row.
- Dark palettes raise selection, hover and accent-soft contrast, and the glass
  chrome is a little more opaque so text stays readable.
- Project rows show open-task counts; folder rows count open projects.
- Long view headings no longer wrap into the toolbar.

### Library

- **Trash** is a ninth built-in perspective. Delete moves tasks and their
  subtrees to Trash; `r` restores them, Delete forever purges them, and
  Settings controls auto-delete after a retention period (`0` keeps forever).
  The automatic sweep runs as a normal command and is undoable. The Trash
  header has a **Select all** button, and `⌘A` selects every trashed task, so
  the whole bin can be restored or purged in one action.
- **Priorities**: optional High / Medium / Low, shown as a coloured mark on
  rows, editable in the inspector, filterable from every view, and available
  as sidebar rows plus CLI (`--priority`) and MCP fields.
- **Project kinds** are persisted alongside the legacy `sequential` flag, so
  older libraries keep their sequential behaviour.
- **Changed** is the tenth built-in perspective, reached with `0`. Introspection
  and editing times feed a configurable `changed_days` window.
- **Attachments** store referenced file paths on tasks and projects; files are
  never copied into the library. Rows show a paperclip count and the
  **Attachments** perspective lists every open task that has one. CLI:
  `--attach`, `--clear-attachments` and `list --attachments`.
- CLI: new `trash` and `convert` commands, `--kind`/`--icon` on projects and
  `--changed`/`--attachments` on `list`. MCP: trash tools,
  `convert_task_to_project`, attachment fields and `icon` on project tools,
  and an `attachments` view for `list_tasks`.
- Added an OmniFocus 4.3 comparison and the next adopted gaps to
  `docs/backlog.md`.

## 0.1.0 — 2026-11-13

First complete release of Flowlist: one Rust library and three front ends.

### Highlights

- **Native desktop app** built with GPUI 0.2.2 — no web view, Tauri or Electron.
- **Keyboard-first interaction**: app-wide vim keybindings, macOS shortcuts,
  delayed tooltips on icon-only controls and a searchable help card.
- **Scriptable CLI** and **MCP server** over the same domain layer, so every
  front end shares validation, repeat rules, search and undo.
- **One local library**: atomic JSON writes, backups, a cross-process lock and
  automatic reload when another process writes.
- **Full task model**: projects, nested folders, tags, due/defer dates,
  estimates, repeat rules, flags, subtasks, checklists with progress, review
  scheduling and nearby tasks based on tag locations.
- **Saved perspectives** with a rich editor for common conditions, grouping and
  sort; unknown filter nodes survive editing unchanged.
- **Everything is undoable**: every GUI, CLI and MCP mutation goes through one
  command engine and one history.

### Interface

- **Ten theme families** — Paper, Mist, Catppuccin, Nord, Gruvbox, Dracula,
  Tokyo Night, Vesper, Graphite and Midnight — each with light and dark
  palettes, chosen in Settings or with ⌘T / ⇧⌘T / ⌥⌘T.
- **Glass chrome**: the sidebar, inspector, header and status bar use GPUI's
  native macOS blur backdrop, while the task list keeps an opaque reading
  surface for contrast. The blur view is pinned to the palette's light/dark
  appearance and the `.underWindowBackground` material, and each pane runs a
  subtle tint gradient instead of a flat fill.
- **Full-height window**: the native titlebar is hidden, traffic lights sit in
  the Flowlist header, and the header doubles as the drag region.
- **One icon language**: embedded SVG icons share a 24px grid, 1.75px stroke
  and round joins, replacing inconsistent text glyphs throughout the app.

### Packaging

```sh
scripts/package.sh
open dist/Flowlist.app
```

`dist/Flowlist.app` bundles the GUI, CLI and MCP server in one executable. The
bundle is ad-hoc signed for local use; it is not Developer ID signed or
notarized. See [docs/packaging.md](docs/packaging.md).

### Quality checks

- `cargo test --workspace` covers the domain, undo/redo, import/export, query
  behaviour, the CLI and cross-process reload with the real binary.
- `scripts/perf-check.sh` exercises 1k, 10k and 30k task libraries; a
  10,000-task library answers a perspective in under 200 ms including process
  start-up. See [docs/performance.md](docs/performance.md).
- `scripts/gui-smoke.sh` drives the mouse-only New project and perspective
  pencil flows with real CoreGraphics events. See
  [docs/gui-automation.md](docs/gui-automation.md).
- Accessibility: every icon-only control has a tooltip and a keyboard route
  where one is possible. GPUI 0.2.2 does not expose an accessibility tree, so
  VoiceOver cannot inspect internal controls yet; that limitation is documented
  in [docs/accessibility.md](docs/accessibility.md).

### Known limitations

- Ad-hoc signing only; distribution to other Macs needs a Developer ID
  certificate and notarization.
- The GUI smoke test depends on the current sidebar geometry and fails loudly
  if that geometry changes.
- Tag removal from the inspector has no dedicated keyboard shortcut yet; use
  the inspector Tag field.
- GPUI 0.2.2 lacks an accessibility tree, so screen-reader support is limited to
  the window title; the CLI and MCP server are the fallback for assistive
  workflows.

### Documentation

[README](README.md) · [CLI](docs/cli.md) · [MCP](docs/mcp.md) ·
[Architecture](docs/architecture.md) · [Features](docs/features.md) ·
[Checklists](docs/checklists.md) · [Packaging](docs/packaging.md) ·
[Performance](docs/performance.md) · [Accessibility](docs/accessibility.md) ·
[GUI smoke test](docs/gui-automation.md)
