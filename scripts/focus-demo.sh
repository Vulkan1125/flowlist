#!/bin/sh
# Open a disposable library for testing Focus without touching the real one.
#
# Usage: scripts/focus-demo.sh [LIBRARY_DIR]
#
# The default directory persists so the focus eye can be tested across launches.
# Delete it when you are done.

set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
bin=${BIN:-$root/dist/Flowlist.app/Contents/MacOS/flowlist}
if [ ! -x "$bin" ]; then
	bin=$root/target/release/flowlist
fi
if [ ! -x "$bin" ]; then
	echo "missing Flowlist; run scripts/package.sh or cargo build --release --features gui" >&2
	exit 1
fi

lib=${1:-${TMPDIR:-/tmp}/flowlist-focus-demo}
mkdir -p "$lib"

if [ ! -f "$lib/data.json" ]; then
	"$bin" --data-dir "$lib" folder add "Demo" >/dev/null
	"$bin" --data-dir "$lib" project add "Focus demo" --folder "Demo" >/dev/null
	"$bin" --data-dir "$lib" project add "Outside focus" >/dev/null
	"$bin" --data-dir "$lib" add "Inside task one" -p "Focus demo" >/dev/null
	"$bin" --data-dir "$lib" add "Inside task two" -p "Focus demo" --due today >/dev/null
	"$bin" --data-dir "$lib" add "Outside task" -p "Outside focus" >/dev/null
fi

cat <<EOF
Test library: $lib

1. Right-click "Focus demo" and choose "Focus on ...".
2. Click the eye to clear the focus.
3. Click it again: it restores "Focus demo".
4. Quit and rerun this script with the same path to check persistence.

Your normal library is not opened.
EOF

exec "$bin" --data-dir "$lib" gui
