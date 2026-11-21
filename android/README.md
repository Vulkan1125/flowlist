# Flowlist for Android

The phone is a client of the Mac library. There is no second library and no
cloud: captures, completions and deletes are sent to `flowlist serve` on the
Mac, which applies them through the same `flowlist-core` commands as the GUI,
CLI and MCP server.

## Build

```sh
export JAVA_HOME=/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

The SDK location lives in `local.properties` (gitignored):

```properties
sdk.dir=/opt/homebrew/share/android-commandlinetools
```

## Pair

1. On the Mac: **Settings → Mobile**, turn on **Serve to my phone**. The login
   agent keeps the server running after the window closes.
2. Note one of the addresses (LAN or Tailscale) and the six-digit code.
3. On the phone: enter both. The token is stored in app-private preferences.

## Layout

- `data/` — DTOs mirroring `data.json`, the HTTP client, the snapshot cache and
  the offline outbox.
- `ui/` — Plain design system and the Compose screens.
- `scripts/` — device builds and screenshots.

## Notes

- Cleartext HTTP is enabled for the LAN. Tailscale encrypts remote traffic; the
  bearer token still guards every request.
- Offline captures get a client-generated ULID, so the Mac and phone agree on
  the id before the first sync. Operations carry an id the server deduplicates.
- Themes: Todoist (default), Plain, Paper, Mist, Catppuccin, Nord, Gruvbox,
  Dracula, Tokyo Night, Vesper, Graphite and Midnight, with theme, wallpaper or
  custom accent tones and six launcher icons.
- Full task editing (title, notes, dates, project, priority, flag, estimate,
  tags), a home-screen widget and due reminders.

Everything in this folder can be deleted without touching the Mac app.
