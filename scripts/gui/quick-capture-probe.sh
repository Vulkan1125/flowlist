#!/bin/sh
# Quick capture probe: the floating panel, its destinations and its focus.
#
# Seeds one inbox task and starts the app, then uses real keyboard and mouse
# input to:
#   1. open the panel with ⌃⌥Space while Finder is in front;
#   2. file a plain thought and confirm it lands in the Inbox;
#   3. file a dated thought and confirm it leaves the Inbox for Forecast;
#   4. press Return to close the panel and hand focus back to the old app;
#   5. drop Finder in front again and confirm the panel stays on screen;
#   6. dismiss with Esc and confirm the previous app owns the screen again.
#
# The app writes FLOWLIST_TEST_STATE on every frame, so the probe waits for
# state instead of guessing timings. macOS, Swift and Accessibility permission
# are required, like the smoke test.

set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}

if [ "$(uname -s)" != "Darwin" ]; then
	echo "SKIP: the quick capture probe needs macOS"
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

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-capture.XXXXXX")
lib=$tmp/lib
state=$tmp/state.json
capture=$tmp/capture.json
helpers=$tmp/bin
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

# The app refuses to open a second library window for the same data directory,
# so a stale packaged copy must not be running.
"$bin" --data-dir "$lib" add 'Seed task' >/dev/null
python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({
    "view": "inbox",
    "window": [1000.0, 900.0],
    "inspector": False,
    "selected": [],
    "vim_mode": True,
}, open(sys.argv[1], "w"))
PY

FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
if [ ! -s "$state" ]; then
	echo "FAIL: the Flowlist window did not appear" >&2
	cat "$tmp/err.log" >&2
	exit 1
fi

value() {
	python3 - "$1" "$2" <<'PY'
import json, sys
try:
    print(json.load(open(sys.argv[1])).get(sys.argv[2], ""))
except (OSError, ValueError):
    print("")
PY
}
wait_value() {
	file=$1
	key=$2
	want=$3
	for _ in $(seq 1 80); do
		if [ "$(value "$file" "$key")" = "$want" ]; then
			return 0
		fi
		sleep 0.1
	done
	echo "FAIL: $key never became '$want' (got '$(value "$file" "$key")')" >&2
	exit 1
}
expect() {
	if [ "$2" != "$3" ]; then
		echo "FAIL: $1: got '$2', wanted '$3'" >&2
		exit 1
	fi
	echo "  $1: $2"
}

# The panel is the only window the process owns above the normal layer.
# Reading its painted rectangle keeps the probe honest when the display or
# its origin changes.
panel_line() {
	"$helpers/window" "$pid" layers | awk -F'\t' '$2 >= 3 { print; exit }'
}
panel_box() {
	panel_line | awk -F'\t' '{ print $3, $4, $5, $6 }'
}
panel_layer() {
	panel_line | cut -f2
}
click_panel() {
	box=$(panel_box)
	if [ -z "$box" ]; then
		echo "FAIL: the capture panel never painted" >&2
		exit 1
	fi
	set -- $box
	px=$1
	py=$2
	pw=$3
	"$helpers/flowctl" mouse "$((${px%.*} + ${pw%.*} / 2))" "$((${py%.*} + 40))"
}

open_panel() {
	open -a Finder
	osascript -e 'tell application "Finder" to activate' >/dev/null 2>&1 || true
	sleep 1.2
	"$helpers/flowctl" key 49 ctrlalt
	wait_value "$capture" open True
	# Synthetic keys do not reach a freshly opened panel until it is clicked.
	for _ in 1 2 3 4 5; do
		click_panel
		sleep 0.4
		[ "$(value "$capture" window_active)" = "True" ] && return 0
	done
	echo "FAIL: the capture panel never took keyboard focus" >&2
	exit 1
}

paste() {
	printf '%s' "$1" | pbcopy
	"$helpers/flowctl" key 9 cmd
	for _ in $(seq 1 40); do
		[ "$(value "$capture" text)" = "$1" ] && return 0
		sleep 0.1
	done
	echo "FAIL: pasting '$1' did not reach the capture field" >&2
	exit 1
}

title_list() {
	"$bin" --data-dir "$lib" list "$@" --json |
		python3 -c 'import json,sys; print("\n".join(t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]))'
}

# 1. Plain text files into the Inbox, and ⇧⏎ keeps the panel open.
open_panel
paste 'Plain capture'
"$helpers/flowctl" key 36 shift
wait_value "$capture" last_captured 'Plain capture'
wait_value "$capture" text ''
expect "plain destination" "$(value "$capture" flash)" "Added to Inbox"
echo "  inbox: $(title_list --inbox | tr '\n' ' ')"
if ! title_list --inbox | grep -qx 'Plain capture'; then
	echo "FAIL: a plain capture did not land in the Inbox" >&2
	exit 1
fi
if title_list --forecast | grep -qx 'Plain capture'; then
	echo "FAIL: a plain capture should not be scheduled" >&2
	exit 1
fi

# 2. An explicit due date schedules the task, so it leaves the Inbox.
paste 'Dated capture due:2026-11-25'
python3 - "$capture" <<'PY'
import json, sys
syntax = json.load(open(sys.argv[1])).get("syntax") or []
if "due:2026-11-25" not in syntax:
    print("FAIL: the due marker was not interpreted:", syntax, file=sys.stderr)
    sys.exit(1)
PY
"$helpers/flowctl" key 36 shift
wait_value "$capture" last_captured 'Dated capture'
if title_list --inbox | grep -qx 'Dated capture'; then
	echo "FAIL: a dated capture stayed in the Inbox" >&2
	exit 1
fi
if ! title_list --forecast | grep -qx 'Dated capture'; then
	echo "FAIL: a dated capture did not reach Forecast" >&2
	exit 1
fi
echo "  dated: left the Inbox for Forecast"

# 3. Full-screen reminder syntax opts the capture in without a button.
paste 'Reminder capture fsr:1h30m @1'
python3 - "$capture" <<'PY'
import json, sys
syntax = json.load(open(sys.argv[1])).get("syntax") or []
if "priority:high" not in syntax:
    print("FAIL: priority was not interpreted:", syntax, file=sys.stderr)
    sys.exit(1)
if not any(item.startswith("fsr:20") for item in syntax):
    print("FAIL: fsr was not resolved to a date:", syntax, file=sys.stderr)
    sys.exit(1)
if any("Reminder capture" in item for item in syntax):
    print("FAIL: the syntax read-out leaked the title:", syntax, file=sys.stderr)
    sys.exit(1)
PY
"$helpers/flowctl" key 36 shift
wait_value "$capture" last_captured 'Reminder capture'
if ! title_list --fullscreen | grep -qx 'Reminder capture'; then
	echo "FAIL: fsr: capture was not a full-screen reminder" >&2
	exit 1
fi
paste 'Discarded capture fsr'
"$helpers/flowctl" key 36 shift
wait_value "$capture" last_captured 'Discarded capture'
if title_list --fullscreen | grep -qx 'Discarded capture'; then
	echo "FAIL: a mark with no time should be discarded" >&2
	exit 1
fi
echo "  fsr: opted in, no-time mark ignored"

paste 'Offset capture due:3d'
python3 - "$capture" <<'PY'
import json, sys
syntax = json.load(open(sys.argv[1])).get("syntax") or []
if not any(item.startswith("due:20") for item in syntax):
    print("FAIL: due:3d did not resolve to a date:", syntax, file=sys.stderr)
    sys.exit(1)
PY
"$helpers/flowctl" key 36 shift
wait_value "$capture" last_captured 'Offset capture'
echo "  due:3d resolved to a date"

# 4. Return closes the panel and puts the old app back in front.
paste 'Closing capture'
"$helpers/flowctl" key 36
wait_value "$capture" open False
wait_value "$state" view inbox
for _ in $(seq 1 40); do
	[ "$("$helpers/window" front)" = "$(pgrep -x Finder | head -n 1)" ] && break
	sleep 0.1
done
expect "focus restored" "$("$helpers/window" front)" "$(pgrep -x Finder | head -n 1)"
if ! title_list --inbox | grep -qx 'Closing capture'; then
	echo "FAIL: Return did not file the capture" >&2
	exit 1
fi
echo "  return: filed and handed focus back"

# 4. The panel stays on top of normal windows when Flowlist loses focus.
open_panel
open -a Finder
sleep 1
expect "still open" "$(value "$capture" open)" "True"
if [ -z "$(panel_line)" ]; then
	echo "FAIL: the capture panel disappeared when Flowlist lost focus" >&2
	exit 1
fi
expect "window layer" "$(panel_layer)" "3"
echo "  float: stays above normal windows while another app is in front"

# 5. Esc closes it and returns to Finder, without touching the main view.
click_panel
sleep 0.5
"$helpers/flowctl" key 53
wait_value "$capture" open False
for _ in $(seq 1 40); do
	[ "$("$helpers/window" front)" = "$(pgrep -x Finder | head -n 1)" ] && break
	sleep 0.2
done
expect "esc restores focus" "$("$helpers/window" front)" "$(pgrep -x Finder | head -n 1)"
expect "main view" "$(value "$state" view)" "inbox"

echo "OK: quick capture files to the Inbox, schedules dated tasks and stays on top"
