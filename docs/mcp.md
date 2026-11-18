# Flowlist MCP server

`flowlist mcp` is a hand-rolled Model Context Protocol server over stdio. It
uses the same store, validation and undo history as the GUI and CLI.

```sh
flowlist mcp
```

- stdout carries one JSON-RPC line per message; diagnostics go to stderr.
- Modern lifecycle `2026-07-28` is supported (`server/discover`, `_meta`,
  `resultType: "complete"`).
- Legacy clients that send `initialize` and `tools/list` also work.
- Tool results include both human-readable `content` and `structuredContent`.

## Client configuration

Use an absolute path to the binary so the client does not depend on your shell
`PATH`.

```json
{
  "mcpServers": {
    "flowlist": {
      "command": "/Users/you/bin/flowlist",
      "args": ["mcp"],
      "env": {
        "FLOWLIST_HOME": "/Users/you/Library/Application Support/Flowlist"
      }
    }
  }
}
```

If `FLOWLIST_HOME` is omitted, the server uses the platform default library. The
GUI, CLI and MCP server can run at the same time; writes are serialized with a
file lock and readers reload when the revision changes.

## Tools

### Tasks

Attachment fields are local file paths or http(s) links on the task;
`convert_task_to_project` turns a
task and its top-level subtasks into a project in one undoable step, moving the
task's attachments to the project and turning leftover checklist items into
tasks. `create_project` and `update_project` accept the same `attachments` and
`clear_attachments` fields.

| Tool | Purpose |
| --- | --- |
| `capture` | add a task using quick-entry text or explicit fields |
| `add_task` | alias of `capture` |
| `get_task` | fetch one task by id or unique prefix |
| `update_task` | change fields; omit to keep, send `null`/`"none"` to clear |
| `complete_task` | complete or reopen; repeating tasks spawn the next occurrence |
| `delete_task` | delete a task and its subtasks; `series: true` takes the whole repeating chain |
| `move_task` | move to the inbox, a project or another task |
| `convert_task_to_project` | turn a task and its top-level subtasks into a project |
| `set_task_tags` | add or remove tag paths |
| `list_tasks` | list by built-in view or filter |

`list_tasks` accepts `view` (`inbox`, `today`, `projects`, `forecast`, `flagged`,
`tags`, `completed`, `archive`, `changed`, `attachments`, `nearby`, `review`,
`trash` or `custom` with `perspective`), plus `project`, `folder`, `tags`
(an array; all must match), `search`, `available`, `flagged`, `inbox`,
`completed`, `priority`, `due`, `sort`, `group`, `show_completed` and `limit`.
`view=review` returns the review queue as projects. A built-in view without
`limit` returns every matching task, grouped the way the GUI shows it.

### Trash

| Tool | Purpose |
| --- | --- |
| `list_trash` | list tasks currently in Trash |
| `restore_tasks` | restore tasks and their subtrees |
| `purge_tasks` | permanently delete tasks from Trash |
| `empty_trash` | permanently delete every trashed task |

Completing a repeating task increments the last number in its title and
spawns the next occurrence with the same series id. Passing `series: true` to
`delete_task` moves every occurrence in that chain to Trash in one undoable
step.

`delete_task` is a soft delete. Trash follows the library's
`trash_retention_days` setting; restoring or purging is recorded in undo history
just like the GUI buttons and `flowlist trash`.

### Checklist items

| Tool | Purpose |
| --- | --- |
| `list_checklist` | list steps with ids and completion state |
| `add_checklist_item` | append a step, or insert at `position` |
| `update_checklist_item` | rename and/or complete a step |
| `delete_checklist_item` | remove a step |
| `move_checklist_item` | move a step to a zero-based index |

Checklist items are not tasks. They never appear in `list_tasks` or
perspectives, and a repeating task copies its checklist with every step
unchecked.

### Projects, tags, folders

| Tool | Purpose |
| --- | --- |
| `list_projects` | projects in library order |
| `create_project` | create a project, optionally under a folder path |
| `update_project` | rename, move, date, review interval or notes |
| `set_project_status` | `active`, `on-hold`, `done`, `dropped` |
| `delete_project` | delete a project and move its tasks to Trash |
| `list_tags` | tags with usage counts |
| `create_tag` | create a nested tag path |
| `delete_tag` | delete a tag and remove it from tasks |
| `list_folders` | folders with project counts |
| `create_folder` | create a nested folder path |

### Review, library and history

| Tool | Purpose |
| --- | --- |
| `review_list` | projects due for review |
| `complete_review` | mark reviewed and schedule the next review |
| `search` | tasks, projects, folders, tags and perspectives |
| `stats` | library counters |
| `undo` | undo the most recent change(s) |
| `redo` | redo undone change(s) |
| `export_library` | export JSON or TaskPaper |
| `import_library` | replace or merge a library |
| `library_info` | paths, revision, undo state, counters, built-ins |

## Examples

Add a task and read it back:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
  "name":"capture",
  "arguments":{"title":"Draft launch email","project":"Work/Launch","tags":["Work/Website"],"due":"tomorrow","estimate":"45m","flagged":true}
}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
  "name":"list_tasks",
  "arguments":{"view":"forecast","limit":20}
}}
```

Add and complete a checklist step:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
  "name":"add_checklist_item",
  "arguments":{"id":"01M...","title":"Proofread the copy"}
}}
```

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
  "name":"update_checklist_item",
  "arguments":{"id":"01M...","item":"7F3P2K9Q","completed":true}
}}
```

Ids may be full ULIDs, unambiguous prefixes or the short eight-character suffix
shown in human output.

## Resources

`resources/list` advertises `flowlist://library/summary`, a read-only JSON
summary of counts, review queue and library location. `resources/read` returns
it as text.

## Error handling

Tool failures return a JSON-RPC error or an `isError: true` result with a
message. Ambiguous ids, empty titles, unknown projects and invalid dates fail
before anything is written. Successful mutations are added to the same undo
history visible in the GUI (`⌘Z`) and CLI (`flowlist undo`).
