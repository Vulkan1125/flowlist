# Using the phone

Flowlist for Android is a client of the library on your Mac. There is no
account and no cloud: the app talks to `flowlist serve`, which runs inside the
Mac app.

## Pair

1. On the Mac, open **Settings → Mobile** (`⌘6`).
2. Turn on **Serve to my phone**. This installs a login agent, so closing the
   window does not stop syncing.
3. The card shows one or more addresses and a six-digit code. Both devices must
   be on the same Wi-Fi, or both on Tailscale.
4. On the phone, type one of the addresses and the code, then **Pair**.

Revoke a phone from the same card. **Unpair this phone** in the app's Settings
also removes its token from the Mac.

## What syncs

The phone reads a cached snapshot and sends operations back through the same
`flowlist-core` commands as the GUI and CLI, so validation, repeats, dates,
projects and tags behave identically.

- **Online:** captures, completions and deletes appear on the Mac immediately,
  and the Mac's revision watcher refreshes the open window.
- **Offline:** reading works from the cache. Captures and completions queue with
  a **Waiting** mark and replay when the Mac is reachable. A capture keeps the
  id it was given offline, and retries are deduplicated, so nothing is added
  twice.
- **Undo:** the snackbar reverses the action you just took. It is scoped to the
  phone and never moves the Mac's global undo cursor.

## Make it yours

Settings has four ways to change the look:

- **Appearance** — System, Light or Dark.
- **Theme** — the same twelve families as the Mac: Todoist (red on white), Plain, Paper, Mist, Catppuccin, Nord, Gruvbox, Dracula, Tokyo Night, Vesper, Graphite and Midnight. The Todoist family is the default; the others use the Mac palettes.
- **Accent** — the theme's own accent, a **Wallpaper** colour from Android's Material You palette (Android 12 and later), or a custom tone chosen from eleven presets. Each preset has a light and a dark version, so there is no colour picker to fiddle with.
- **App icon** — six launcher colours: red, blue, green, orange, purple and graphite. Changing one replaces the home-screen icon in place.

## Edit a task

Tap a task to open the editor. Change the title and notes, then set:

- due and defer dates with Android's date and time pickers (or clear either),
- project, priority (P1–P3), the flag and an estimate,
- any existing tag.

**Save** sends one `update_task` command through the same engine as the Mac, plus a `set_task_tags` command when tags changed. It works offline: the change waits with the other queued operations and syncs later. Complete and Move to Trash are in the same sheet; the snackbar undoes either.

## Screens

- **Inbox** is unscheduled work, exactly as on the Mac. Adding a due or defer
  date moves a task to Today or Forecast.
- **Today** lists overdue and today, overdue first.
- **Forecast** groups the next fortnight by day.
- **Browse** lists projects with their open counts.
- **Capture** parses the same syntax: `#tag`, `#parent/child`, `::Project`,
  `due:friday`, `defer:tomorrow`, `est:45m`, `repeat:every week` and `!`.
- **Task detail** shows notes, dates, tags and checklist progress, with complete,
  reopen and move to Trash.

## Security

The server listens on your local network and requires a bearer token created by
pairing. Pairing codes are single-use and expire after ten minutes. On a LAN the
traffic is plain HTTP, protected by the token; Tailscale encrypts it end to end
when you are away. There is no port forwarding and no public endpoint.

## Requirements

Android 8 or later. The APK is signed with a Flowlist release key; install a
newer one over the old app to update.

## Empty states

Inbox, Today, Forecast and Browse show a small hand-drawn illustration when they have nothing in them. The check, sun, calendar and folder doodles animate gently, so an empty view feels intentional rather than unfinished.

## Home screen and reminders

- **Widget**: add Flowlist from the Android widget picker to see today's and Inbox counts. Tap it to open the app.
- **Due reminders**: Settings → Due reminders schedules a quiet summary when tasks are due today or overdue. Android 13 and later asks for notification permission the first time; nothing is sent off the device.

## Settings

Settings is grouped into cards:

- **Appearance** — system/light/dark, theme, accent and launcher icon.
- **Tasks** — first day of the week, the default time used by the date sheet,
  haptics and swipe actions.
- **Notifications** — due reminders, permission state and a test notification.
- **Connection** — the Mac address, live status, last sync, a manual sync and
  unpairing.
- **About** — version and links.

The sync badge is intentionally absent from the top bar. If the Mac has been
unreachable for more than a minute while the app is open, a slim line appears
below the heading; tapping it retries.

## Search and checklists

The magnifier in the header searches the cached library by task title, notes,
project or tag. Results can be completed in place or opened.

Checklist steps appear in the task editor. Tapping one sends
`update_checklist_item` through the same offline queue as every other edit.

## Subtasks

Child tasks appear indented under their parent. In the task editor, the
**Subtasks** section lists them and offers **Add subtask**; the new task is
created under the current one and stays in the same project. Completing a
subtask uses the same optimistic offline queue.

Each subtask row has a **move** button that opens a searchable picker of
top-level tasks; choosing one re-parents the subtask there and inherits that
task's project.

## Checklists

**Add checklist item** appends a step to the current task. Every step has up
and down arrows for reordering and a delete button. Ticking a step is
independent of completing the task, and all edits use the same offline queue.

Settings → About → **Check for updates** reads the latest GitHub release and
offers the APK when a newer version exists.

## Projects, folders and tags

**Browse** groups open projects under their folders. **New project** and
**New folder** sit at the top of the list. A folder is a section header with
its own menu: edit it, create a project or child folder inside, or delete the
whole tree. A project's menu edits its name, folder and status, archives it, or
deletes it and moves its tasks to Trash.

The project editor also opens from the pencil beside a project's name. Status
choices are Active, On Hold, Done and Dropped; Done and Dropped leave Browse
and appear in Archive.

**Tags** has a New tag row. Each tag's menu renames or re-parents it, or
deletes it from every task and project. Tag names and sibling uniqueness follow
the same rules as the Mac. From the task editor you can move a task to any
open project or toggle its tags.

## Forecast

Forecast opens on a **month grid**. Each day shows how many tasks are due, with
previous/next month and **Today** controls. Tapping a day lists its tasks below
the grid. **Agenda** switches back to the rolling two-week list.

## Saved perspectives

The drawer has a **Saved** section. **New perspective** builds a view from any
combination of text, project, tag, priority and due window, then chooses
grouping (project, due date, tag, folder), sorting (due, name, priority,
flagged, estimate) and whether completed tasks appear. Saved perspectives use
the Mac's `save_perspective` command, so they appear there too; the row menu
edits or deletes them.

## Perspectives and the drawer

The bottom bar holds four perspectives and **Settings** on the far right.
Settings → Appearance → **Bottom bar** assigns each of the four slots; picking
a perspective that already lives in another slot swaps the two.

Review, Completed, Archive, Trash, Tags, Priorities, Changed and Attachments
open as full pages, not modal sheets. Back returns to the previous perspective.
Inside Tags, Back first returns to the tag list, then leaves Tags.

The hamburger lists only the perspectives that are **not** on the bottom bar,
under a **Perspectives** heading, with the connection status pinned at the
bottom. Settings is never repeated there.

Tag counts match the tag view: active tasks only, including child tags, never
completed or trashed tasks.

Settings lives in the fifth tab. Its home lists Appearance, Tasks, Library,
Notifications, Connection and About, and each opens a short page with a back
arrow. The header no longer carries a gear. **Settings → Tasks → Open on**
chooses Inbox, Today, Forecast or Library for a normal launch.

Library defaults — review interval, changed window, completed visibility and
trash retention — write through `update_settings`, so the Mac sees the same
values. Repeating tasks can be set to None, Daily, Weekly, Monthly or Yearly,
and a series can be deleted in one action.
