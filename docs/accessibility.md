# Accessibility

Flowlist is keyboard-first by design. Every action that has an icon-only button
also has a key route, and every icon-only control explains itself with a
tooltip after a short hover. This page records what is covered and where the
current GPUI toolkit limits assistive technology.

## Keyboard routes for icon-only controls

Hover-only controls (the sidebar pencil and ×) appear when the mouse is over
them, so the keyboard routes below are the primary way to reach them.

### Window header

| Control | Tooltip | Keyboard |
| --- | --- | --- |
| Add task | Add a task (⌃⌥Space) | `⌃⌥Space` from anywhere; `n` or `⌘N` in a capture view |
| Undo arrow | Undo (⌘Z) | `⌘Z` or `u` |
| Redo arrow | Redo (⇧⌘Z) | `⇧⌘Z` or `Ctrl-r` |
| Question mark | Keyboard and mouse help (?) | `?` |
| Gear | Settings (⌘,) | `⌘,` |
| Moon / sun | Switch to dark/light mode (⌘T) | `⌘T`, or `t` in vim mode |
| Info (i) | Toggle inspector (i) | `i` or `⌘I` |

The title bar holds the search field, centered on the content pane, so the
content header keeps only the view title and Sort/Group/Filter chips on one
row, with the subtitle below. Focusing search (`⌘F` or `/`) selects its
contents; its × clears the query and keeps the field focused. If a view name is
too long to fit beside the controls it is omitted rather than clipped; the same
name is always available in the sidebar and the window title, and every view
has a keyboard shortcut.

The Forecast month calendar adds three controls beside the month name:
previous month, Today and next month. They are labelled `‹`, `Today` and `›`
with tooltips. Every date cell is clickable and has a tooltip naming the day;
the selected day is highlighted and an "All dates" button clears it. On a wide
pane the calendar shows two consecutive months; the heading names the range and
each month keeps its own weekday rail. Keyboard routes: `[` / `]` step the
selected day, `{` / `}` step the month, and `\` clears the day filter. The
list below keeps the normal task keyboard routes.

### Sidebar

| Control | Tooltip | Keyboard |
| --- | --- | --- |
| Pencil on a saved perspective | Edit perspective | `Ctrl-w h` then `r` |
| Pencil on a project / folder / tag | Rename | `Ctrl-w h` then `r` |
| × on a perspective / folder / tag / project | Delete | `Ctrl-w h` then `⌘⌫` or `dd` |
| Chevron on a folder | Expand / collapse folder | `l` / `h` |
| Chevron on a tag | Expand / collapse tag | `l` / `h` |
| Crosshair on a tag | Set location / Change location | `⇧L` |
| Plus buttons | Project / Folder / Tag / View | `⇧⌘N`, `⌥⌘N`, `⌃⌘N`, `⇧⌘P` |

`Ctrl-w h` focuses the sidebar, except inside a text field where `Ctrl-w` is
left to the field. Arrow keys also move through the sidebar, `Enter` opens the
row, and `Esc` returns to the content list.

Every sidebar row also has a right-click menu with the actions that apply to
it: Open, Rename, Delete, New project/subfolder/subtag, project status, tag
location and Empty Trash. The same actions keep their keyboard routes, and
`Esc` closes the menu. Hovering a row reveals its pencil and × even when the
row is the active view.

### Dropdown menus and the new-project prompt

Sort and Group open a menu under their chip, and the inspector's project
picker opens once you type. Clicking anywhere outside closes them, and `Esc`
does the same. When a typed project path does not exist, a Create project
sheet asks first: `Enter` creates the project and files the task, `Esc` leaves
the task unchanged.

### Task rows and inspector

| Control | Tooltip | Keyboard |
| --- | --- | --- |
| ○ completion circle | Complete task (x) / Reopen task (x) | `x` or `space` |
| Flag | Flag task (!) / Unflag task (!) | `!` or `⌘D` |
| ▸ / ▾ subtasks | Expand / collapse subtasks | `l` / `h` on the parent, or `Tab` to nest |
| ○ checklist step | Tick / untick step | `Ctrl-w j`, then `space` |
| ↑ / ↓ checklist arrows | Move step up / down | `⇧J` / `⇧K` in the checklist pane |
| × checklist step | Delete step | `⌫` in the checklist pane |
| × tag chip | Remove tag | no direct key; remove it in the inspector Tag field |
| × close inspector | Close inspector (i) | `i` or `⌘I` |

Toolbar chips (`Sort`, `Group`, `Completed`, `Priority`) and project status
and type chips are labelled buttons rather than icon-only controls, so they
work with the mouse and are described by their text. Their tooltips add detail
about what a click does.

Trash is an ordinary perspective in the same keyboard model. `Delete` moves
selected tasks to Trash everywhere except inside Trash, where it permanently
deletes them; `r` restores the selection, and the header exposes labelled
Restore and Delete forever buttons. Deleting keeps the next row selected.
The × on a sidebar project opens a Delete project sheet that names the project
and explains that its tasks move to Trash. Enter confirms, Esc cancels, and
`⌘Z` still restores everything in one step.

A sidebar row's pencil and × stay hidden until the row is hovered, even when
the row is selected. While idle the name uses the whole row width; hovering
makes room for the actions, and a long name fades at that edge instead of
ending in an ellipsis. The same actions keep their keyboard routes (`⌘R` to
rename, `⌘⌫` to delete, `dd` in vim mode) and their tooltips.

## Tooltip coverage

Tooltips stay hidden for roughly five seconds after hover begins, and clicking
a button mutes its tooltip until the pointer leaves the button. They are
attached to every icon-only control:

- the labelled Add task button and the six header icon buttons;
- sidebar pencils, × buttons, disclosure arrows and the `⌖` location button;
- task completion circles, flags and subtask disclosures;
- checklist tick boxes, reorder arrows and delete buttons;
- inspector flag, close, tag-remove and project-status controls;
- every Forecast date cell and the priority filter chip.

The help card (`?`) lists the same routes in one place, so a keyboard user does
not need to discover a tooltip first.

## Focus and selection visibility

- The focused pane is the one that receives vim motions. `Ctrl-w h`, `Ctrl-w l`
  and `Ctrl-w j` move between the sidebar, content and checklist.
- Sidebar rows under the keyboard cursor get the same background as rows under
  the mouse.
- Selection in the content list is independent of the mouse and is announced in
  the status bar (`N selected`).
- Every theme keeps a single accent colour for selection and focus; destructive
  actions use one warm red and are never encoded by colour alone — the close
  icon and the tooltip carry the meaning.
- The reading surface stays opaque in all ten theme families, in both light and
  dark mode. Translucency is limited to the sidebar, inspector, header and
  status bar, so wallpaper contrast never reaches task text.
- Chrome tint is checked at both gradient stops, and against the blur over a
  pure black (light) or white (dark) wallpaper. The wash is decorative; it can
  never lower text below the contrast floors.

## Known limitation: the accessibility tree

GPUI 0.2.2 does not expose an accessibility tree. VoiceOver sees the window
title but cannot inspect individual buttons, rows or text fields, and there is
no `aria-*` equivalent to annotate controls. That is a toolkit limitation, not
a Flowlist setting; when GPUI gains an accessibility API these labels and hints
can be attached without changing the interaction model.

Until then, Flowlist compensates with:

- key routes for every icon-only action, documented in the help card;
- delayed tooltips on every icon-only control;
- visible focus and selection states;
- a CLI and MCP server that expose the same library for screen-reader-friendly
  terminal workflows.

## What was checked

- Every `widgets::icon_button` call site and every icon-only row control was
  inspected for a tooltip.
- Disclosure arrows in the sidebar and task rows received tooltips that state
  the expand/collapse action.
- Sidebar delete now matches its tooltip and the help card: `⌘⌫` removes the
  selected perspective, folder or tag, and `dd` does the same in vim mode.
- Window title, window bounds and launch were verified through the macOS
  accessibility API. A full VoiceOver walk-through is not possible with GPUI
  0.2.2 because the internal controls are not in the tree.
