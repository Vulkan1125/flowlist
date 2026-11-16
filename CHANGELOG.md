# Changelog

## Unreleased

### Interface

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
  Today / next controls and an accent bar under every date that has a task. On
  a wide pane (the working window, full screen, or the minimum window without
  the inspector) it shows two consecutive months side by side, so a single
  month never stretches into wide, empty cells. The existing forecast buckets
  below it are unchanged.
- The Forecast calendar's Months chip cycles Auto, one month and two months,
  and its width chip switches between calendar-sized cells and the full pane.
  Both choices are library settings, so they survive a relaunch and apply in
  every window; `M` and `W` do the same from the keyboard.
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
