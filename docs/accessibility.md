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
| Back / forward arrows | Back (⌘[) / Forward (⌘]) | `⌘[` and `⌘]`, or the header arrows |
| Undo arrow | Undo (⌘Z) | `⌘Z` or `u` |
| Redo arrow | Redo (⇧⌘Z) | `⇧⌘Z` or `Ctrl-r` |
| Question mark | Keyboard and mouse help (?) | `?` |
| Gear | Settings (⌘,) | `⌘,` |
| Moon / sun | Switch to dark/light mode (⌘T) | `⌘T`, or `t` in vim mode |
| Info (i) | Toggle inspector (i) | `i` or `⌘I` |

The title bar holds the search field, centered on the content pane, so the
content header keeps only the view title and Sort/Group/Filter chips on one
row, with the subtitle below. While a chip menu or a sheet is open it owns
the pointer: hover and clicks cannot reach the rows behind it. Focusing search
(`⌘F` or `/`) selects its contents; its × clears the query and keeps the field
focused. If a view name is
too long to fit beside the controls it is omitted rather than clipped; the same
name is always available in the sidebar and the window title, and every view
has a keyboard shortcut.

The Forecast month calendar adds three controls beside the month name:
previous month, Today and next month. They are labelled `‹`, `Today` and `›`
with tooltips. Every date cell is clickable and has a tooltip naming the day;
the selected day is highlighted and an "All dates" button clears it. On a wide
pane the calendar shows two consecutive months; the heading names the range and
each month keeps its own weekday rail. Keyboard routes: `[` / `]` step the
selected day, `{` / `}` step the month, and `\` clears the day filter.

Two more chips sit before those controls. Months cycles Auto → one month → two
months (`M`); Auto follows the pane width, while a pinned count stays put even
in a narrow pane. Width switches between calendar-sized cells and a calendar
that fills the pane (`W`). Both chips show the current choice and have
tooltips; the settings are stored in the library, so they survive a relaunch.
Alternating date cells carry a decorative checker tint built from the theme's
text colour at 4% alpha; the contrast audit checks numbers on a tinted square
just as it does every other surface, so the pattern never trades legibility for
texture. The list below keeps the normal task keyboard routes.

### Sidebar

| Control | Tooltip | Keyboard |
| --- | --- | --- |
| Pencil on a saved perspective | Edit perspective | `Ctrl-w h` then `r` |
| Pencil on a project / folder / tag | Rename | `Ctrl-w h` then `r` |
| × on a perspective / folder / tag / project | Delete | `Ctrl-w h` then `⌘⌫` or `dd` |
| Chevron on a folder | Expand / collapse folder | `l` / `h` |
| Section heading chevron | Collapse / expand a rail section | Settings (`⌘,`) hides or shows whole sections; the pointer folds one for the session |
| Folder row | Opens the folder; a second click folds its tree | `Enter` on the row does the same |
| Chevron on a tag | Expand / collapse tag | `l` / `h` |
| Hide overdue chip (Today) | Show or hide overdue tasks in Today | click only |
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

Projects can be dragged onto a folder row to file them, onto the LIBRARY
heading, or onto empty space under the tree to take them back to the root. The
folder row or heading under the pointer highlights during the drag, so the
destination is visible before releasing, and the project's painted row moves
as soon as the drop lands. Each move is one undoable command, so `⌘Z`
restores the previous placement. Task drags onto projects and tags are
unchanged. Filing a project is pointer-only for now; the sidebar cursor can
open and rename it but cannot move it between folders, so a keyboard route is
still a gap.

### Dropdown menus and the new-project prompt

Sort and Group open a menu under their chip, and the inspector's project
picker opens once you type. Clicking anywhere outside closes them, and `Esc`
does the same. When a typed project path does not exist, a Create project
sheet asks first: `Enter` creates the project and files the task, `Esc` leaves
the task unchanged.

### The View sheet

Each condition in a saved perspective is a labelled row that opens a menu
listing every value, rather than a chip that cycles. Once a menu is open, `j`
and `k` or the arrow keys move the highlight, Home and End jump to the ends,
`Enter` chooses and `Esc` closes the menu without changing the form. `Tab`
still moves between the sheet's text fields, `Esc` closes the sheet, and
`Enter` saves it. The summary row says how many tasks the current conditions
match and offers Clear filters; clearing is the only action that removes the
conditions the GUI cannot show. A decision menu leaves each chosen value in
view, so the selected project, status, due window, grouping and sort can be
read without opening anything.

### Task rows and inspector

Subtasks appear indented under their parent from the start in every list, and
keep their place through sorting and grouping. The disclosure chevron hides or
shows a whole branch with one click, and folding a middle task leaves its
siblings visible. `za` toggles the selected task's branch, `zo` opens it and
`zc` closes it; a task without children says so in the status bar instead of
changing the view. Expansion is view state, not a library edit, so it is not
undoable and does not touch the data file.

| Control | Tooltip | Keyboard |
| --- | --- | --- |
| ○ completion circle | Complete task (x) / Reopen task (x) | `x` or `space` |
| Flag | Flag task (!) / Unflag task (!) | `!` or `⌘D` |
| ▸ / ▾ subtasks | Expand / collapse subtasks | `za` toggles, `zo` opens, `zc` closes |
| ○ checklist step | Tick / untick step | `Ctrl-w j`, then `space` |
| ↑ / ↓ checklist arrows | Move step up / down | `⇧J` / `⇧K` in the checklist pane |
| × checklist step | Delete step | `⌫` in the checklist pane |
| × tag chip | Remove tag | no direct key; remove it in the inspector Tag field |
| × close inspector | Close inspector (i) | `i` or `⌘I` |
| Project badge | `N open · M of T done`, or `All N done` | opens the project |
| Calendar | Pick a due/defer date | icon opens it; arrows, `Enter`, `T`, `Delete`, `Esc` |
| All day / At time | Keep the day all-day / set a time | `A` toggles; chips are clickable |
| Clock face | Set the hour and minute | arrows, `Page Up`/`Page Down`, `Tab` or `H`/`M`, `A`, `Enter`, `Esc` |

### Due and Defer date pickers

The calendar icon at the right of each date field opens a two-step picker
below it. Step one is a month grid: `←`/`→` move a day, `↑`/`↓` a week,
`Page Up`/`Page Down` change month, `Enter` picks, `T` picks today, `Delete`
clears the field and `Esc` closes without changing anything. Clicking the
field's text still opens the normal editor, so a typed time of day can be
entered there; picking a day from the calendar keeps a time that is already
set.

The All day / At time chips choose what the day carries. All day (the default
when the field has no time) commits the day as soon as it is clicked, so a
plain date stays two clicks. At time turns the picker into step two, either
straight away or after a day is chosen: a clock face showing 1–12 with the
hour hand. Clicking or using the arrow keys sets the hour and advances to
minutes, where the numbers run 0–55 in fives; `Page Up`/`Page Down` move five,
`Tab` or `H`/`M` switch between hour and minute, and the digital hour, minute
and AM/PM controls set the same values. `Enter` applies the date and time,
`Esc` goes back to the calendar with the time kept, and `Delete` clears the
field. The time starts at the library's default due/defer time (9 AM when none
is set) and is only written on Done or Enter, so backing out leaves the stored
stamp alone.

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
`⌘Z` still restores everything in one step. The × on a folder, its right-click
menu and `⌘⌫` / `dd` open the same kind of sheet, explaining that its nested
folders, projects and their tasks move to Trash with it and that one Undo
restores the tree.

Deleting a repeating task opens a two-choice sheet. `Tab` and the arrow keys
move between **This occurrence** and **Whole series**, `Enter` confirms the
highlighted choice, `Esc` cancels, and `⌘Z` restores every trashed occurrence.

The Archive is a project surface rather than a task list. Finished and dropped
projects sit in collapsible folder sections under `Library` or their folder
path, each row opens that project, and a labelled Reopen button shows on every
row. Reopen runs the ordinary project-status command, so `⌘Z` puts the project
straight back in the Archive. The pane is reached from the sidebar like any
other perspective, and when nothing is left it shows a calm “No finished
projects” message.

A sidebar row's pencil and × stay hidden until the row is hovered, even when
the row is selected. While idle the name uses the whole row width; hovering
makes room for the actions, and a long name fades at that edge instead of
ending in an ellipsis. The same actions keep their keyboard routes (`⌘R` to
rename, `⌘⌫` to delete, `dd` in vim mode) and their tooltips.

Inside the inspector's inline fields, `Tab` and `Enter` save the current value
and move to the next field; a validation error keeps that field open, and
`Enter` on the last field ends the session. A first-time task stays a draft
until the session ends, so its Delete and Complete actions do not appear while
the user is still filling it in. Clicking away from a field or opening a sheet
also ends the session.

A rename sheet opens with the whole name selected. Typing replaces it, one
`Backspace` or `Delete` clears it, and the arrow keys or a click place the
caret. Dragging inside any sheet text field selects a range, and `⌘A`, `⌘C`
and `⌘X` work as usual.

## Tooltip coverage

Tooltips stay hidden for three seconds after hover begins, and clicking
a button mutes its tooltip until the pointer leaves the button. They are
attached to every icon-only control:

- the labelled Add task button and the six header icon buttons;
- sidebar pencils, × buttons, disclosure arrows and the `⌖` location button;
- task completion circles, flags and subtask disclosures;
- checklist tick boxes, reorder arrows and delete buttons;
- inspector flag, close, tag-remove, Due/Defer calendar and project-status
  controls;
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
