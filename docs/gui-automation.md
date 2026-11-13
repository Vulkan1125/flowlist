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

Both flows use a temporary library, so the test never touches the real one.

## The test-state hook

Setting `FLOWLIST_TEST_STATE=/path/to/state.json` makes the window write:

```json
{
  "view": "custom:01M79...",
  "modal": "Perspective",
  "selected": 0,
  "window": [1254.0, 1404.0],
  "theme": "catppuccin",
  "appearance": "light",
  "blur": "light",
  "traffic_lights": "hidden",
  "modal_theme": null
}
```

`modal` is `null` when no sheet is open, otherwise the sheet title (`New
project`, `Perspective`, `Settings`, …). `traffic_lights` reports whether the
AppKit window buttons were hidden. The file is rewritten only when one of
those values changes, and the hook is inert unless the variable is set. It does
not contain task text, notes or the library itself.

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
| saved-perspective pencil | `(190, 384)` |

If the sidebar layout changes, update those two rows. The script fails loudly
rather than pretending the flow worked. When GPUI gains an accessibility API,
these clicks can be replaced by named actions without changing the assertions.

## Helpers

- `scripts/gui/flowctl.swift` posts mouse moves, clicks and key events.
- `scripts/gui/window.swift` prints the live window title, origin and size.
- Both are compiled into the temporary directory when the smoke test runs.
