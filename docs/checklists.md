# Checklists

Checklist items are lightweight steps inside one task: "pack passport", "book
the room", "proofread the copy". They are deliberately **not** tasks.

| | Task | Checklist item |
| --- | --- | --- |
| Appears in perspectives | yes | no |
| Project / folder | yes | no |
| Dates, estimate, tags | yes | no |
| Nested children | yes | no |
| Own completion state | yes | yes |
| Undoable | yes | yes |
| Copied by repeat rules | yes | yes, unchecked |

## Data shape

```json
{
  "id": "01M...",
  "title": "Prepare the talk",
  "checklist": [
    { "id": "01M...", "title": "Outline slides", "completed": true },
    { "id": "01M...", "title": "Practice run", "completed": false }
  ]
}
```

Tasks expose `checklist`, `checklist_completed` and `checklist_total` in CLI/MCP
JSON. The task row in the GUI shows a `☑ n/m` badge; the inspector shows the
steps with their own completion circles.

## Ordering

Items are ordered by their position in `Task::checklist`. Moving an item removes
it and reinserts it at the target index. Positions are zero-based in the CLI and
MCP APIs.

## Completion

Ticking a checklist item does not complete the parent task. Completing a task
does not automatically tick its steps; the steps remain as a record. Reopening
a task leaves checklist state untouched.

## Repeating tasks

When a repeating task is completed, Flowlist creates the next occurrence, copies
the checklist, and resets every copied item to incomplete. This is useful for
weekly reviews and recurring procedures:

```sh
flowlist add 'Weekly review ::Work repeat:every week' 
flowlist checklist add <task> 'Clear inbox'
flowlist checklist add <task> 'Review projects'
flowlist done <task>       # next week's task has both steps unchecked
```

A repeating task without due/defer dates still recurs: completing it produces a
fresh copy with no dates.

## CLI

```sh
flowlist checklist list <task>
flowlist checklist add <task> 'Book the room' --before 0
flowlist checklist set <task> <item> --title 'Book the large room'
flowlist checklist set <task> <item> --done
flowlist checklist move <task> <item> --to 2
flowlist checklist remove <task> <item>
```

`<item>` accepts a full id, an unambiguous prefix or the short eight-character
suffix. `flowlist checklist list --json` returns the parent task plus the item
array, so scripts do not need a second lookup.

## MCP

```json
{"name":"list_checklist","arguments":{"id":"01M..."}}
{"name":"add_checklist_item","arguments":{"id":"01M...","title":"Book the room"}}
{"name":"update_checklist_item","arguments":{"id":"01M...","item":"7F3P2K9Q","completed":true}}
{"name":"move_checklist_item","arguments":{"id":"01M...","item":"7F3P2K9Q","to":0}}
{"name":"delete_checklist_item","arguments":{"id":"01M...","item":"7F3P2K9Q"}}
```

## GUI

- Click **Add a step…** in the inspector, type, press `⏎`.
- `⌘L` focuses the checklist input for the selected task.
- Click a step's circle to toggle it.
- Click a step's title to rename it; `⏎` saves, `Esc` cancels.
- Use the `↑` / `↓` controls to reorder and `×` to delete.
- Hover the inspector buttons; sidebar entity actions stay hidden until the
  pointer is directly over them.

Every checklist action goes through the command engine, so `⌘Z` undoes it just
like a task edit.
