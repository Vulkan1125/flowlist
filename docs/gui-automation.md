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

`modal` is `null` when no sheet is open, otherwise the sheet title (`New
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
| saved-perspective pencil | `(186, 558)` |

If the sidebar layout changes, update those two rows. The script fails loudly
rather than pretending the flow worked. When GPUI gains an accessibility API,
these clicks can be replaced by named actions without changing the assertions.

## Helpers

- `scripts/gui/flowctl.swift` posts mouse moves, left and right clicks and key
  events. Its modifiers include `ctrlalt`, used for the capture hotkey.
- `scripts/gui/window.swift` prints the live window title, origin and size.
  `window <pid> move <x> <y>` also activates the app and asks the window
  server to move it, which keeps absolute clicks reliable when macOS restores
  a window partly off screen.
- Both are compiled into the temporary directory when the smoke test runs.

Dropdown menus close on mouse-up, not mouse-down: closing on mouse-down
removes the menu before GPUI delivers the click to the item under the pointer.
Keep that ordering in mind when adding new overlays.
