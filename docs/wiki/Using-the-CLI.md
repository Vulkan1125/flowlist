# Using the CLI

The `flowlist` command is the same app without the window. It reads and writes
the same library as the GUI, the phone and the MCP server, and shares the same
undo history.

This page is the complete command reference. Every command accepts
`--help`, `--json` and `--data-dir <DIR>` unless noted.

## Install the command

A macOS app bundle does not put its binary on `PATH` by itself.

1. Open **Settings → About → Command line**.
2. Choose **Install command-line tool**.
3. Confirm with:

```sh
flowlist --version
```

Flowlist links the executable into the first writable folder already on
`PATH` (it prefers `~/.local/bin`). If it had to use a folder outside `PATH`,
the same card offers a `~/.zshrc` line to copy.

From a terminal:

```sh
"/Applications/Flowlist.app/Contents/MacOS/flowlist" install-cli
```

After moving the app, run the installer again. To remove only the command:

```sh
flowlist uninstall-cli
```

## Global conventions

```text
flowlist [--data-dir <DIR>] [--json] <COMMAND>
```

| Option | Meaning |
| --- | --- |
| `--data-dir <DIR>` | Library directory. Defaults to the platform data directory. |
| `FLOWLIST_HOME` | Environment alternative to `--data-dir`. |
| `--json` | Machine-readable output where the command supports it. |
| `--version` | Print the version. |
| `--help` | Print help for the command or subcommand. |

Task ids may be full ULIDs, any unambiguous prefix, or the short
eight-character suffix shown in list output.

## Quick entry

`add` accepts the same line as the GUI:

```sh
flowlist add 'Draft the launch email #writing ::Website due:friday est:45m // ask Sam'
```

| Token | Meaning |
| --- | --- |
| `#writing`, `#work/website` | Tag. Nested tags are created. |
| `::Website`, `::Work/Website` | Project. Created if missing. |
| `due:friday`, `defer:tomorrow`, `planned:(next monday)`, `due:3d` | Dates. Compact offsets and parenthesised phrases work. |
| `est:45m`, `est:2h30m` | Estimate. |
| `repeat:every week` | Repeat rule. |
| `!` | Flag. |
| `// text` | Note. |
| `fsr`, `fsr:1h30m` | Full-screen reminder follows due, or fires at an offset. |

## `add`

```text
flowlist add [OPTIONS] [TEXT]...
```

| Option | Meaning |
| --- | --- |
| `-p, --project <PROJECT>` | File into a project or folder/project path. |
| `-t, --tag <TAGS>` | Attach a tag. Repeat the flag for several. |
| `--due <DUE>` | Due date or phrase. |
| `--defer <DEFER>` | Defer date. |
| `--planned <PLANNED>` | Planned date. Never changes availability. |
| `--estimate <ESTIMATE>` | `30m`, `1h30m`, and similar. |
| `--repeat <REPEAT>` | Repeat rule. |
| `--note <NOTE>` | Notes. |
| `--flag` | Flag the task. |
| `--priority <PRIORITY>` | `low`, `medium`, `high`. |
| `--parent <PARENT>` | Make this a subtask. |
| `--attach <ATTACHMENTS>` | File path or `http(s)` link. Repeatable. |
| `--alerts <ALERTS>` | Relative alerts such as `due:-15m, defer`. |
| `--remove-notification <KIND>` | Remove a default: `defer`, `planned`, `latest_start`, `due`. |
| `--custom-notification <WHEN>` | Absolute custom notification date/time. |

## `list`

```text
flowlist list [OPTIONS]
```

Views:

| Option | Meaning |
| --- | --- |
| `--inbox` | Inbox tasks. |
| `--today` | Overdue, due today, deferred to today and planned today. |
| `--forecast` | Upcoming dated work. |
| `--flagged` | Flagged tasks. |
| `--review` | Review queue. |
| `--completed` | Completed tasks. |
| `--trash` | Tasks in Trash. |
| `--changed` | Recently changed tasks. |
| `--attachments` | Tasks with file or link attachments. |
| `--fullscreen` | Tasks with an active full-screen reminder. |
| `--archive` | Finished and dropped projects. |
| `--priority <PRIORITY>` | `low`, `medium`, `high`. |
| `--available` | Only tasks available now. |

Filters:

| Option | Meaning |
| --- | --- |
| `-t, --tag <TAG>` | Tag path. |
| `-p, --project <PROJECT>` | Project path. |
| `--folder <FOLDER>` | Folder path. |
| `-P, --perspective <PERSPECTIVE>` | Saved or built-in perspective. |
| `-s, --search <SEARCH>` | Text search. |
| `--due <DUE>` | Window or date: `today`, `overdue`, `next7`, `2026-12-01`. |
| `--planned <PLANNED>` | Window or date. |
| `--limit <LIMIT>` | Maximum rows. |
| `--include-completed` | Include completed tasks. |

Ordering:

| Option | Values |
| --- | --- |
| `--sort <SORT>` | `manual`, `due`, `defer`, `planned`, `name`, `priority`, `flagged`, `estimate`, `created`, `completed`, `due_flagged`, `tag`, `added`, `changed`, `dropped`, `review`. |
| `--group <GROUP>` | `none`, `project`, `folder`, `tag`, `tags_combined`, `due`, `defer`, `planned`, `completed`, `dropped`, `added`, `changed`, `review`, `flagged`, `estimate`. |

Filters combine:

```sh
flowlist list --today -t writing --priority high --sort due
```

## `done`

```text
flowlist done [OPTIONS] <ID>...
```

| Option | Meaning |
| --- | --- |
| `--undo` | Reopen the task instead of completing it. |

Completing a repeating task creates its next occurrence.

## `edit`

```text
flowlist edit [OPTIONS] <ID>
```

| Option | Meaning |
| --- | --- |
| `--title <TITLE>` | New title. |
| `--notes <NOTES>` | New notes. |
| `--due <DUE>` | Date phrase, or `none` to clear. |
| `--defer <DEFER>` | Date phrase, or `none`. |
| `--planned <PLANNED>` | Date phrase, or `none`. |
| `--estimate <ESTIMATE>` | `30m`, `1h30m`, or `none`. |
| `--flag` / `--unflag` | Flag state. |
| `--repeat <REPEAT>` | Repeat rule, or `none`. |
| `-p, --project <PROJECT>` | Project path, or `inbox` to clear. |
| `--parent <PARENT>` | Parent task id, or `none` to clear. |
| `--sequential` | Work through children in order. |
| `--priority <PRIORITY>` | `low`, `medium`, `high`, or `none`. |
| `--attach <ATTACHMENTS>` | Add a file or link. Repeatable. |
| `--clear-attachments` | Remove every attachment. |
| `--alerts <ALERTS>` | Relative alerts; `none` clears them. |
| `--remove-notification <KIND>` | Remove a default notification. |
| `--restore-notification <KIND>` | Restore a removed default. |
| `--custom-notification <WHEN>` | Add an absolute notification. |
| `--clear-custom-notifications` | Remove every absolute notification. |

## `remove`

```text
flowlist remove [OPTIONS] <ID>...
```

| Option | Meaning |
| --- | --- |
| `--series` | Move the whole repeating series to Trash. |

## `trash`

```text
flowlist trash [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `--restore <ID>` | Restore these tasks and their subtrees. |
| `--delete <ID>` | Permanently delete these tasks and their subtrees. |
| `--empty` | Permanently delete everything in Trash. |

## `mv`

```text
flowlist mv [OPTIONS] <ID>...
```

| Option | Meaning |
| --- | --- |
| `-p, --project <PROJECT>` | Destination project. |
| `--folder <FOLDER>` | Destination folder. |
| `--parent <PARENT>` | Destination parent task. |
| `--inbox` | Move to the Inbox. |
| `--before <BEFORE>` | Place before this sibling. |
| `--after <AFTER>` | Place after this sibling. |
| `--start` | Place at the start. |

## `convert`

Turn a task and its subtasks into a project:

```text
flowlist convert [OPTIONS] <ID>
```

| Option | Meaning |
| --- | --- |
| `--folder <FOLDER>` | Folder for the new project. Defaults to the task’s folder. |

## `checklist`

**`flowlist checklist list <TASK> [--hide-done]`**

**`flowlist checklist add <TASK> <TITLE>... [--before <INDEX>]`**

**`flowlist checklist set <TASK> <ITEM> [--title <TITLE>] [--done] [--undone]`**

**`flowlist checklist move <TASK> <ITEM> --to <INDEX>`**

**`flowlist checklist remove <TASK> <ITEM>`**

Checklist items are lightweight steps. They never appear as tasks in a
perspective.

## `undo` and `redo`

```text
flowlist undo [--steps <N>]
flowlist redo [--steps <N>]
```

Both default to one step. The history is shared by every surface.

## `tag`

**`flowlist tag add <PATH>`**

| Option | Meaning |
| --- | --- |
| `--parent <PARENT>` | Parent tag path. |
| `--status <STATUS>` | `available`, `blocked`, `on-hold`. |
| `--lat <LAT>` / `--lon <LON>` | Optional location. |

**`flowlist tag set <ID>`**

| Option | Meaning |
| --- | --- |
| `--name <NAME>` | Rename. |
| `--parent <PARENT>` | Parent path, or `none`. |
| `--status <STATUS>` | `available`, `blocked`, `on-hold`. |
| `--lat <LAT>` / `--lon <LON>` | Location. |
| `--clear-location` | Remove coordinates. |

**`flowlist tag remove <ID>`**

**`flowlist tags`**

List tags with their paths and usage.

## `project`

**`flowlist project add <NAME>`**

| Option | Meaning |
| --- | --- |
| `--folder <FOLDER>` | Folder path. |
| `--note <NOTE>` | Notes. |
| `--due`, `--defer`, `--planned` | Project dates. |
| `--review-days <DAYS>` | Review interval. |
| `--sequential` | Sequential project. |
| `--kind <KIND>` | `parallel`, `sequential`, `single_actions`. |
| `--icon <ICON>` | Named icon or one emoji. |
| `--attach <ATTACHMENTS>` | File or link. Repeatable. |

**`flowlist project list [--closed]`**

**`flowlist project edit <ID>`**

Options: `--name`, `--note`, `--folder`, `--due`, `--defer`, `--planned`,
`--review-days`, `--next-review`, `--sequential`, `--kind`, `--icon`,
`--attach`, `--clear-attachments`, `--alerts`, `--remove-notification`,
`--restore-notification`, `--custom-notification`, `--clear-custom-notifications`.

**`flowlist project status <ID> <STATUS>`**

`active`, `on-hold`, `done`, `dropped`.

**`flowlist project review <ID> [--on <DATE>]`**

**`flowlist project delete <ID>`**

The project’s tasks move to Trash.

## `folder`

**`flowlist folder add <PATH>`**

**`flowlist folder list`**

**`flowlist folder rename <ID> <NAME>`**

**`flowlist folder remove <ID>`**

Nested folders and their projects are removed; tasks move to Trash.

## `perspective`

**`flowlist perspective add <NAME>`**

| Option | Meaning |
| --- | --- |
| `--filter <JSON>` | Filter tree as JSON. |
| `--group-by <GROUP>` | Grouping value. |
| `--sort <SORT>` | Sort value. |
| `--show-completed` | Include completed tasks. |
| `--pinned` | Keep above the saved section. |
| `--icon <ICON>` | Named icon or one emoji. |
| `--description <TEXT>` | One-line subtitle. |

**`flowlist perspective list`**

**`flowlist perspective remove <ID>`**

Filter JSON uses the shared tree shape:

```json
{
  "type": "all",
  "nodes": [
    { "type": "condition", "condition": { "type": "availability", "value": "available" } },
    { "type": "none", "nodes": [
      { "type": "condition", "condition": { "type": "tag", "id": "<tag-id>", "include_children": true } }
    ]}
  ]
}
```

## `review`

```text
flowlist review
flowlist review done <ID> [--on <DATE>]
```

`review` lists projects due for review. `review done` marks one reviewed and
schedules the next date.

## `search`

```text
flowlist search [OPTIONS] <QUERY>
```

| Option | Meaning |
| --- | --- |
| `--limit <LIMIT>` | Maximum hits. Default `25`. |

Searches tasks, projects, folders, tags and perspectives.

## `stats`

```text
flowlist stats
```

Counts open, available, flagged, due, overdue, projects and the review queue.

## `export`

```text
flowlist export [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `--format <FORMAT>` | `json` or `taskpaper`. Default `taskpaper`. |
| `-o, --output <FILE>` | Write to a file instead of stdout. |
| `--backup` | Write a timestamped copy of `data.json` into `backups/`. |

## `import`

```text
flowlist import [OPTIONS] <FILE>
```

| Option | Meaning |
| --- | --- |
| `--format <FORMAT>` | `auto`, `json` or `taskpaper`. |
| `--merge` | Merge into the current library instead of replacing it. |

## `doctor`

```text
flowlist doctor [--fix]
```

Checks dangling references and ordering arrays. `--fix` repairs them.

## `where`

```text
flowlist where
```

Prints the library location.

## `serve`

```text
flowlist serve [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `--port <PORT>` | Port. Default `7777`. |
| `--bind <BIND>` | Interface. Default `0.0.0.0`. |
| `--install-agent` | Install and start the login-time server. |
| `--uninstall-agent` | Stop and remove the login-time server. |
| `--status` | Print whether the server is running and the pairing code. |

## `mcp`

```text
flowlist mcp
```

Starts the MCP server on stdio. See [Using the MCP server](Using-the-MCP).

## `gui`

```text
flowlist gui
```

Opens the desktop window with the same library.

## `install-cli` and `uninstall-cli`

```text
flowlist install-cli
flowlist uninstall-cli
```

Link or unlink the command in a folder on `PATH`.

## `uninstall`

```text
flowlist uninstall [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `--keep-data` | Remove the app and command, keep the library. |
| `--purge-data` | Remove the library, backups and history too. |
| `-y, --yes` | Do not ask for confirmation. |
| `--dry-run` | Print what would be removed. |

Without `--yes` the command asks you to type `uninstall` and refuses to run in
a non-interactive shell.

## Scripting

Every command that returns data accepts `--json`:

```sh
flowlist list --today --json | jq '.groups[].tasks[].title'
flowlist stats --json
flowlist search "launch" --limit 5 --json
```

A failing command exits non-zero and prints a message on stderr. Unknown
projects, ambiguous ids and invalid dates fail before anything is written.

## Examples

```sh
# Capture into a project with dates and an estimate
flowlist add "Draft the homepage copy" -p "Website relaunch" --due tomorrow --estimate 45m

# Today, grouped by project
flowlist list --today --group project --sort due

# Complete a task, then undo it
flowlist done 01J8Z3K4
flowlist undo

# Move a task under another task
flowlist mv 01J8Z3K4 --parent 01J8Z3K5

# Review the weekly queue
flowlist review
flowlist review done <PROJECT-ID>
```
