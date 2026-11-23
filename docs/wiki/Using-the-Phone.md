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
- **Theme** — twelve families: Tomato (red on white), Plain, Paper, Mist, Catppuccin, Nord, Gruvbox, Dracula, Tokyo Night, Vesper, Graphite and Midnight. Pick a light theme and a dark theme separately; Tomato is the default for both. The light choice is used whenever the app is light and the dark choice whenever it is dark, so the two palettes stay independent of each other. Outside Tomato, medium-priority yellow uses one clear gold/yellow pair so it never reads as brown or as red, and high-priority red uses one vivid red pair so it never looks muted.
- **Accent** — the theme's own accent, a **Wallpaper** colour from Android's Material You palette (Android 12 and later), or a custom tone chosen from eleven presets. Each preset has a light and a dark version, so there is no colour picker to fiddle with.
- **App icon** — six launcher colours: red, blue, green, orange, purple and graphite. Changing one replaces the home-screen icon in place.

## Edit a task

Tap a task to open the editor. Change the title and notes, then set:

- due and defer dates with Android's date and time pickers (or clear either),
- project, priority (H, M or L), the flag and an estimate. The selected pill and the task circle share the priority colour: red for H, yellow for M and blue for L,
- any existing tag,
- files through **Add attachment**, and web links through **Add link**.

**Save** sends one `update_task` command through the same engine as the Mac, plus a `set_task_tags` command when tags changed. On a repeating task, changing the priority asks whether to apply it to **This task** or **All repeats**; the series choice uses `set_task_priority` with `series: true`.

A defer date after the due date is rejected before saving, with the message under the date rows. Clearing a date, project, estimate or repeat sends an explicit `null`, which now clears that field instead of leaving the old value in place. It works offline: the change waits with the other queued operations and syncs later. Complete and Move to Trash are in the same sheet; the snackbar undoes either.

## Screens

- **Inbox** is unscheduled work, exactly as on the Mac. Adding a due or defer
  date moves a task to Today or Forecast.
- **Today** lists overdue and today, overdue first.
- **Forecast** groups the next fortnight by day.
- **Browse** lists projects with their open counts.
- **Capture** parses the same syntax: `#tag`, `#parent/child`, `::Project`,
  `due:friday`, `defer:tomorrow`, `est:45m`, `repeat:every week` and `!`.
- **Task detail** shows notes, dates, tags and subtasks, with complete,
  reopen and move to Trash.
- Lists that support bulk actions have a checklist button at the top right.
  Tap it to enter select mode; it becomes a close button, and tapping it again
  leaves select mode.

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

- **Widget**: add Flowlist from the Android widget picker to see today's and Inbox counts. Tap it to open the app. The **+** button opens the floating capture card in its own task, so it does not pull the app forward when Flowlist is in the background. A widget set to **Follow app** updates its colours immediately when the app theme changes.
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

## Search

The magnifier in the header searches the cached library by task title, notes,
project or tag. Results can be completed in place or opened.

## Subtasks

Child tasks appear indented under their parent. In the task editor, the
**Subtasks** section lists them and offers **Add subtask**; the new task is
created under the current one and stays in the same project. Completing a
subtask uses the same optimistic offline queue.

Each subtask row has a **move** button that opens a searchable picker of
top-level tasks; choosing one re-parents the subtask there and inherits that
task's project.

Every subtask row also has a drag grip. Long-press it to reorder the children;
the other rows move aside and the list settles into the new order on release.

**Parallel**, **Sequential** and **One-off** are the three step modes; only one
is active. The drag grip for both subtasks and one-off steps is at the far right
of the row. Parallel and Sequential show child tasks; in Sequential only the
first incomplete child can be ticked and later rows are dimmed. One-off shows
checklist-style steps that tick, drag and delete in place and never appear as
tasks. Switching to One-off with child tasks asks first: each child becomes a
step, and notes, dates, tags and priority are removed. Switching back restores
the steps as subtasks.

The project picker is as wide as its row. Its last item is **Create new
project**, and the tag row ends with a **+** chip for a new tag; both select the
new entity on the task. Tag chips wrap onto further lines when they run out of
room.

## Attachments

In the task editor, **Add attachment** opens the system file picker. The file
is uploaded to `attachments/` inside the library and referenced on the task, so
the Mac sees the same path. Tapping an attachment downloads it to the phone's
cache and opens it with any installed viewer; the close button removes it from
the task. The download button saves a durable copy inside the app's private
files, after which the attachment opens without the Mac. Tapping that button
again removes the offline copy.

**Add link** attaches an `http(s)` URL; it opens in the browser and appears in
the same attachment list as files. The dialog also takes an optional **Name**;
leave it empty to show the address as before. Existing links have a rename
action. Links do not show the offline download button; only file attachments
can be saved on the phone. Attachments alone uses the coloured headings: High is
red, Medium amber and Low blue; No priority is a filled badge. Date headings use
the theme accent, and a missing date is a filled badge such as **No due date**.
In dark mode those badges invert to a white fill with grey text. Other views keep
their quiet grey headings. Groups leave clear space between each other.

Uploads go over `POST /api/v1/files` and downloads over `GET /api/v1/files`.
Both require the paired token. Files are capped at 40 MB, and the Mac only
serves paths inside its attachments folder or paths already listed on a task or
project.

### Save everything

**Settings → Library → Offline files** has a **Save all attachments** toggle.
Turning it on downloads every attachment in the library, including project
attachments, and new attachments are saved as the library syncs.

**Remove all offline copies** deletes every saved file from the phone after a
confirmation warning and switches the toggle off. Files on the Mac are never
touched.

## Task rows

Priority is shown by the task circle and the selected pill: red for high,
yellow for medium and blue for low. A flagged task shows a single flag at the
far right of its row; priorities never add their own flag.

Subtasks start collapsed. Tap the chevron at the right of a parent row to open
or close its children. Long-press a task to drag it: drop it on another task to
nest it, or on the lower half of a root task to move it back to the project
root at that position. The whole row tints while it is the target.


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

Forecast opens on a **Todoist-style date strip**. The Month/Agenda selector uses
roomy pills with clear space between them. Under the month name and its chevron,
a single row of seven dates shows how many tasks are due each day. The strip
starts at today and shows the next seven days; it does not scroll back before
today. Swipe it left to move seven days forward, through future months. Drag the
strip down, or drag the pill below it, to open the full month grid; drag up on
the pill to fold it back. The change animates smoothly. The month title and its
chevron also toggle the grid. The grid opens on the month that was visible, and
in full mode the **Today** button and previous/next month arrows appear and the
calendar can move backwards. The full month grid scrolls with the task list, so
a long day does not squash it; the collapsed strip keeps its space under the
month name. Tapping a day lists its tasks below the calendar. **Agenda** switches
back to the rolling two-week list and hides the month heading and its controls;
the choice is remembered across app restarts.

## Saved perspectives

**Flagged** is a built-in perspective next to Forecast. It lists every task
with the flag turned on, and can be pinned to the bottom bar like any other
perspective. Long-press a perspective in the drawer and drag it up or down to
change the order; the new order is remembered. **Priorities** uses a level
icon, so it is not confused with **Flagged**.

The drawer has a **Saved** section. **New perspective** opens as a card below
the status bar and matches the Mac editor:
text, Inbox only, Flagged only, Status, Projects, Tags with child tags,
Priorities, Due and Defer windows, an estimate range, grouping, sorting and
whether completed tasks appear. Grouping covers project, folder, tag, due,
defer, flagged and estimate; sorting covers manual, due, defer, name, priority,
flagged, estimate, created and completed. Saved perspectives use the Mac's
`save_perspective` command, so they appear there too; the row menu edits or
deletes them. Each list’s sort is remembered for that view, so changing Inbox
does not reorder Today, Forecast or the other destinations.

## Perspectives and the drawer

The bottom bar holds four perspectives and **Settings** on the far right.
Settings → Appearance → **Bottom bar** assigns each of the four slots; picking
a perspective that already lives in another slot swaps the two. Every row in
the drawer shows its task count when the count is non-zero. Tags and Nearby
count tasks rather than tags, and zero counts stay hidden. A saved perspective
with nothing matching shows a bare tick instead.

Review, Completed, Archive, Trash, Tags, Priorities, Changed and Attachments
open as full pages, not modal sheets. Back returns to the previous perspective.
Inside Tags, Back first returns to the tag list, then leaves Tags.

In **Trash**, swipe right to restore a task and left to delete it permanently;
the delete swipe still asks before purging. Settings → Tasks → **Trash swipes**
can swap those sides or turn swipes off, which brings the Restore and Delete
buttons back.

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
