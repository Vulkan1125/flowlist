#!/bin/sh
# Mouse-only GUI smoke test.
#
# Usage: scripts/gui-smoke.sh
#
# Builds two small Swift/CoreGraphics helpers, starts the release binary on a
# temporary library, then uses real mouse clicks to:
#   1. open the sidebar "New project" sheet, type a name and create it;
#   2. open a saved perspective with its hover pencil.
#
# The app writes FLOWLIST_TEST_STATE (view, open modal, selection, window size)
# so the test can wait for state instead of guessing from pixels. If macOS,
# Swift or Accessibility permission is missing the test skips instead of
# failing a headless environment.

set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
bin=${BIN:-$root/target/release/flowlist}

if [ "$(uname -s)" != "Darwin" ]; then
	echo "SKIP: the GUI smoke test needs macOS"
	exit 0
fi
if ! command -v swiftc >/dev/null 2>&1; then
	echo "SKIP: swiftc is not installed"
	exit 0
fi
if [ ! -x "$bin" ]; then
	echo "missing $bin; run: cargo build --release" >&2
	exit 1
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-gui-smoke.XXXXXX")
lib="$tmp/library"
state="$tmp/state.json"
helpers="$tmp/bin"
mkdir -p "$lib" "$helpers"
pid=""
cleanup() {
	if [ -n "$pid" ]; then
		kill "$pid" 2>/dev/null || true
		wait "$pid" 2>/dev/null || true
	fi
	rm -rf "$tmp"
}
trap cleanup EXIT

swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

"$bin" --data-dir "$lib" add 'Seed task' >/dev/null
"$bin" --data-dir "$lib" perspective add 'Smoke view' >/dev/null

FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/gui.log" 2>&1 &
pid=$!

window=""
for _ in $(seq 1 100); do
	if [ -s "$state" ]; then
		window=$("$helpers/window" "$pid" 2>/dev/null || true)
		[ -n "$window" ] && break
	fi
	sleep 0.2
done
if [ -z "$window" ]; then
	echo "FAIL: the Flowlist window did not appear" >&2
	cat "$tmp/gui.log" >&2
	exit 1
fi

read_window() {
	window=$("$helpers/window" "$pid" 2>/dev/null || true)
	[ -n "$window" ] || {
		echo "FAIL: lost the Flowlist window" >&2
		exit 1
	}
	wx=$(printf '%s' "$window" | cut -f2)
	wy=$(printf '%s' "$window" | cut -f3)
	ww=$(printf '%s' "$window" | cut -f4)
	wh=$(printf '%s' "$window" | cut -f5)
}

modal() {
	python3 - "$state" <<'PY'
import json
import sys

try:
    print(json.load(open(sys.argv[1])).get("modal") or "")
except (OSError, ValueError):
    print("")
PY
}

focus_app() {
	read_window
	"$helpers/flowctl" mouse "$((${wx%.*} + ${ww%.*} / 2))" "$((${wy%.*} + 200))"
	sleep 0.3
}

# 1. Sidebar "New project" button. The button row wraps above the status bar;
#    try a few offsets so a small theme or spacing change still finds it.
opened=""
for offset in 77 90 70 84 97 63 104; do
	focus_app
	"$helpers/flowctl" mouse "$((${wx%.*} + 35))" "$((${wy%.*} + ${wh%.*} - offset))"
	sleep 0.5
	if [ "$(modal)" = "New project" ]; then
		opened=1
		break
	fi
	"$helpers/flowctl" key 53
	sleep 0.2
done
if [ "$opened" != 1 ]; then
	echo "FAIL: the New project button did not open its sheet" >&2
	exit 1
fi

printf 'Smoke project' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.3
"$helpers/flowctl" key 36
sleep 0.9

if ! "$bin" --data-dir "$lib" project list --json | grep -q '"Smoke project"'; then
	echo "FAIL: the New project sheet did not create a project" >&2
	exit 1
fi

# 2. Hover pencil on the only saved perspective. The row sits below the
#    nine built-in perspectives, the PRIORITIES section and its three rows, so
#    its centre is roughly 482px below the top of the window. The offsets cover
#    small spacing drift.
opened=""
for y in 482 478 486 474 490 470 494 466 498 462 502 458 506 454 510; do
	for x in 190 195 185; do
		focus_app
		"$helpers/flowctl" mouse "$((${wx%.*} + x))" "$((${wy%.*} + y))"
		sleep 0.45
		if [ "$(modal)" = "Perspective" ]; then
			opened=1
			break 2
		fi
		"$helpers/flowctl" key 53
		sleep 0.15
	done
done
if [ "$opened" != 1 ]; then
	echo "FAIL: the perspective pencil did not open the editor" >&2
	exit 1
fi

"$helpers/flowctl" key 53
echo "GUI smoke test passed: New project and perspective pencil"
