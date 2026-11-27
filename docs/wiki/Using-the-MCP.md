# Using the MCP server

Flowlist ships a [Model Context Protocol](https://modelcontextprotocol.io)
server. Point an AI assistant at it and the assistant can read and change the
same library you use in the window — no copy, no export, no plugin.

Everything runs locally over stdio. The assistant sees only the tools below;
the library never leaves the machine.

## Start the server

```sh
/Applications/Flowlist.app/Contents/MacOS/flowlist mcp
```

The server speaks JSON-RPC on stdout and writes diagnostics to stderr. Leave it
running from your MCP client rather than typing into it by hand.

## Configure a client

Most clients take a JSON block in their MCP settings:

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

Add `env` if the library is not in the default folder:

```json
{
  "mcpServers": {
    "flowlist": {
      "command": "/Applications/Flowlist.app/Contents/MacOS/flowlist",
      "args": ["mcp"],
      "env": {
        "FLOWLIST_HOME": "/Users/you/Library/Application Support/Flowlist"
      }
    }
  }
}
```

Use an absolute path. The GUI, CLI and server can run at the same time; writes
are serialised, and each side reloads when the library changes.

## Quick start

Once connected, ask in plain language:

- *Add “Finish the launch brief” to Website relaunch, due Friday, tagged writing.*
- *What is due today? Group it by project.*
- *Move the dentist task to next Monday and flag it.*
- *Mark “Get a review from Sam” done in the launch task.*
- *Which projects are due for review?*
- *Undo the last change.*
- *Export the library as TaskPaper to my Desktop.*

The assistant chooses the tools. Ask it to show what it changed; every result
includes the affected ids.

## Tools

### Tasks

| Tool | What it does |
| --- | --- |
| `capture` / `add_task` | Add a task from quick-entry text or explicit fields. |
| `list_tasks` | List by a built-in view or custom perspective, or filter by project, tag, folder, due window, priority, search, availability and completion. |
| `get_task` | Fetch one task by id or unique prefix. |
| `update_task` | Change fields. Omit to keep; send `null` or `"none"` to clear a date, estimate, project or parent. |
| `complete_task` | Complete or reopen. Repeating tasks spawn the next occurrence. |
| `delete_task` | Move a task and its subtasks to Trash. `series: true` takes the whole repeating chain. |
| `move_task` | Move to the Inbox, a project or another task, with before/after placement. |
| `convert_task_to_project` | Turn a task and its top-level subtasks into a project. |
| `set_task_tags` | Add or remove tag paths. |

`update_task` also accepts notes, flag, priority, alerts, default notification
removals, custom notifications, repeat, sequential mode, attachments and
attachment clearing.

### List and filter values

`list_tasks` accepts:

| Field | Values |
| --- | --- |
| `view` | `all`, `inbox`, `today`, `projects`, `forecast`, `flagged`, `tags`, `completed`, `changed`, `attachments`, `nearby`, `review`, `trash`, `custom`. |
| `perspective` | Saved perspective name or id when `view` is `custom`. |
| `tags` | Tag paths; every listed tag must match. |
| `due` | `today`, `overdue`, `tomorrow`, `next7`, `next30`, `none`, or a date. |
| `priority` | `low`, `medium`, `high`. |
| `sort` | `manual`, `due`, `defer`, `planned`, `name`, `priority`, `flagged`, `estimate`, `created`, `completed`, `due_flagged`, `tag`, `added`, `changed`, `dropped`, `review`. |
| `group` | `none`, `project`, `folder`, `tag`, `tags_combined`, `due`, `defer`, `planned`, `completed`, `dropped`, `added`, `changed`, `review`, `flagged`, `estimate`. |

`available`, `flagged`, `inbox` and `completed` are booleans. `show_completed`
and `limit` control completed rows and the maximum result count.

### Checklist

| Tool | What it does |
| --- | --- |
| `list_checklist` | Steps with ids and completion state. |
| `add_checklist_item` | Append a step, or insert before an index. |
| `update_checklist_item` | Rename and/or complete a step. |
| `move_checklist_item` | Reorder a step. |
| `delete_checklist_item` | Remove a step. |

Checklist steps are not tasks and never appear as tasks in a list.

### Projects

| Tool | What it does |
| --- | --- |
| `list_projects` | Projects in library order; `include_closed` adds Done, Dropped and archived projects. |
| `create_project` | Create a project, optionally inside a folder path. |
| `update_project` | Name, notes, folder, dates, review interval, kind, completion mode, icon, attachments, notifications and `archived`. |
| `set_project_status` | `active`, `on-hold`, `done` or `dropped`. |
| `archive_projects` | Archive or unarchive several projects in one undo step. |
| `archive_folder` | Archive or unarchive every project inside a folder. |
| `delete_project` | Delete a project; its tasks move to Trash. |

### Tags and folders

| Tool | What it does |
| --- | --- |
| `list_tags` | Tags with usage counts. |
| `create_tag` | Create a nested tag, with status and optional coordinates. |
| `delete_tag` | Delete a tag and remove it from tasks. |
| `list_folders` | Folders and their project counts. |
| `create_folder` | Create a folder, including nested paths. |

### Review, search and history

| Tool | What it does |
| --- | --- |
| `review_list` | Projects due for review. |
| `complete_review` | Mark reviewed and schedule the next date. |
| `search` | Tasks, projects, folders, tags and perspectives. |
| `stats` | Open, available, flagged, due, overdue, projects and review counts. |
| `undo` / `redo` | Step through the shared history. |
| `export_library` | Export JSON or TaskPaper. |
| `import_library` | Import JSON or TaskPaper, replacing or merging. |
| `library_info` | Paths, schema revision, undo state, counters and built-in views. |

### Trash

| Tool | What it does |
| --- | --- |
| `list_trash` | Tasks currently in Trash. |
| `restore_tasks` | Restore tasks and their subtrees. |
| `purge_tasks` | Permanently delete selected tasks. |
| `empty_trash` | Permanently delete everything in Trash. |

`delete_task` is a soft delete. Trash follows **Settings → Trash**, and
restoring is undoable too.

## Resources

The server exposes a read-only resource:

```text
flowlist://library/summary
```

It returns counts, the review queue and the library location.

## Tool schemas

The full input schemas are available from the server with the MCP `tools/list`
request. The source reference lives in
[docs/mcp.md](https://github.com/Vulkan1125/flowlist/blob/main/docs/mcp.md).

## Safety

- Every tool reads and writes the same local library as the GUI and CLI.
- Every mutation lands in the same undo history.
- Invalid dates, unknown projects and ambiguous ids fail before anything is
  written.
- The server binds to stdio. It does not open a network port.
