#!/bin/sh
# Build assets/icons/Flowlist.icns from the Core Graphics drawing script.
#
# Usage: scripts/make-icon.sh

set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
icons="$root/assets/icons"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-icon.XXXXXX")
setdir="$tmp/Flowlist.iconset"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$icons" "$setdir"
swift "$root/scripts/make-icon.swift" "$tmp/icon_1024.png"

for spec in \
	"16:icon_16x16.png" \
	"32:icon_16x16@2x.png" \
	"32:icon_32x32.png" \
	"64:icon_32x32@2x.png" \
	"128:icon_128x128.png" \
	"256:icon_128x128@2x.png" \
	"256:icon_256x256.png" \
	"512:icon_256x256@2x.png" \
	"512:icon_512x512.png" \
	"1024:icon_512x512@2x.png"; do
	scale=${spec%%:*}
	name=${spec#*:}
	sips -z "$scale" "$scale" "$tmp/icon_1024.png" --out "$setdir/$name" >/dev/null
done

iconutil -c icns "$setdir" -o "$icons/Flowlist.icns"
cp "$tmp/icon_1024.png" "$icons/Flowlist.png"
echo "wrote $icons/Flowlist.icns and $icons/Flowlist.png"
