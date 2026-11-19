# GUI smoke test

GPUI 0.2.2 has no accessibility tree, so the mouse-only parts of the GUI cannot
be driven through named controls. `scripts/gui-smoke.sh` uses real CoreGraphics
mouse and keyboard events instead, and checks the result through:

- the library on disk, written by the same command engine as the CLI;
- `FLOWLIST_TEST_STATE`, a small JSON file the app writes when that environment
  variable is set.

```sh
cargo build --release
scripts/gui-smoke.sh
scripts/gui/forecast-probe.sh
scripts/gui/quick-capture-probe.sh
```

The script skips on non-macOS hosts and when `swiftc` is missing. It needs
Accessibility permission for the terminal (System Settings → Privacy & Security
→ Accessibility). Without it the clicks land nowhere and the test reports a
failure.

## What it covers

1. **New project button.** Clicks `＋ Project` at the bottom of the sidebar,
   waits for the sheet, types a name through the clipboard and creates the
   project. It then checks `flowlist project list --json` for the new name.
2. **Perspective pencil.** Clicks the pencil on a saved perspective and waits
   for the rich perspective editor. This is the mouse-only path that has no
   keyboard equivalent for opening the editor's conditions.
3. **Capture hotkey.** Activates Finder, posts `⌃⌥Space` and waits for
   `focus` to become `Capture`. Because Finder owns the menu bar first, this
   exercises the same background path a real user takes.

All flows use a temporary library, so the test never touches the real one.

`scripts/gui/forecast-probe.sh` covers the merged Forecast month grid: it
checks the painted day cells and event pills, folds the grid from the header
chip, filters the list by a date and clears it, then drags the bottom grip and
confirms the new height is clamped and persisted.

`scripts/gui/quick-capture-probe.sh` covers the floating capture panel: it
files a plain thought into the Inbox, files a dated thought into Forecast,
checks that the dated task left the Inbox, closes with `Enter`, returns focus
to Finder, confirms the panel keeps layer 3 while Finder is in front, and
checks that `Esc` closes it without moving the main view.

## The test-state hook

Setting `FLOWLIST_TEST_STATE=/path/to/state.json` makes the window write:

```json
{
  "view": "custom:01M79...",
  "modal": "Perspective",
  "focus": "Content",
  "field": null,
  "selected": 0,
  "draft": false,
  "suggestions": 0,
  "window": [1254.0, 1404.0],
  "toast": null,
  "modal_fields": [],
  "theme": "catppuccin",
  "appearance": "light",
  "blur": "light",
  "traffic_lights": "hidden",
  "modal_theme": null,
  "modal_close": true,
  "sort_menu": false,
  "group_menu": false,
  "priority_menu": false,
  "sidebar_menu": "project:01M79...",
  "sort_anchor": true,
  "group_anchor": true,
  "priority_anchor": true,
  "title_space": 80.0,
  "title_visible": true,
  "tip": "Show only the next available task (First Available)",
  "tip_muted": null
}
```

`sidebar_rows` maps each painted sidebar row (`project:<id>`, `folder:<id>`,
`tag:<id>`, `custom:<id>`, `library`, …) to its window-space box, so a probe
can drag a project onto a folder, open a saved perspective's editor, or assert
the order tags sort into, without hard-coding sidebar offsets. `sidebar_names`
maps the same rows
to the box where the row's icon starts, plus `empty:<folder>` for an empty
folder's note, which makes indentation assertable. `sidebar_drag_target` is
the row a dragged project is currently over (`folder:<id>`, `library`, or
null), so a split drag can check the highlight before releasing the mouse.
`modal_fields` lists every field in the open sheet with its label and current
text, which lets a probe check the New Project review default and the result of
`Tab` between fields. `toast` carries the live toast message, or null once the
timer or a click clears it.
`collapsed` lists the tree keys (`folder:<id>`, `tag:<id>`) the user has
folded, so a probe can assert a folder row's open/close click without relying
on pixels.
`sidebar_sections` maps each collapsible heading (`priorities`, `library`,
`saved`, `tags`) to its box, so a probe can fold a section by name.
`sidebar_visible` lists the row keys the rail targets on this frame, after
Settings visibility and section folds are applied, while `expanded` lists the
tree keys explicitly opened and `sidebar_settings` mirrors the persisted
`show_*`/`counts_*` choices. `settings_toggle_rows` maps every Sidebar control
in the Settings sheet to a box, and `modal_sidebar` reports the sheet's
unsaved copy, so a probe can click a toggle and check it before Save.
`sidebar_counts` maps each count badge actually painted (`builtin:<slug>`,
`priority:<slug>`, `folder:<id>`, `tag:<id>`) to its box, and
`sidebar_count_values` reports the number inside each badge. Today's badge is
always present, which makes the quiet count defaults assertable, while the
folder value proves a badge counts live projects rather than every project
under the folder.
`history` is the view trail and `history_index` the current entry;
`history_rows` maps the header arrows (`history-back`, `history-forward`) to
boxes, and `can_go_back` / `can_go_forward` report whether a valid view lies
that way.
`content_rows` lists the task rows
actually painted on the last frame, in order, with `id`, `title`, `depth` and
`has_children`; hidden descendants are absent, so a probe can assert what a
disclosure reveals. `task_disclosures` maps each parent id to its chevron box
for clicking it. `archive_projects` lists the finished and dropped projects in
library order with `id`, `name` and `status` (`done` or `dropped`);
`archive_groups` is the Archive's painted sections in order, each with its
`key` (`archive:root` or `archive:folder:<id>`), `title` (`Library` or the
folder path) and the project ids it holds. `archive_rows` maps each Archive row
and its Reopen and Delete buttons to a box (`project:<id>`, `reopen:<id>`,
`delete:<id>`), so a probe can open, revive or remove a project without
guessing at pixels. `date_picker` names the
open date field (`due` or `defer`, or
null) with `date_picker_step` (`calendar` or `clock`), `date_picker_unit`
(`hour` or `minute`), `date_picker_time` as `HH:MM`, `date_picker_cursor` and
`date_picker_month`. `date_picker_boxes` maps the picker's controls to their
boxes: `day:<YYYY-MM-DD>` for each calendar cell, `hour:<1-12>` or
`minute:<0-55>` for the clock numbers, and `all-day`, `at-time`, `clock-all-day`,
`today`, `clear`, `back` and `done` for the buttons. `view_menu` names the open menu in the View sheet (`project`, `tag`, `status`,
`due`, `defer`, `group` or `sort`, or null). `view_boxes` maps every View sheet
control to its box: the seven select rows by name, `option:<field>:<slug>` for
each open menu choice, and `clear` for Clear filters, so a probe can pick a
value directly. `view_form` reports the sheet's current state
(`name`, `inbox_only`, `flagged_only`, `show_completed`, `tag_children`,
`status`, `due`, `defer`, `project`, `tag`, `group`, `sort`, `extra`,
`matches`), where `extra` counts the conditions the GUI cannot show and
`matches` is the live task count. A fresh sheet has no Clear filters control,
so poll for `clear` only after a condition is set. `theme_boxes` maps a
Settings sheet's controls to their boxes: `theme:<slug>` for each family
swatch (`plain`, `paper`, `mist`, `catppuccin`, `nord`, `gruvbox`, `dracula`,
`tokyo-night`, `vesper`, `graphite`, `midnight`) and `appearance:<slug>` for
the `light` and `dark` chips, so a probe can check the live preview without
guessing at the grid. If the sheet does not paint another frame on its own, a
harmless Tab press forces one. Prepaint boxes land in the
state file one frame after they are measured, so a probe that opens the picker
should poll for the key (nudging the pointer forces the repaint). `due_anchor`
and `defer_anchor` give the calendar icon boxes. `hide_overdue` is Today's
header toggle and `hide_overdue_anchor` its chip box. `empty_note` is the title
the content pane painted for an empty list (for example `Nothing due today`),
so a probe can check the calm Today copy. `forecast_calendar_open`,
`forecast_calendar`, `forecast_calendar_height`, `forecast_days`,
`forecast_chips`, `forecast_toggle_anchor` and `forecast_grip_anchor` describe
the Forecast month grid: whether it is folded, the block and grip boxes, the
height the user dragged it to (null for the natural size), and the painted day
and chip boxes. A probe can drag the grip or click the header chip and then
check the same keys. `modal` is `null` when no sheet is
open, otherwise the sheet title (`New
project`, `Perspective`, `Settings`, …). `focus` names the active pane or field
(`Content`, `Capture`, `Sidebar`, `Field`, `Modal`, …). `field` names the
inspector field being edited (`Title`, `Notes`, `Project`, …) and is null
otherwise. `draft` is true while a
task created with `o`/`O` is still being titled, and `suggestions` counts the
rows in the open project picker. `traffic_lights` reports
whether the AppKit window buttons were hidden. `sort_menu`, `group_menu` and
`priority_menu` say whether the header menus are open; `sidebar_menu` names the
row a right-click menu belongs to (`builtin:inbox`, `project:<id>`, `tag:<id>`,
…) or is null. `sort_anchor`, `group_anchor` and `priority_anchor` confirm the
chip bounds needed to position those menus were measured. `title_space` is the
width the view title painted into on the last frame and `title_visible` says
whether it was drawn at all: a name that cannot fit in full is omitted.
Deleting a project from the sidebar sets `modal` to
`Delete project` while the project still exists; a probe confirms it,
presses Return and checks that the project's tasks moved to Trash. `modal_close` is
false for sheets that deliberately omit the corner close control, such as
project creation. `tip` names the tooltip under the pointer and `tip_muted` the
one a click silenced until the pointer leaves. The file is rewritten only when
one of those values changes, and the hook is inert unless the variable is set.
It does not contain task text, notes or the library itself.

The floating capture panel writes `capture.json` beside the main state file
when the same environment variable is set. Its keys are `open`, `text`,
`title`, `error`, `destination`, `last_captured`, `flash`, `window_active`,
`active_window` and `app_active`, so a probe can wait for the panel to take
focus, check the parsed destination, and confirm that a capture was filed.

## Why coordinates

The app exposes only its window title and bounds through the macOS
accessibility API. The script therefore reads the live window origin and clicks
at offsets that match the current sidebar layout. A handful of nearby offsets
are tried before failing, so a titlebar or spacing change does not produce a
false pass.

The offsets are:

| Target | Offset from the window's top-left |
| --- | --- |
| `＋ Project` | `(35, height − 77)` |

If the sidebar footer layout changes, update that row. The saved-perspective
pencil is no longer a fixed offset: the smoke test reads the row's live box
from `sidebar_rows` and clicks near its right edge. The script fails loudly
rather than pretending the flow worked. When GPUI gains an accessibility API,
these clicks can be replaced by named actions without changing the assertions.

## Helpers

- `scripts/gui/flowctl.swift` posts mouse moves, left and right clicks, scroll
  wheel lines and key events. Its modifiers include `ctrlalt`, used for the
  capture hotkey.
  `drag` does a complete drag in one call; `down`, `dragto` and `up` split it
  so a probe can read the test state mid-drag and assert the drop highlight.
- `scripts/gui/window.swift` prints the live window title, origin and size.
  `window <pid> move <x> <y>` also activates the app and asks the window
  server to move it, which keeps absolute clicks reliable when macOS restores
  a window partly off screen. `window <pid> layers` lists every on-screen
  window for the process with its layer and rectangle, which is how the
  capture probe proves the panel sits above normal windows. `window front`
  prints the frontmost app's process id.
- Both are compiled into the temporary directory when the smoke test runs.

## Probes

Focused probes use the test-state boxes above and run against a temporary
library:

- `scripts/gui/phase4-probe.sh` covers quick capture from several views,
  Undo, Escape, `⇧⏎`, `⌘⏎`, and the rule that a plain capture always goes to
  the Inbox.
- `scripts/gui/quick-capture-probe.sh` covers Inbox versus scheduled capture,
  focus restoration, and the panel's floating layer.
- `scripts/gui/forecast-probe.sh` covers the merged Forecast calendar.
- `scripts/gui/archive-probe.sh` checks Archive grouping, Reopen, Delete with
  its confirmation sheet, Undo, row opening and the empty state.
- `scripts/gui/sidebar-probe.sh` checks folded-on-launch folders, section
  headings, tag ordering by usage, the live-project folder badge, title-bar
  history, Settings visibility and count choices, and that those choices
  survive a relaunch.

Both build the release binary first (`cargo build --release`) and compile the
Swift helpers themselves.

Dropdown menus close on mouse-up, not mouse-down: closing on mouse-down
removes the menu before GPUI delivers the click to the item under the pointer.
Keep that ordering in mind when adding new overlays.
