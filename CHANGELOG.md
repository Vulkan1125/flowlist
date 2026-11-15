# Changelog

## Unreleased

### Interface

- Removed the macOS close/miniaturize/zoom buttons from the app window.
- Reduced the title bar to the app name. The app badge and the
  `Flowlist / view` suffix are gone.
- Replaced the inspector toggle's sidebar glyph with an info (`i`) icon, in
  both the header and the inspector panel.
- Added a Monday-first month calendar to the top of Forecast, with previous /
  Today / next controls and an accent bar under every date that has a task. The
  existing forecast buckets below it are unchanged.
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
  no longer competes with the view title and chips. The title-bar theme
  (palette) icon is gone; `⌘T` and `⇧⌘T` / `⌥⌘T` still change the theme.
- Sidebar rows spend their width on the name until the pointer arrives. Only
  the hovered row makes room for its pencil and ×, and its long name then fades
  out at the edge instead of adding an ellipsis. At rest long project, folder,
  tag, perspective and review-card names still end with an ellipsis (…). The
  inspector shows the project name in full and wraps it instead of cutting it.
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
