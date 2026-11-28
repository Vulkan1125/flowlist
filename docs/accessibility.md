# Accessibility

Flowlist is keyboard-first by design. Every action that has an icon-only button
also has a key route, and every icon-only control explains itself with a
tooltip after a short hover. This page records what is covered and where the
current GPUI toolkit limits assistive technology.

## Keyboard routes for icon-only controls

A task row's flag is a hover-only control, and sidebar rows keep their actions
in the right-click menu, so the keyboard routes below are the primary way to
reach them without the mouse.

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

### Quick capture panel

`⌘O` opens Quick Open, a keyboard-first navigation palette for perspectives,
folders, projects, tags and priorities. It is also the fastest route for a
keyboard user to reach an entity without walking the sidebar.

`⌃⌥Space` opens the floating capture panel from any app; the same chord closes
it again without filing. The title field has keyboard focus as soon as the
panel opens, and `Esc` always closes it without filing. Inside the panel:

| Key | Action |
| --- | --- |
| `Enter` | Add the task and close |
| `⇧Enter` | Add the task and keep capturing |
| `⌘Enter` | Add the task and select it in the main window |
| `Esc`, `⌘W` | Close without adding |
| `⌘A`, `⌘C`, `⌘X`, `⌘V` | Select all, copy, cut and paste in the field |

The panel names its destination in the footer and previews the parsed title,
project, tags, dates, estimate and repeat rule as chips, so the result of a
marker is visible before `Enter` files it. A plain capture always files into the
Inbox; `::Project` files into that project, and `due:` / `defer:` schedules the
task into Today or Forecast so it leaves the Inbox. The panel stays above normal
windows when another app comes forward. Nothing in the main window moves while
the panel is open, and a completed capture can be undone with `⌘Z`. `Enter` and
`Esc` return focus to the app that was in front before capture; `⌘Enter` keeps
Flowlist in front because it reveals the task. The title-bar plus button has the
same tooltip and keyboard route.

The title bar holds the search field, centered on the content pane with the
inspector's column reserved even when it is closed, so the content header keeps
only the view title and View/Available chips on one row. A saved view's
description sits below it; a view without a description skips that row. While a chip menu or a sheet is open it owns
the pointer: hover and clicks cannot reach the rows behind it. Focusing search
(`⌘F` or `/`) selects its contents; its × clears the query and keeps the field
focused. If a view name is
too long to fit beside the controls it is omitted rather than clipped; the same
name is always available in the sidebar and the window title, and every view
has a keyboard shortcut.

The Forecast month calendar adds three controls beside the month name:
previous month, Today and next month. They are labelled `‹`, `Today` and `›`
with tooltips. Every date cell is clickable and has a tooltip naming the day;
the selected day is highlighted and an "All dates" button clears it. The
calendar always shows one full-width month, so the cells never shrink into
empty slabs on a large window. A Calendar chip in the content header hides and
restores the calendar (`forecast-calendar-toggle`); it remembers the last mode,
so reopening returns to the same week or month. Hiding also clears the picked
day and restores the full agenda. A chevron beside the month name
folds the full grid to the current-week strip (`forecast-calendar-collapse`),
and the strip's chevron opens the full month again (`forecast-week-expand`).
Visibility, mode and week are saved with the session. Keyboard routes: `[` /
`]` step the selected day, `{` / `}` step the month, and `\` clears the day
filter.

The calendar's height is adjustable. Drag the grip under the grid with the
mouse, or press `-` / `=` to shrink or grow it by a row. The height is clamped
so the day cells stay readable and the list below always keeps room, and it is
saved with the session, so a size chosen once survives a relaunch. The list
below keeps the normal task keyboard routes.

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
| Hide past chip (Today) | Show or hide past plans and overdue tasks in Today | click only |
| Crosshair on a tag | Set location / Change location | `⇧L` |
| Plus buttons | Project / Folder / Tag / View | `⇧⌘N`, `⌥⌘N`, `⌃⌘N`, `⇧⌘P` |

`Ctrl-w h` focuses the sidebar, except inside a text field where `Ctrl-w` is
left to the field. Arrow keys also move through the sidebar, `Enter` opens the
row, and `Esc` returns to the content list.

Every sidebar row also has a right-click menu with the actions that apply to
it: Open, Rename, Delete, New project/subfolder/subtag, project status, tag
location and Empty Trash. The same actions keep their keyboard routes, and
`Esc` closes the menu. It is the only place where a sidebar row's rename,
delete, status and location actions live.

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

The **View** chip opens Sort, Group and Layout tabs under one popover. It
stays open while choices change, and `Esc` or an outside click closes it. The
chip keeps a fixed width and the card names the active sort, grouping and
layout at the top. The dot marks only a changed sort or grouping; a layout
change stays quiet. **Available** sits beside View as a one-click filter, and
**Completed** appears where a view can show finished work. Priorities keep
their High / Medium / Low rows in the sidebar. When a typed project path does
not exist, a Create project sheet asks first: `Enter` creates the project and
files the task, `Esc` leaves the task unchanged.

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

Subtasks sit indented under their parent in every list and start collapsed, so
they keep their place through sorting and grouping. The disclosure chevron hides or
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

Toolbar chips (`View`, `Filter`, `Completed`, `Priority`) and project status
and type chips are labelled buttons rather than icon-only controls, so they
work with the mouse and are described by their text. Their tooltips add detail
about what a click does. The small × that clears filters has its own tooltip.

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

A sidebar row's actions live in its right-click menu, so the name always owns
the full row width and long names end in an ellipsis. Those actions keep their
keyboard routes (`⌘R` to rename, `⌘⌫` to delete, `dd` in vim mode) and their
tooltips.

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
- sidebar disclosure arrows;
- task completion circles, flags and subtask disclosures;
- checklist tick boxes, reorder arrows and delete buttons;
- inspector flag, close, tag-remove, Due/Defer calendar and project-status
  controls;
- every Forecast date cell and the priority filter chip.

The help card (`?`) lists the same routes in one place, so a keyboard user does
not need to discover a tooltip first. Settings sections are reachable with
`⌘1`–`⌘8`, `⇥` visits only the fields in the section on screen, and Plain's
accent presets are the digits `1`–`9` while Appearance is open.

## Focus and selection visibility

- The focused pane is the one that receives vim motions. `Ctrl-w h`, `Ctrl-w l`
  and `Ctrl-w j` move between the sidebar, content and checklist.
- Sidebar rows under the keyboard cursor get the same background as rows under
  the mouse.
- Selection in the content list is independent of the mouse and is announced in
  the status bar (`N selected`).
- Every theme keeps a single accent colour for selection and focus. Plain
  lets that accent be chosen from nine presets (`1`–`9` while Appearance is
  open), a custom picker or the system eyedropper; custom tones are clamped so
  the colour still carries text and primary-button labels at 4.5:1.
  Destructive actions use one warm red and are never encoded by colour alone —
  the close icon and the tooltip carry the meaning.
- The reading surface stays opaque in all eleven theme families, in both light and
  dark mode. Translucency is limited to the sidebar, inspector, header and
  status bar, so background contrast never reaches task text. Light and dark
  backgrounds are chosen separately, and each is held to the same contrast
  floor as the rest of the chrome.
- Chrome tint is checked at both gradient stops, and against the blur over a
  pure black (light) or white (dark) background. The wash is decorative; it can
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

## Full-screen reminders

A task, project or folder can be marked for full-screen reminders at creation,
or from the task inspector. Only marked items (and tasks inside a marked
project or folder) take over the screen. The reminder window is focusable:
**S** cycles the snooze length (15, 30 or 60 minutes), **Enter** snoozes with
the shown length, **⌘Enter** completes the task, **O** opens it in the main
window, **V** opens the temporary Full-screen list when several share a time,
and **Esc**
dismisses the reminder. Every action is also a labelled
button. On Android the same actions are buttons on the alarm screen; TalkBack
still reaches them when the reminder is drawn over another app.
