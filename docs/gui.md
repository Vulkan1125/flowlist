# Flowlist GUI guide

Flowlist is a native GPUI window over the same local library the CLI and MCP
server use. This page is a tour of the window and the behaviour you can rely
on. For the exhaustive checklist see [features.md](features.md); for keyboard
routes and tooltips see [accessibility.md](accessibility.md).

## The window at a glance

| Area | What it does |
| --- | --- |
| Title bar | app name, back/forward arrows, search, **Add task**, theme, help |
| Sidebar | perspectives, priorities, library folders and projects, saved views, tags |
| Content | the selected view, its Sort/Group/Filter chips, and the task rows |
| Inspector | details for the selected task or project, toggled with `i` or `⌘I` |

The window is full-height glass with the macOS traffic lights hidden. The title
bar blends into the content so the sidebar is the visual anchor.

### Title bar

- **Back / forward** sit beside the app name and walk the views the window came
  through (`⌘[` / `⌘]`). A new view drops the forward trail, deleted projects
  are skipped, and the arrows dim when there is nowhere to go.
- **Search** is centered on the content pane at every width. The × clears it.
  Search covers task titles and notes, checklist steps, projects, folders, tags
  and perspectives.
- **Add task** focuses capture from any view, including a view that has no
  inline capture bar.
- The theme button switches light/dark (`⌘T`); `⇧⌘T` / `⌥⌘T` step through the
  eleven theme families. Plain, the first-run default, is a pure white or
  near-black surface with neutral ink and an almost imperceptible chrome wash,
  so it stays neutral over any wallpaper.
- Help (`?`) opens the keyboard and mouse card. Settings (`⌘,`) opens the
  settings sheet.

## Views

The sidebar's fixed perspectives are the backbone:

| View | Contents |
| --- | --- |
| Inbox | tasks with no project and no due/defer date |
| Today | due today, deferred to today, and overdue; overdue can be hidden |
| Projects | open projects and their tasks |
| Forecast | overdue / today / tomorrow / later, with a collapsible, resizable month calendar |
| Flagged | flagged tasks |
| Review | projects due for review |
| Tags | tasks grouped by tag |
| Completed | completed tasks |
| Archive | finished and dropped projects, grouped by folder |
| Changed | recently edited tasks that are still open |
| Attachments | open tasks with file paths |
| Nearby | tasks whose tagged location is inside the radius |
| Trash | deleted tasks, with Restore, Delete forever and Empty |

Saved perspectives appear after the built-ins. They can use any filter the
View sheet can express, plus conditions imported from the CLI/MCP that the GUI
keeps but does not show.

### Content header

The view title and Sort / Group / Filter chips share one row; the subtitle sits
on its own row. A view name that cannot fit beside the controls is omitted
rather than clipped. Chips open anchored menus with an outside-click scrim.
Only one chip menu is open at a time; Escape closes it.

Grouping choices are project, folder, tag, due, defer, flagged and estimate.
Sort choices are manual, due, defer, name, flagged, estimate, created and
completed. The priority filter is stored per view. First Available keeps only
rows that are currently actionable.

## Capture and editing

`⌃⌥Space` is the system-wide capture hotkey. It opens a compact floating panel
over the current app instead of bringing the whole window forward, so the main
window keeps its view, selection and scroll. The panel stays above normal
windows when another app comes forward, the way Things' quick entry does.
`n` or `⌘N` starts capture inside the app, and the title-bar plus opens the same
panel from any view.

The panel is a square-cornered card in the middle of the screen, so it reads as
a single surface rather than a second app window. It parses as you type: the top
line is the title, a row of chips previews the project, tags, flag, due/defer
dates, estimate and repeat rule, and the footer says where the task will land.
A plain thought always goes to the Inbox, whatever view was last open. An
explicit `::Project` files it there, and a `due:` or `defer:` date schedules it
into Today or Forecast, so a scheduled task leaves the Inbox.

| Key | What it does |
| --- | --- |
| `Enter` | File the task and close the panel |
| `⇧Enter` | File the task and stay open, ready for the next thought |
| `⌘Enter` | File the task, close the panel and select it in the main window |
| `Esc` or `⌘W` | Close without filing and hand focus back to the previous app |

Every capture goes through the same command and store layer as `flowlist add`,
so one `⌘Z` in the main window reverses it. A plain `Enter` and `Esc` hand focus
back to the app that was in front before capture; `⌘Enter` keeps Flowlist in
front because it reveals the new task. Reopening the hotkey reuses the same
panel and selects any text left in the field.

Capture uses the same quick-entry parser as `flowlist add`:

```text
Draft launch email #Work/Website ::Work/Launch due:tomorrow est:45m ! // ask Sam
```

`#Tag` and `#Parent/Child` attach tags (creating missing ones), `::Project` and
`::Folder/Project` file the task, `due:` / `defer:` accept natural-language
dates, `est:` takes a duration, `repeat:` takes a repeat rule, `!` flags the
task, and `//` starts a note. Wrap phrases with spaces in parentheses.

Inside a task row you can edit the title inline, open the inspector for notes,
tags, project, dates, estimate, repeat, attachments and priority, and add
checklist steps. In the inspector, `Tab` and `Enter` save the current field and
move to the next one; a validation error keeps that field open, and `Enter` on
the last field ends the session. A first-time task stays a draft until then, so
its Delete and Complete actions stay hidden while you are still filling it in.
Attachments take local file paths and http(s) links side by side: **Attach
file…** opens the macOS picker and **Add link…** takes a pasted address.
Checklist steps are lightweight and never appear as tasks; completing a
repeating task copies the checklist with every step unchecked. The last number
in a repeating title moves on with each occurrence, so “Week 3 review” spawns
“Week 4 review”. Deleting a repeating task opens a sheet with a **This
occurrence** / **Whole series** choice; `Tab` or the arrow keys flip it, `Enter`
confirms, `Esc` cancels, and one Undo restores everything.

Drag a task onto a project, tag, parent or sibling position. Drag a project or
a folder onto a folder, onto the LIBRARY heading, or onto empty space under the
tree to return it to the root. A folder nests inside another folder but never
inside itself or its own subtree. Drops are undoable.

## Sidebar

Folders launch **folded**, so a fresh window is quiet. Clicking a folder opens
its view and unfolds its tree; clicking the same open folder again folds the
tree without leaving the view. The chevron toggles just the tree. Nested
folders and projects indent one step per level and each row shows only its own
name, so indentation carries the path instead of repeating `Parent / Child`.
An empty folder says “Empty” aligned with the names.

Folder badges count **active projects**, not everything filed under the folder:
a project whose tasks are all ticked, or a closed project, does not inflate the
number. Nested folders roll their live projects into the parent's badge.

Tags sort by live task count, busiest first, then by name ignoring case, with
the id as a stable tie-break. Nested tags follow the same rule inside their
parent. Completed tasks do not lift a tag up the list.

Perspectives stay fixed. Priorities, Library, Saved and Tags fold from their
headings; the chevron waits at the right edge and appears while the pointer
rests on that heading. Settings can hide whole sections and choose which rows
carry counts. The default is quiet: every section is visible, only the Library
shows a number, and Today always keeps its day count. The choice is
library-wide and survives a relaunch.

Every sidebar row has a right-click menu: Open, Rename, Delete, New child,
project status, tag location, and Empty Trash where it applies. Hovering a row
reveals its pencil and × without stealing width from the name at rest.

Task rows follow the same rule: the flag control and hover wash appear only
under the pointer, so an idle list is titles and metadata. Empty views share
one illustrated halo, counts read as prose (`1 task`, `6 tasks`), and menus,
toasts and sheets fade in over 120 ms. Settings previews each theme family as
a miniature window.

## Projects, review and archive

Projects can be parallel, sequential or single-action and carry a status of
active, on hold, done or dropped. A project with tasks but nothing left open
shows a tick in the sidebar instead of `0`; a brand-new project keeps its `0`
until it has a task. Project rows also show their icon, and all ten icons are
available in the New Project sheet.

Review shows projects whose next review date has arrived. Each project keeps
its own review interval, set in the New Project sheet (pre-filled from the
library default) or with `flowlist project add --review-days`; completing a
review advances the next date by that project's interval.

The Archive lists finished and dropped projects grouped by folder. A row opens
the project, Reopen brings it back without leaving the Archive, and Delete asks
in the usual confirmation sheet. Undo restores the project and its folder
group.

## Modals and settings

The New Project, New Folder, New Tag, New Perspective, Rename, Delete, Import,
Settings, View and Tag Location sheets share one visual language: a title,
subtitle, fields, and Cancel / confirm in the footer. Fields accept a click for
the caret and a drag for selection. Rename sheets select the whole name on open
so typing replaces it. The New Project sheet adds Notes and a Review every
(days) field, so each project can have its own cadence.

Settings groups:

- **Appearance** — theme family, light/dark, follow-system appearance.
- **Sidebar** — which rail sections appear and which rows carry counts.
- **Numbers** — Changed window, nearby radius, Trash retention and other
  defaults.
- **Import / export** — JSON or TaskPaper, merge or replace.

A theme choice applies immediately; closing Settings does not discard it. The
sidebar choice is saved with the library, so the CLI and MCP see the same
settings document.

## Data and safety

Everything is one JSON library in a directory (`FLOWLIST_HOME` or
`--data-dir`). Writes are atomic, with a previous-good backup, cross-process
locking and revision-based reloads. Every mutation — including GUI actions,
sidebar deletes, drag/drop moves and Archive deletes — goes through the same
command layer, so `⌘Z` and `flowlist undo` can undo it.

The app never copies attachments into the library; it stores their paths.
Nothing leaves the machine unless you export or sync the library yourself.

## Automation

`FLOWLIST_TEST_STATE=/path/to/state.json` makes the window write a JSON
snapshot of its view, focus, modal, sheet state and measured boxes. The GUI
smoke test and the focused probes use those boxes instead of hard-coded
offsets. See [gui-automation.md](gui-automation.md).

## Related pages

- [CLI reference](cli.md)
- [MCP tools](mcp.md)
- [Feature checklist](features.md)
- [Accessibility and keyboard routes](accessibility.md)
- [Data model and architecture](architecture.md)
