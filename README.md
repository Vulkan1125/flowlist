# Flowlist

**A fast, local-first task manager for macOS and Android.** Inbox, projects,
tags, repeats, reviews and a floating capture panel in one native window. A
phone app uses the same library, and the `flowlist` CLI and MCP server expose it
to scripts and AI assistants.

**[Download Flowlist 0.9.21](https://github.com/Vulkan1125/flowlist/releases/latest)**
· [Wiki](https://github.com/Vulkan1125/flowlist/wiki)
· [Using the GUI](https://github.com/Vulkan1125/flowlist/wiki/Using-the-GUI)
· [CLI](https://github.com/Vulkan1125/flowlist/wiki/Using-the-CLI)
· [MCP](https://github.com/Vulkan1125/flowlist/wiki/Using-the-MCP)

![The Flowlist Inbox on a colourful desktop](docs/images/hero.png)

*Pictured in Catppuccin. Plain is the default; all eleven theme families are in
Settings → Appearance.*

## Install

1. Download **Flowlist-0.9.21.dmg** (or the zip) from the latest release.
2. Open the DMG and drag **Flowlist** to Applications, or unzip and move the app
   yourself.
3. The first time, right-click the app and choose **Open**. The build is
   ad-hoc signed and not notarised, so macOS asks once.
4. In Flowlist, open **Settings → About → Command line → Install
   command-line tool**. Then `flowlist` works in Terminal. You can also run
   `/Applications/Flowlist.app/Contents/MacOS/flowlist install-cli` once.
5. Requires macOS 13 or later.

Your library lives in `~/Library/Application Support/Flowlist`. Nothing is
uploaded; there is no account, no sync service and no telemetry.

## What you get

- **One window for the day.** Inbox, Today, Forecast (with a month calendar),
  Review, Projects, Tags, Flagged, Completed, Archive, Trash and your saved
  perspectives.
- **Capture without breaking flow.** `⌃⌥Space` opens a floating parser anywhere:
  `Draft the launch email ::Website #writing due:friday est:45m` becomes a filed
  task, then returns you to the app you were in.
- **Real projects.** Parallel, sequential and single-action projects, folders,
  nested tags, subtasks, repeats that spawn the next occurrence, and review
  schedules you can change later.
- **An inspector that earns its space.** Notes, priority, checklist, subtasks,
  attachments, dates and repeat rules in place. Images and PDFs preview in the
  window instead of opening another app.
- **Glass and colour.** Eleven theme families, Plain light/dark with nine
  accent presets, a custom colour picker and a screen eyedropper.
- **Keyboard-first, undo everywhere.** Vim motions are on by default; every
  edit, drag, import and tool call is one `⌘Z` away.
- **Updates in the app.** Settings → About checks the latest release and
  installs it in place.
- **Due reminders.** Settings → Workflow schedules macOS notifications for
  upcoming due dates. macOS only grants notification permission to a signed
  build; the ad-hoc download explains this in place, and packaging with an
  Apple Development or Developer ID identity enables it.

## Three ways to use it

| Surface | Start it with | Good for |
| --- | --- | --- |
| Desktop app | `open /Applications/Flowlist.app` | planning, dragging, reviewing |
| Android app | install `Flowlist-Android-0.9.21.apk` | capture and checking off on the go |
| CLI | `…/Flowlist.app/Contents/MacOS/flowlist` | scripts and terminal habits |
| MCP server | `…/flowlist mcp` | letting an AI assistant read and change the library |

All four use the same library and the same undo history.

## Android

The phone is a client of the Mac library, not a second one. Nothing syncs
through a cloud: the app talks to a small server inside Flowlist, on your Wi-Fi
or over Tailscale.

1. Install `Flowlist-Android-0.9.21.apk` from the release. Android will ask you
   to allow installs from your browser or file manager.
2. On the Mac, open **Settings → Mobile** and turn on **Serve to my phone**. The
   login agent keeps it running after you close the window.
3. On the phone, enter one of the addresses shown there and the six-digit
   pairing code.

Captures, completions and deletes appear on the Mac immediately. If the Mac is
asleep or you are out of range, the phone shows a **Waiting** mark and syncs
when it can. Everything works offline for reading, capturing and completing.

The app covers Inbox, Today, Forecast, Browse, capture with the same
`due:`/`#tag`/`::Project` syntax, task detail, complete, trash and undo.
Editing every field on the phone, a home-screen widget and due notifications are
next.

## Capture

The floating panel opens from any app with **`⌃⌥Space`**, or from the title bar
inside Flowlist. It parses as you type and shows where the task will land.

| Syntax | Meaning |
| --- | --- |
| `#writing`, `#work/website` | tag, created if missing |
| `::Website`, `::Work/Website` | project, created if missing |
| `due:friday`, `defer:tomorrow` | dates; wrap phrases with spaces in parentheses |
| `est:45m`, `est:2h30m` | estimate |
| `repeat:every week` | repeat rule |
| `!` | flag |
| `// ask Sam` | note |

Press `Enter` to file and return to the previous app, `⇧Enter` to keep
capturing, or `⌘Enter` to file and select the task in the main window.

## Quick start in the terminal

After **Settings → About → Install command-line tool**:

```sh
flowlist add 'Draft the launch email #writing ::Website due:friday est:45m'
flowlist list --today
flowlist done <ID>
flowlist review
flowlist undo
```

The installer puts a link in the first writable folder on your `PATH` and
prefers `~/.local/bin`. If that folder is not on `PATH`, the sheet offers a
one-click `~/.zshrc` line. **Remove command** takes the link away again without
touching the app or your library. You can always use the full path instead:
`/Applications/Flowlist.app/Contents/MacOS/flowlist`.

### Uninstall

**Settings → About → Danger zone → Uninstall Flowlist…** removes the app and
the `flowlist` command. The sheet asks whether your library stays at
`~/Library/Application Support/Flowlist` or is deleted with everything else.
The CLI does the same and prints what it will remove first:

```sh
flowlist uninstall --keep-data           # or --purge-data, add --dry-run
```

Add `--json` to any command for scripts. `FLOWLIST_HOME` or `--data-dir` points
the CLI at another library. The full command reference is in the
[CLI guide](https://github.com/Vulkan1125/flowlist/wiki/Using-the-CLI).

## Let an assistant use it

Run `flowlist mcp` from an MCP client and it gets tasks, projects, tags,
checklists, reviews, search, stats and undo over stdio — the same store as the
window.

```json
{
  "mcpServers": {
    "flowlist": {
      "command": "/Applications/Flowlist.app/Contents/MacOS/flowlist",
      "args": ["mcp"]
    }
  }
}
```

Then ask things like *“Add a task to Website relaunch, due Friday”* or *“What’s
due today?”*. See the
[MCP guide](https://github.com/Vulkan1125/flowlist/wiki/Using-the-MCP) for the
tool list and examples.

## Keyboard

| Shortcut | Action |
| --- | --- |
| `⌃⌥Space` | capture from any app |
| `n` / `⌘N` | capture |
| `/` or `⌘F` | search |
| `⌘⇧F` | search this view or the whole library |
| `space` | complete or reopen |
| `!` / `⌘D` | flag |
| `⏎` | edit the title |
| `Tab` / `⇧Tab` | nest / unnest |
| `⌘I` | toggle the inspector |
| `⌘Z` / `⇧⌘Z` | undo / redo |
| `⌘,` | settings |
| `?` | full keyboard help |

## Documentation

- [Wiki home](https://github.com/Vulkan1125/flowlist/wiki)
- [Using the GUI](https://github.com/Vulkan1125/flowlist/wiki/Using-the-GUI)
- [Using the CLI](https://github.com/Vulkan1125/flowlist/wiki/Using-the-CLI)
- [Using the MCP server](https://github.com/Vulkan1125/flowlist/wiki/Using-the-MCP)
- `docs/` in this repository for design notes, packaging and testing

## Build from source

Requires Rust 1.88+ and macOS 13+.

```sh
cargo build --release
./target/release/flowlist gui
```

`scripts/package.sh` builds `dist/Flowlist.app` and the release zip used by the
in-app updater. `cargo test --workspace` runs the suite.

## License

MIT
