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
`--estimate`, `--repeat`, `--note`, `--flag`, `--parent`.

### `list`

```sh
flowlist list [OPTIONS]
```

Choose a view or filter:

| Flag | Meaning |
| --- | --- |
| `--inbox` | inbox tasks |
| `--flagged` | flagged tasks |
| `--forecast` | forecast buckets |
| `--review` | projects due for review |
| `--completed` | completed tasks |
| `--available` | only available tasks |
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
flowlist remove <ID>...
flowlist edit <ID> [--title ...] [--notes ...] [--due ...] [--defer ...] \
  [--estimate ...] [--flag|--unflag] [--repeat ...] [--project ...] \
  [--parent ...] [--sequential]
```

Use `none` to clear dates, estimates, repeat rules, projects or parents.

### `mv`

```sh
flowlist mv <ID>... --project <PATH>
flowlist mv <ID>... --parent <TASK>
flowlist mv <ID>... --inbox
flowlist mv <ID>... --before <TASK> | --after <TASK> | --start
```

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
  [--defer ...] [--review-days N] [--sequential]
flowlist project list [--closed]
flowlist project edit <ID> [--name ...] [--note ...] [--folder <PATH>|none] \
  [--due ...] [--defer ...] [--review-days N] [--next-review DATE] [--sequential]
flowlist project status <ID> active|on-hold|done|dropped
flowlist project review <ID> [--on DATE]

flowlist folder add <PATH>
flowlist folder list
flowlist folder rename <ID> <NAME>
flowlist folder remove <ID>      # child projects move up one level
```

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
