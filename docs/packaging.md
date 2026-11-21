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
5. validates the plist and ad-hoc signs the bundle (`codesign --sign -`);
6. packs `dist/Flowlist-<version>.zip`, the release asset the in-app updater
   expects.

The result can be launched from Finder or with:

```sh
open dist/Flowlist.app
```

`dist/` is ignored by Git; build it locally or attach it to a release.

## Publish a release

The in-app update check reads
`https://api.github.com/repos/Vulkan1125/flowlist/releases/latest`, compares
the tag with the running `Cargo.toml` version and downloads the first `.zip`
asset. To publish one:

```sh
# 1. Bump the workspace version in Cargo.toml, then commit.
# 2. Build the app and its zip.
scripts/package.sh

# 3. Tag the release commit and publish a normal (not draft or prerelease)
#    release with the zip attached.
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
gh release create "v$version" "dist/Flowlist-$version.zip" \
  --title "Flowlist $version" \
  --notes-file notes.md \
  --latest
```

The app only offers an update when the release tag is numerically newer than
the running version (`v0.2.0` beats `0.1.0`; a suffix such as `-beta.1` is
ignored for the comparison). The zip must keep `Flowlist.app` at its root;
`package.sh` does this. Downloads made by the app go through `curl`, so they
do not carry the quarantine flag that a browser download would.

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
