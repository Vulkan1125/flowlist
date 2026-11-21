# Flowlist

Flowlist is a local-first task manager written in Rust. A native
[GPUI](https://gpui.rs) desktop app, a scriptable CLI and a Model Context Protocol
server all read and write the same JSON library. Nothing leaves your machine.

```
crates/
  flowlist-core   domain model, persistence, commands, undo, queries, import/export
  flowlist        GPUI app, CLI and MCP server (one binary)
```

- Pure Rust, no web view, no Tauri/Electron.
- Official `gpui = 0.2.2` with `runtime_shaders` (no Metal toolchain step).
- JSON persistence with atomic writes, backups and cross-process locking.
- Every mutation goes through one command engine, so GUI, CLI and MCP all share
  validation, repeat rules and undo/redo.

## Build and run

Requires Rust 1.88 or newer and a macOS/Linux toolchain supported by GPUI.

```sh
cargo build --release
./target/release/flowlist gui      # desktop app
./target/release/flowlist --help   # CLI
./target/release/flowlist mcp      # MCP server on stdio
```

The CLI and MCP server can be built without GPUI:

```sh
cargo build --release --no-default-features
```

Point every front end at the same library with `FLOWLIST_HOME` or `--data-dir`:

```sh
export FLOWLIST_HOME="$HOME/Library/Application Support/Flowlist"
./target/release/flowlist where
```

The default location is `~/Library/Application Support/Flowlist` on macOS,
`%APPDATA%\Flowlist` on Windows and `$XDG_DATA_HOME/flowlist` (or
`~/.local/share/flowlist`) on other Unix systems.

## Quick start

```sh
flowlist add 'Draft launch email #Work/Website ::Work/Launch due:tomorrow est:45m !'
flowlist list --forecast
flowlist checklist add <task> 'Proofread the copy'
flowlist checklist list <task>
flowlist done <task>
flowlist undo
```

Quick entry understands:

| Syntax | Meaning |
| --- | --- |
| `#Tag` / `#Parent/Child` | attach a tag, creating missing levels |
| `::Project` / `::Folder/Project` | file into a project, creating it if needed |
| `due:tomorrow 5pm`, `defer:next monday` | natural-language date; parenthesize phrases with spaces |
| `est:1h30m` | estimate in minutes |
| `repeat:every 2 weeks` | repeat rule (`after completion`, `(defer another)` also work) |
| `!` | flag the task |
| `// note text` | everything after `//` becomes a note |

Short ids (the last eight characters of a ULID) are accepted anywhere an id is
expected. An unambiguous prefix also works.

## Desktop app

The window hides the native macOS titlebar and its traffic lights. A custom
glass header shows the app name, the search field, a light/dark toggle and a
labelled **Add task** button, with a persistent sidebar and an optional
inspector around the task list.

Key behaviour:

- Perspectives: Inbox (unscheduled work), Projects, Forecast, Flagged,
  Review, Tags, Completed, Nearby and custom saved perspectives.
- Projects and folders form a tree; tags form a nested context tree.
- Task rows support multi-select, drag/drop into projects, tags, parents and
  sibling positions, inline completion circles and flag toggles.
- The inspector edits title, notes, tags, project, defer, due, estimate, repeat
  rules and checklist steps in place.
- Checklist items have their own completion state and progress badge; they are
  not tasks and never appear in perspectives.
- Saved perspectives open a full editor for their conditions, grouping and
  sort; conditions the editor does not expose survive editing unchanged.
- Tags can carry a latitude and longitude; **Nearby** ranks tasks tagged with
  a located context by distance.
- Import JSON or TaskPaper, export TaskPaper with `⇧⌘E`, and undo everything
  with `⌘Z`.
- Session state (view, selection, inspector, sort, grouping, theme family and
  light/dark mode, vim mode and window size) is persisted in `session.json`.

Frequently used shortcuts:

| Shortcut | Action |
| --- | --- |
| `n` | quick capture |
| `⌃⌥Space` | capture from any app in a floating panel; the main window stays put |
| `/` or `⌘F` | search |
| `⌘⇧F` | search this view or the whole library |
| `↑` / `↓`, `⇧↑` / `⇧↓` | move / extend selection |
| `space` | complete or reopen |
| `!` or `⌘D` | flag / unflag |
| `⏎` | edit title |
| `Tab` / `⇧Tab` | nest / unnest |
| `za` / `zo` / `zc` | toggle / open / close subtasks |
| `⌥↑` / `⌥↓` | move within siblings |
| `i` or `⌘I` | toggle inspector |
| `⌘L` | add a checklist step |
| `⌘R` | rename current project, folder, tag or perspective |
| `⌘Z` / `⇧⌘Z` | undo / redo |
| `⌘1…⌘8` | built-in perspectives |
| `⌘N` | quick capture |
| `⇧⌘N` / `⌥⌘N` / `⌃⌘N` | new project / folder / tag |
| `⇧⌘P` | new saved perspective |
| `⌘T` | toggle light / dark |
| `⇧⌘T` / `⌥⌘T` | next / previous theme family |
| `⌘⌫` | delete the selection, or the sidebar entity |
| `⌘,` | settings |
| `⌘O` | import |
| `?` | keyboard and mouse help |

### Vim mode

Vim mode is on by default and covers the whole window: the task list, the
sidebar and the checklist pane. It can be turned off in **Settings**
(`⌘,`). macOS shortcuts keep working at the same time.

| Keys | Action |
| --- | --- |
| `j` / `k` | move down / up; a count multiplies (`3j`) |
| `h` / `l` | outdent / indent a task; collapse / expand a folder or tag |
| `gg` / `G` | jump to top / bottom; `5G` jumps to line 5 |
| `Ctrl-d` / `Ctrl-u` | half page down / up |
| `o` / `O` | new task below / above |
| `x` | complete or reopen |
| `!` | flag or unflag |
| `dd` / `yy` / `p` | delete / yank / paste |
| `u` / `Ctrl-r` | undo / redo |
| `r` / `e` | rename / edit the selected title |
| `⇧L` | set a tag or library location |
| `⇧J` / `⇧K` | move a task down / up among its siblings |
| `Ctrl-w h` | focus the sidebar |
| `Ctrl-w l` | focus the task list |
| `Ctrl-w j` | focus the checklist pane |
| `/`, `n`, `N` | search, next match, previous match |
| `?` | help |
| `Esc` | clear counts, selection and search |

In vim mode unmodified digits build a count, so perspectives move to
`⌘1…⌘8`.

## CLI

The CLI is designed for scripts and shell pipelines. Every command supports
`--json` and the same undo history as the GUI.

```sh
flowlist add 'Call the bank' --due tomorrow --flag
flowlist list --inbox --group project
flowlist list --project 'Work/Launch' --include-completed
flowlist edit <id> --due 'next friday 5pm' --repeat 'every week'
flowlist mv <id> --project 'Work/Launch' --before <other-id>
flowlist checklist add <task> 'Book the room' --before 0
flowlist checklist set <task> <item> --done
flowlist checklist move <task> <item> --to 2
flowlist checklist remove <task> <item>
flowlist tag add 'Places/Cafe'
flowlist project status <project> on-hold
flowlist folder rename <folder> 'Archive'
flowlist perspective add 'Deep work' --group project --sort due
flowlist export --format json -o backup.json
flowlist import --merge backup.json
flowlist doctor --fix
```

Full command reference: [docs/cli.md](docs/cli.md).

## MCP server

`flowlist mcp` speaks JSON-RPC over stdio. It supports both the modern
`2026-07-28` lifecycle (`server/discover`, `_meta`, `resultType`) and the legacy
`initialize` handshake, so current and older clients can connect.

```json
{
  "mcpServers": {
    "flowlist": {
      "command": "/absolute/path/to/flowlist",
      "args": ["mcp"]
    }
  }
}
```

The server exposes task, project, tag, folder, checklist, review, search,
import/export and undo tools. See [docs/mcp.md](docs/mcp.md) for the tool list
and examples.

## Packaging

Build a double-clickable app bundle:

```sh
scripts/package.sh        # dist/Flowlist.app
open dist/Flowlist.app
open dist/Flowlist.app --args --data-dir /path/to/library
```

The bundle contains the same binary as the CLI and MCP server, so
`dist/Flowlist.app/Contents/MacOS/flowlist list --json` works too. The script
ad-hoc signs the bundle; see [docs/packaging.md](docs/packaging.md) for
Gatekeeper and notarization notes.

## Data and safety

A library directory contains:

| File | Purpose |
| --- | --- |
| `data.json` | the whole normalized library |
| `data.json.bak` | previous good revision |
| `history.json`, `history.json.bak` | undo/redo snapshots |
| `session.json` | GUI session state |
| `data.lock` | cross-process write lock |
| `backups/` | timestamped copies created by `flowlist export --backup` |

Writes are atomic (write, fsync, rename). A revision number lets every process
reload when another process writes. Schema migrations run on open, and
`flowlist doctor` reports or repairs dangling references.

## Documentation

- [CLI reference](docs/cli.md)
- [MCP tools](docs/mcp.md)
- [GUI guide](docs/gui.md)
- [Architecture](docs/architecture.md)
- [Feature checklist](docs/features.md)
- [Checklists](docs/checklists.md)
- [Packaging](docs/packaging.md)
- [Performance](docs/performance.md)
- [Accessibility](docs/accessibility.md)
- [GUI smoke test](docs/gui-automation.md)
- [Backlog](docs/backlog.md)

The GitHub wiki is generated from these pages. After creating the first wiki
page in the GitHub web UI, run `scripts/publish-wiki.sh` to publish the full
tree.

## Development

```sh
cargo test --workspace
cargo check --workspace
cargo build -p flowlist --no-default-features   # headless CLI/MCP build
scripts/gui-smoke.sh                            # mouse-only GUI flows (macOS)
scripts/gui/forecast-probe.sh                   # Forecast calendar probe (macOS)
```
