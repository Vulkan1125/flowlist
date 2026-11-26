#!/bin/sh
# Assemble dist/Flowlist.app from the release binary.
#
# Usage: scripts/package.sh
#
# The bundle is self-contained: the GUI, CLI and MCP server are the same
# executable, so `Flowlist.app/Contents/MacOS/flowlist` can also be used from a
# shell (`flowlist.app/Contents/MacOS/flowlist list`).

set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

if [ ! -f assets/icons/Flowlist.icns ]; then
	scripts/make-icon.sh
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
cargo build --release --locked

app=dist/Flowlist.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

cp target/release/flowlist "$app/Contents/MacOS/flowlist"
cp assets/icons/Flowlist.icns "$app/Contents/Resources/Flowlist.icns"

cat >"$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>flowlist</string>
	<key>CFBundleIconFile</key>
	<string>Flowlist</string>
	<key>CFBundleIdentifier</key>
	<string>dev.flowlist.Flowlist</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Flowlist</string>
	<key>CFBundleDisplayName</key>
	<string>Flowlist</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>$version</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.productivity</string>
	<key>LSMinimumSystemVersion</key>
	<string>13.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSCalendarsUsageDescription</key>
	<string>Flowlist reads your Calendar events to show them in Forecast.</string>
	<key>NSCalendarsFullAccessUsageDescription</key>
	<string>Flowlist reads your Calendar events to show them in Forecast.</string>
	<key>NSHumanReadableCopyright</key>
	<string>MIT licensed</string>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist" >/dev/null
# macOS only grants UserNotifications to a non-ad-hoc signature. Set
# FLOWLIST_SIGN_IDENTITY to an Apple Development or Developer ID identity to
# produce a build where Settings -> Workflow -> Reminders can actually ask for
# permission. Without it the app is ad-hoc signed and notifications are
# refused by the system.
if [ -n "${FLOWLIST_SIGN_IDENTITY:-}" ]; then
	if ! codesign --force --options runtime --sign "$FLOWLIST_SIGN_IDENTITY" "$app" >/dev/null; then
		echo "error: could not sign with $FLOWLIST_SIGN_IDENTITY" >&2
		exit 1
	fi
	echo "signed with: $FLOWLIST_SIGN_IDENTITY"
else
	# A stable designated requirement matters for macOS privacy databases.
	# A plain ad-hoc signature is tied to the CDHash, so every rebuild looks
	# like a new app and Calendar/Notifications ask again. Keep the identifier
	# stable across rebuilds instead.
	codesign --force --sign - \
		--identifier dev.flowlist.Flowlist \
		-r='designated => identifier "dev.flowlist.Flowlist"' \
		"$app" >/dev/null 2>&1 || {
		echo "warning: ad-hoc signing failed; the bundle still runs locally" >&2
	}
	echo "warning: ad-hoc signed; notifications need FLOWLIST_SIGN_IDENTITY" >&2
fi

echo "packaged $app ($version)"

# The in-app updater downloads a release zip and looks for Flowlist.app inside
# it, so keep the bundle at the archive root and publish this file as the
# release asset.
zip="dist/Flowlist-$version.zip"
rm -f "$zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$zip"
echo "release asset: $zip"

# A drag-to-Applications disk image for people who prefer not to unzip.
dmg="dist/Flowlist-$version.dmg"
rm -rf dist/dmg-root
mkdir -p dist/dmg-root
cp -R "$app" dist/dmg-root/Flowlist.app
ln -s /Applications dist/dmg-root/Applications
rm -f "$dmg"
if hdiutil create -volname "Flowlist" -srcfolder dist/dmg-root -ov -format UDZO "$dmg" >/dev/null; then
	echo "release image: $dmg"
else
	echo "warning: could not create $dmg" >&2
fi
rm -rf dist/dmg-root
echo "launch with: open $app"
