# Using the CLI

The `flowlist` command is the same app, without the window. It reads and writes
the same library as the GUI, so anything you do here shows up there, and the
same `⌘Z` / `flowlist undo` history covers both.

## Install the command

A macOS app bundle does not put its binary on `PATH` by itself. Open
**Settings → About → Command line** and choose **Install command-line tool**.
Flowlist creates a `flowlist` link in the first writable folder already on your
`PATH` (it prefers `~/.local/bin`), so this works without an admin password:

```sh
flowlist --version
```

If it had to use a folder outside `PATH`, the same card offers a one-click
`~/.zshrc` line and shows the line to copy. You can also install it from a
terminal once:

```sh
"/Applications/Flowlist.app/Contents/MacOS/flowlist" install-cli
```

After moving the app to another folder, run the installer again so the link
points at the new bundle. To remove just the command, use **Remove command** in
Settings or `flowlist uninstall-cli`; the app and library stay put.

## Uninstall Flowlist

Settings → About → **Danger zone → Uninstall Flowlist…** removes the app, the
`flowlist` command and macOS support files. Choose **Keep my library** to leave
tasks, history and backups in place, or **Delete the app and all data** to
remove the library as well. This cannot be undone.

From a terminal:

```sh
flowlist uninstall --keep-data            # remove app and command, keep library
flowlist uninstall --purge-data --yes     # remove everything
flowlist uninstall --purge-data --dry-run # print the paths first
```

Without `--yes` the command asks you to type `uninstall`, and refuses to run in
a non-interactive shell. A helper finishes the deletion after the app quits,
because a running process cannot delete its own bundle.

Every command accepts `--json` and `--data-dir <DIR>`. The library defaults to
`~/Library/Application Support/Flowlist`; set `FLOWLIST_HOME` to point the CLI
somewhere else:

```sh
export FLOWLIST_HOME="$HOME/Dropbox/Flowlist"
flowlist where
```

## Capture a task

`add` takes the same quick-entry line as the GUI:

```sh
flowlist add 'Draft the launch email #writing ::Website due:friday est:45m // ask Sam'
```

| Token | Meaning |
| --- | --- |
| `#writing`, `#work/website` | tag (nested tags are created) |
| `::Website`, `::Work/Website` | project (created if missing) |
| `due:friday`, `defer:tomorrow`, `planned:(next monday)`, `due:3d` | dates; compact offsets and parenthesised phrases |
| `fsr:1h30m`, `fsr:2d` | full-screen reminder time, or bare `fsr` to follow due |
| `est:45m`, `est:2h30m` | estimate |
| `repeat:every week` | repeat rule |
| `!` | flag |
| `// text` | note |

Prefer explicit flags? They work too:

```sh
flowlist add "Draft the launch email" \
  -p "Website" -t writing --due tomorrow --estimate 45m --note "ask Sam"
```

Useful options: `-p/--project`, `-t/--tag`, `--due`, `--defer`, `--planned`, `--estimate`,
`--repeat`, `--note`, `--priority`, `--flag`, `--parent` (make it a subtask)
and `--attach` (repeatable; a local path or `http(s)` link).

## See what is there

```sh
flowlist list                 # everything, grouped by project
flowlist list --inbox         # unscheduled tasks
flowlist list --today         # due today, deferred to today, overdue
flowlist list --forecast      # upcoming dated work
flowlist list --flagged
flowlist list --completed
flowlist list --trash
flowlist list -p "Website"    # one project
flowlist list -t writing      # one tag
flowlist list --available     # only tasks you can start now
flowlist list --due next7 --sort due
```

Filters combine: `flowlist list --today -t writing --priority high`. Group with
`--group project|folder|tag|due|defer|flagged|estimate` and order with
`--sort manual|due|defer|name|flagged|estimate|created|completed`.

Ids in the list output can be full ULIDs, a unique prefix, or the short
eight-character suffix. Any command that takes `<ID>` accepts all three.

## Change tasks

```sh
flowlist done <ID>                 # complete
flowlist done <ID> --undo          # reopen
flowlist edit <ID> --title "New title" --due none --flag
flowlist edit <ID> --project "Website" --priority high
flowlist mv <ID> --project "Q4 planning"
flowlist mv <ID> --parent <PARENT>
flowlist convert <ID>              # task + subtasks become a project
```

`edit` takes `--title`, `--notes`, `--due`, `--defer`, `--estimate`, `--repeat`,
`--project`, `--parent`, `--priority`, `--flag`/`--unflag`, `--sequential`,
`--attach` and `--clear-attachments`. Pass `none` to clear a date, estimate,
repeat rule, project or parent.

Deleting is a soft delete:

```sh
flowlist remove <ID>               # move to Trash
flowlist remove <ID> --series      # the whole repeating chain
flowlist trash --restore <ID>
flowlist trash --delete <ID>       # delete forever
flowlist trash --empty
```

## Checklist steps

```sh
flowlist checklist list <TASK>
flowlist checklist add <TASK> "Proofread the copy"
flowlist checklist set <TASK> <ITEM> --done
flowlist checklist set <TASK> <ITEM> --title "Proofread the final copy"
flowlist checklist move <TASK> <ITEM> 0
flowlist checklist remove <TASK> <ITEM>
```

Checklist steps are not tasks and never appear in `list` or a perspective.

## Projects, tags and folders

```sh
flowlist project add "Website relaunch" --icon rocket --review-days 7
flowlist project add "Q4 planning" --folder "Work" --kind sequential
flowlist project list
flowlist project status <ID> active|on-hold|done|dropped
flowlist project edit <ID> --name "Website" --review-days 14
flowlist project delete <ID>       # its tasks move to Trash

flowlist tag add writing
flowlist tag add work/website
flowlist tags

flowlist folder add Work
flowlist folder list
```

`project add` accepts `--kind parallel|sequential|single_actions`, `--icon`,
`--due`, `--defer`, `--note`, `--review-days` and `--attach`.

## Reviews

```sh
flowlist review                    # projects whose review date has arrived
flowlist review done <PROJECT>     # mark reviewed; schedule the next one
flowlist review done <PROJECT> --on 2026-12-01
```

## Search, stats and locations

```sh
flowlist search launch
flowlist stats
flowlist where
```

`search` covers tasks, projects, folders, tags and perspectives. `stats` prints
counts; `where` prints the library folder in use.

## Import, export and backup

```sh
flowlist export -o backup.taskpaper
flowlist export --format json -o backup.json
flowlist export --backup            # timestamped copy in the backups folder
flowlist import backup.json         # or TaskPaper text
flowlist import --merge backup.json # merge instead of replacing
```

## Undo anything

```sh
flowlist undo
flowlist redo
```

The GUI and CLI share one undo history. A task filed from the floating panel,
a terminal edit, a drag in the window and an MCP tool call all appear in the
same stack.

## JSON and scripts

Add `--json` to any command for machine-readable output:

```sh
flowlist list --today --json
flowlist add "Water the plants" --repeat weekly --json
flowlist stats --json
```

A small loop:

```sh
# Complete every overdue task that has the #errand tag.
flowlist list --due overdue -t errand --json \
  | jq -r '.groups[].tasks[].id' \
  | while read -r id; do flowlist done "$id"; done
```

## Open the other surfaces

```sh
flowlist gui     # the desktop window
flowlist mcp     # the MCP server on stdio
```
