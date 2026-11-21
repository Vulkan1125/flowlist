# Using the MCP server

Flowlist ships a [Model Context Protocol](https://modelcontextprotocol.io)
server. Point an AI assistant at it and the assistant can read and change the
same library you use in the window — no copy, no export, no plugin.

Everything runs locally over stdio. The assistant sees only the tools below;
the library never leaves the machine.

## Start it

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

Use an absolute path to the binary. The GUI, CLI and server can run at the same
time; writes are serialised, and each side reloads when the library changes.

## Tools

### Tasks

| Tool | What it does |
| --- | --- |
| `capture` / `add_task` | add a task from quick-entry text or explicit fields |
| `list_tasks` | list by view (`inbox`, `today`, `forecast`, `flagged`, `review`, …) or filter by project, tag, folder, due window, priority and search |
| `get_task` | fetch one task by id or unique prefix |
| `update_task` | change fields; omit to keep, send `null` to clear |
| `complete_task` | complete or reopen; repeating tasks spawn the next occurrence |
| `delete_task` | move a task and its subtasks to Trash; `series: true` takes a chain |
| `move_task` | move to the Inbox, a project or another task |
| `convert_task_to_project` | turn a task and its subtasks into a project |
| `set_task_tags` | add or remove tag paths |

### Checklist

| Tool | What it does |
| --- | --- |
| `list_checklist` | steps with ids and completion state |
| `add_checklist_item` | append a step, or insert at a position |
| `update_checklist_item` | rename and/or complete a step |
| `move_checklist_item` | reorder a step |
| `delete_checklist_item` | remove a step |

Checklist steps are not tasks and never appear as tasks in a list.

### Projects, tags and folders

| Tool | What it does |
| --- | --- |
| `list_projects` / `create_project` / `update_project` | projects, folders, dates, icons, review interval, attachments |
| `set_project_status` | active, on hold, done or dropped |
| `delete_project` | delete a project; its tasks move to Trash |
| `list_tags` / `create_tag` / `delete_tag` | nested tags |
| `list_folders` / `create_folder` | folders for projects |

### Review, library and history

| Tool | What it does |
| --- | --- |
| `review_list` | projects due for review |
| `complete_review` | mark reviewed and schedule the next one |
| `search` | tasks, projects, folders, tags and perspectives |
| `stats` | library counters |
| `undo` / `redo` | step through the shared history |
| `export_library` / `import_library` | JSON or TaskPaper; replace or merge |
| `library_info` | paths, revision, undo state and counters |

### Trash

| Tool | What it does |
| --- | --- |
| `list_trash` | tasks currently in Trash |
| `restore_tasks` | restore tasks and their subtrees |
| `purge_tasks` / `empty_trash` | delete forever |

`delete_task` is a soft delete. Trash follows the retention setting in the app
(**Settings → Trash**) and restoring is undoable too.

## Example conversations

Once the server is connected, ask in plain language:

- *Add “Finish the launch brief” to Website relaunch, due Friday, tagged writing.*
- *What is due today? Group it by project.*
- *Move the dentist task to next Monday and flag it.*
- *Mark “Get a review from Sam” done in the launch task.*
- *Which projects are due for review?*
- *Undo the last change.*
- *Export the library as TaskPaper to my Desktop.*

The assistant chooses the tools; you can ask it to show what it changed. Ids
may be full ULIDs, unambiguous prefixes or the short eight-character suffix.

## Safety

- Every tool call reads and writes the same local library as the GUI and CLI.
- Every mutation lands in the same undo history: `⌘Z` in the window or
  `flowlist undo` in a terminal reverses it.
- Invalid dates, unknown projects and ambiguous ids fail before anything is
  written.
- There is a read-only `flowlist://library/summary` resource with counts, the
  review queue and the library location.

## More

The full tool schemas are available from the server with the MCP `tools/list`
request, and in the
[MCP reference](https://github.com/Vulkan1125/flowlist/blob/main/docs/mcp.md)
in the repository.
