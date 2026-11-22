# Flowlist CLI reference

The CLI is a thin, scriptable front end over the same command engine used by the
GUI and MCP server. Every mutation is undoable and every command accepts
`--json`.

```sh
flowlist [GLOBAL OPTIONS] <COMMAND> [OPTIONS]
```

Global options:

| Option | Meaning |
| --- | --- |
| `--data-dir <DIR>` | library directory (also `FLOWLIST_HOME`) |
| `--json` | machine-readable output |
| `-h, --help` | command help |
| `-V, --version` | version |

Run `flowlist <command> --help` for the exact flags of your build.

## Quick-entry syntax

`flowlist add` and the GUI capture bar share one parser.

```sh
flowlist add 'Draft launch email #Work/Website ::Work/Launch due:tomorrow est:45m ! // ask Sam'
```

| Token | Meaning |
| --- | --- |
| `#Tag`, `#Parent/Child` | attach a tag; missing tags are created |
| `::Project`, `::Folder/Project` | file into a project; missing containers are created |
| `due:`, `defer:` | natural-language dates (`due:(next friday 5pm)`) |
| `est:` / `estimate:` | duration (`15m`, `1h30m`) |
| `repeat:` | repeat rule (`every 2 weeks`, `every week after completion`) |
| `@1`, `@2`, `@3` | priority: high, medium, low |
| `!` | flag |
| `//` | the rest of the line is a note |

Wrap date and project phrases containing spaces in parentheses, for example
`due:(next monday 9am)` and `::(Work/Website Redesign)`.

## Tasks

### `add`

```sh
flowlist add [OPTIONS] [TEXT]...
```

Useful flags: `-p/--project`, `-t/--tag` (repeatable), `--due`, `--defer`,
`--estimate`, `--repeat`, `--note`, `--flag`, `--parent` and `--attach`
(repeatable; stores a reference to a local file path or an http(s) link).

### `list`

```sh
flowlist list [OPTIONS]
```

Choose a view or filter:

| Flag | Meaning |
| --- | --- |
| `--inbox` | inbox tasks |
| `--today` | due today, deferred to today, and overdue |
| `--flagged` | flagged tasks |
| `--forecast` | forecast buckets |
| `--review` | projects due for review |
| `--completed` | completed tasks |
| `--trash` | tasks currently in Trash |
| `--changed` | tasks edited within the Changed window |
| `--attachments` | open tasks that have attachments (files or links) |
| `--archive` | tasks in finished and dropped projects |
| `--available` | only available tasks |
| `--priority high\|medium\|low` | only tasks with that priority |
| `-t/--tag <PATH>` | tasks with a tag |
| `-p/--project <PATH>` | tasks in a project |
| `--folder <PATH>` | tasks in a folder tree |
| `-P/--perspective <NAME\|ID>` | a saved perspective |
| `-s/--search <TEXT>` | text filter |
| `--due <WINDOW>` | `today`, `overdue`, `next7`, or a date |
| `--sort <ORDER>` | `manual`, `due`, `defer`, `name`, `flagged`, `estimate`, `created`, `completed` |
| `--group <GROUP>` | `none`, `project`, `folder`, `tag`, `due`, `defer`, `flagged`, `estimate` |
| `--limit <N>` | maximum rows |
| `--include-completed` | include completed tasks |

### `done`, `remove`, `edit`

```sh
flowlist done <ID>... [--undo]
flowlist remove <ID>... [--series]   # move to Trash; --series takes the chain
flowlist edit <ID> [--title ...] [--notes ...] [--due ...] [--defer ...] \
  [--estimate ...] [--flag|--unflag] [--repeat ...] [--project ...] \
  [--parent ...] [--sequential] [--priority ...] \
  [--attach <PATH>]... [--clear-attachments]
```

Use `none` to clear dates, estimates, repeat rules, projects or parents.
Completing a repeating task increments the last number in its title, so
`Week 3 review` spawns `Week 4 review`; `--series` on `remove` moves every
occurrence of that chain to Trash instead of only the named one.

### Trash

Deleting a task moves it and its subtasks to Trash, where it stays until it is
restored, purged, or removed by the retention policy in Settings.

```sh
flowlist list --trash
flowlist trash --restore <ID>...
flowlist trash --delete <ID>...   # delete forever
flowlist trash --empty            # empty the whole bin
```

`trash_retention_days` in the library settings controls automatic emptying; `0`
keeps trashed tasks forever. Both restoring and purging are undoable until the
library is compacted.

### `mv`

```sh
flowlist mv <ID>... --project <PATH>
flowlist mv <ID>... --parent <TASK>
flowlist mv <ID>... --inbox
flowlist mv <ID>... --before <TASK> | --after <TASK> | --start
```

### `convert`

```sh
flowlist convert <TASK> [--folder <PATH>]
```

Turns a task and its top-level subtasks into a project as one undoable step.
The new project keeps the task's notes, tags, due/defer dates, attachments and
sequential setting; leftover checklist items become tasks so nothing is lost.
The command prints the created project.

### `undo` / `redo`

```sh
flowlist undo [--steps N]
flowlist redo [--steps N]
```

## Checklists

Checklist items are lightweight steps inside one task. They never appear as
tasks, have no project/dates/tags, and are copied to the next occurrence when a
repeating task is completed.

```sh
flowlist checklist list <TASK> [--hide-done]
flowlist checklist add <TASK> <TITLE>... [--before <INDEX>]
flowlist checklist set <TASK> <ITEM> [--title <TEXT>] [--done|--undone]
flowlist checklist move <TASK> <ITEM> --to <INDEX>
flowlist checklist remove <TASK> <ITEM>
```

Aliases: `check`, `ls`, `edit`, `mv`, `rm`.

`<TASK>` and `<ITEM>` accept a full ULID, an unambiguous prefix or the short
eight-character suffix. Human-readable `list` output shows progress as `☑ n/m`.

## Projects and folders

```sh
flowlist project add <NAME> [--folder <PATH>] [--note ...] [--due ...] \
  [--defer ...] [--review-days N] [--sequential] [--icon <ICON>]
flowlist project list [--closed]
flowlist project edit <ID> [--name ...] [--note ...] [--folder <PATH>|none] \
  [--due ...] [--defer ...] [--review-days N] [--next-review DATE] \
  [--sequential] [--icon <ICON>]
flowlist project status <ID> active|on-hold|done|dropped
flowlist project delete <ID>      # tasks move to Trash; undo restores both
flowlist project review <ID> [--on DATE]

flowlist folder add <PATH>
flowlist folder list
flowlist folder rename <ID> <NAME>
flowlist folder remove <ID>      # nested folders and projects go too; tasks move to Trash
```

Project icons: `project`, `folder`, `star`, `briefcase`, `home`, `heart`,
`book`, `code`, `rocket`, `coffee`. Like task attachments, project attachments
store a local file path or an http(s) link; files are never copied into the
library.

## Tags

Tags are contexts and can be nested.

```sh
flowlist tag add <PATH> [--parent <PATH>] [--status available|blocked|on-hold] \
  [--lat L] [--lon L]
flowlist tags
flowlist tag set <ID> [--name ...] [--parent <PATH>|none] \
  [--status available|blocked|on-hold] [--lat L] [--lon L] [--clear-location]
flowlist tag remove <ID>
```

## Perspectives

```sh
flowlist perspective add <NAME> [--filter '<JSON FILTER>'] \
  [--group-by none|project|folder|tag|due|defer|flagged|estimate] \
  [--sort <ORDER>] [--show-completed]
flowlist perspective list
flowlist perspective remove <ID>
```

The filter is a JSON `FilterNode` tree. A simple flagged filter:

```sh
flowlist perspective add 'Flagged work' \
  --filter '{"type":"condition","condition":{"type":"flagged","value":true}}'
```

## Review

```sh
flowlist review                 # list projects due for review
flowlist review done <ID> [--on YYYY-MM-DD]
```

Reviewing a project advances its next review date by its review interval.

## Search, stats and location

```sh
flowlist search <QUERY> [--limit N]
flowlist stats
flowlist where
```

`search` covers tasks (including checklist titles), projects, folders, tags and
perspectives. `stats` prints open/available/flagged/due counts plus project,
tag and folder totals.

## Output and automation

Every command accepts `--json` (before or after the subcommand) and prints one
JSON value. Ids are full ULIDs, and every object also carries the short
eight-character `short_id` shown in human output; both forms work wherever a
command takes an `<ID>`.

- `add` returns the created task.
- `list` returns `{ "groups": [...], "count": N }`, preserving the view's
  sections and task order.
- `stats` returns flat counters such as `open_tasks`, `available`,
  `projects_due_review` and `trashed_tasks`.
- Mutations return the affected entity or a small result object and are always
  recorded in the same undo history the GUI uses.

A script can therefore read a task, edit it, and undo the change without
touching the data file:

```sh
id=$(flowlist add 'Ship the release' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
flowlist edit "$id" --due tomorrow --priority high --json
flowlist done "$id" --json
flowlist undo --json
```

## Import, export and repair

```sh
flowlist export [--format taskpaper|json] [-o FILE] [--backup]
flowlist import <FILE> [--format auto|json|taskpaper] [--merge]
flowlist doctor [--fix]
```

- `export` defaults to TaskPaper on stdout.
- `--backup` writes a timestamped copy of `data.json` into `backups/`.
- `import` replaces the library unless `--merge` is passed.
- `doctor` reports dangling ids and order arrays; `--fix` repairs them.

JSON output from `export --format json` is a complete, portable library and can
be re-imported into any Flowlist installation.

## MCP server

```sh
flowlist mcp
```

Runs the Model Context Protocol server on stdin/stdout. See
[mcp.md](mcp.md).
