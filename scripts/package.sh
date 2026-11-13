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
	<key>NSHumanReadableCopyright</key>
	<string>MIT licensed</string>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist" >/dev/null
codesign --force --sign - "$app" >/dev/null 2>&1 || {
	echo "warning: ad-hoc signing failed; the bundle still runs locally" >&2
}

echo "packaged $app ($version)"
echo "launch with: open $app"
