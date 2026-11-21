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
