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
