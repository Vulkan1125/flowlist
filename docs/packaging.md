# Packaging Flowlist for macOS

Flowlist ships as one executable. The GPUI app, the CLI and the MCP server are
the same binary, so the app bundle only needs that file, the icon and a small
`Info.plist`.

## Build the app

```sh
scripts/package.sh
```

The script:

1. regenerates `assets/icons/Flowlist.icns` when it is missing;
2. reads the version from the workspace `Cargo.toml`;
3. runs `cargo build --release --locked`;
4. creates `dist/Flowlist.app` with `Contents/MacOS/flowlist`,
   `Contents/Resources/Flowlist.icns` and `Contents/Info.plist`;
5. validates the plist and ad-hoc signs the bundle (`codesign --sign -`).

The result can be launched from Finder or with:

```sh
open dist/Flowlist.app
```

`dist/` is ignored by Git; build it locally or attach it to a release.

## Bundle layout

```
dist/Flowlist.app/
  Contents/
    Info.plist
    MacOS/flowlist          the only executable
    Resources/Flowlist.icns
    _CodeSignature/         ad-hoc signature
```

Useful bundle identifiers:

| Key | Value |
| --- | --- |
| `CFBundleIdentifier` | `dev.flowlist.Flowlist` |
| `CFBundleShortVersionString` | workspace version (`0.1.0`) |
| `LSMinimumSystemVersion` | `13.0` |
| `LSApplicationCategoryType` | `public.app-category.productivity` |

## Launch with a specific library

The app accepts the same flags as the CLI. Pass them after `--args`:

```sh
open dist/Flowlist.app --args --data-dir "$HOME/Library/Application Support/Flowlist"
```

When no directory is given, Flowlist uses the platform data directory. The
`FLOWLIST_HOME` environment variable works for launched-from-shell runs:

```sh
FLOWLIST_HOME=/path/to/library dist/Flowlist.app/Contents/MacOS/flowlist
```

## Use the bundled binary as a CLI

```sh
dist/Flowlist.app/Contents/MacOS/flowlist list --json
dist/Flowlist.app/Contents/MacOS/flowlist add 'Draft release notes due:tomorrow'
dist/Flowlist.app/Contents/MacOS/flowlist mcp        # stdio MCP server
```

The same library can be open in the GUI and the CLI at once: writes are
serialized through a cross-process lock and the GUI reloads when another
process changes the library.

## Signing and Gatekeeper

`scripts/package.sh` produces an ad-hoc signature, which is enough for local
use. It is not Developer ID signed or notarized. On another Mac, Gatekeeper may
refuse the first launch; use **System Settings → Privacy & Security → Open
Anyway**, or remove the quarantine flag:

```sh
xattr -d com.apple.quarantine /Applications/Flowlist.app
```

For distribution, sign with a Developer ID certificate and notarize the bundle
before attaching it to a release.

## Regenerating the icon

```sh
scripts/make-icon.sh
```

The icon is drawn with Core Graphics by `scripts/make-icon.swift`, then scaled
into an `.iconset` with `sips` and packed with `iconutil`. This needs macOS with
the Swift toolchain; the checked-in `.icns` keeps Linux and CI builds working
without it.

## Build without GPUI

For servers or CI where a windowing toolkit is unnecessary:

```sh
cargo build --release --locked --no-default-features
```

That binary keeps the CLI and MCP server and omits the GUI code and its
dependencies.
