#!/bin/sh
# Row layout probe: the per-view Layout menu.
#
# Seeds one flagged task with a project, tag, due date and estimate, then uses
# real mouse input to:
#   1. check the default Fluid / Comfortable / default-column state;
#   2. open the Layout menu from the content header;
#   3. switch density to Compact and confirm the row actually shrinks;
#   4. hide the Project field and confirm the view remembers it;
#   5. switch to Columns and turn the Defer column on;
#   6. relaunch and confirm the view kept its layout, density and checkboxes;
#   7. write one screenshot of each mode when SHOT_DIR is provided.
#
# macOS, Swift and Accessibility permission are required, like the other GUI
# probes.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}

if [ "$(uname -s)" != "Darwin" ]; then
	echo "SKIP: the layout probe needs macOS"
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

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-layout.XXXXXX")
lib=$tmp/lib
state=$tmp/state.json
shots=$tmp/shots
helpers=$tmp/bin
mkdir -p "$lib" "$helpers" "$shots"
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

"$bin" --data-dir "$lib" add 'Draft the proposal' \
	--project 'Client work' --tag 'writing' --due '2026-12-05' \
	--estimate 1h --flag >/dev/null

python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump(
    {
        "theme": "plain",
        "appearance": "light",
        "view": "flagged",
        "window": [1254.0, 1000.0],
        "inspector": False,
        "vim_mode": False,
        "selected": [],
    },
    open(sys.argv[1], "w"),
)
PY

launch() {
	rm -f "$state"
	FLOWLIST_TEST_STATE="$state" FLOWLIST_TEST_SHOT_DIR="$shots" \
		FLOWLIST_TEST_FLOAT=1 "$bin" --data-dir "$lib" gui \
		>"$tmp/out.log" 2>"$tmp/err.log" &
	pid=$!
	for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
	window=""
	for _ in $(seq 1 30); do
		window=$("$helpers/window" "$pid" move 40 20 || true)
		if [ -n "$window" ]; then
			wx=$(printf '%s' "$window" | cut -f2)
			wy=$(printf '%s' "$window" | cut -f3)
			WX=${wx%.*}
			WY=${wy%.*}
			WX=${WX%,*}
			WY=${WY%,*}
			if [ "$WX" -ge 0 ] && [ "$WX" -lt 1500 ] && [ "$WY" -ge 0 ] && [ "$WY" -lt 500 ]; then
				break
			fi
		fi
		sleep 0.3
	done
	# Park the pointer away from the header so a click from an earlier run
	# cannot land on the Layout chip while the first frame settles.
	"$helpers/flowctl" move $((WX + 20)) $((WY + 20))
	echo "window=$window"
}

value() {
	python3 - "$state" "$1" <<'PY'
import json, sys
try:
    value = json.load(open(sys.argv[1])).get(sys.argv[2])
except (OSError, ValueError):
    value = None
print("" if value is None else value)
PY
}
wait_value() {
	for _ in $(seq 1 80); do
		[ "$(value "$1")" = "$2" ] && return 0
		sleep 0.1
	done
	echo "FAIL: $1 is '$(value "$1")', wanted '$2'" >&2
	exit 1
}
wait_has() {
	# wait_has <key> <member> for the JSON list keys such as columns.
	for _ in $(seq 1 80); do
		if python3 - "$state" "$1" "$2" <<'PY'
import json, sys
try:
    values = json.load(open(sys.argv[1])).get(sys.argv[2]) or []
except (OSError, ValueError):
    values = []
sys.exit(0 if sys.argv[3] in values else 1)
PY
		then
			return 0
		fi
		sleep 0.1
	done
	echo "FAIL: $1 never contained '$2' (got '$(value "$1")')" >&2
	exit 1
}
wait_map() {
	map=$1
	key=$2
	for i in $(seq 1 80); do
		if python3 - "$state" "$map" "$key" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get(sys.argv[2], {})
except (OSError, ValueError):
    boxes = {}
sys.exit(0 if sys.argv[3] in boxes else 1)
PY
		then
			return 0
		fi
		if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
		"$helpers/flowctl" move $((WX + PX)) $((WY + PY))
		sleep 0.1
	done
	echo "FAIL: $map box '$key' never painted" >&2
	exit 1
}
click_map() {
	map=$1
	key=$2
	wait_map "$map" "$key"
	prev=""
	for i in $(seq 1 20); do
		point=$(python3 - "$state" "$map" "$key" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
)
		if [ "$point" = "$prev" ]; then break; fi
		prev=$point
		if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
		"$helpers/flowctl" move $((WX + PX)) $((WY + PY))
		sleep 0.15
	done
	"$helpers/flowctl" mouse $((WX + ${point% *})) $((WY + ${point#* }))
	sleep 0.6
}
row_height() {
	python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
rows = d.get("content_row_boxes") or {}
if not rows:
    print(0)
else:
    print(int(next(iter(rows.values()))[3]))
PY
}
shot() {
	name=$1
	"$helpers/flowctl" key 111
	file=""
	for _ in $(seq 1 40); do
		file=$(ls "$shots"/shot-*.png 2>/dev/null | head -1)
		[ -n "$file" ] && break
		sleep 0.1
	done
	if [ -z "$file" ]; then
		echo "FAIL: no screenshot was written" >&2
		exit 1
	fi
	mv "$file" "$shots/$name.png"
	echo "  screenshot=$shots/$name.png"
}

echo "1. Defaults reproduce today's fluid rows"
launch
wait_value layout fluid
wait_value density comfortable
wait_has columns project
wait_has columns tags
wait_has columns due
wait_has columns flag
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["layout"] == "fluid", d["layout"]
assert d["density"] == "comfortable", d["density"]
assert d["hidden_fields"] == [], d["hidden_fields"]
assert set(d["columns"]) == {"project", "tags", "due", "flag"}, d["columns"]
assert d["content_rows"], "no task rows"
assert d["header_view_label"] == "View", d["header_view_label"]
assert d["header_view_summary"] == "Default order and grouping", d["header_view_summary"]
assert d["header_view_dot"] is False, d["header_view_dot"]
print("  fresh view: fluid, comfortable, four default columns")
PY
shot fluid

echo "2. The View chip holds Sort, Group and Layout; Layout has every control"
click_map header_view_boxes chip
wait_value header_view_menu True
for key in chip tab:sort tab:group tab:layout; do
	wait_map header_view_boxes "$key"
done
click_map header_view_boxes tab:layout
wait_value header_view_tab layout
for key in mode:fluid mode:columns density:comfortable density:compact \
	field:project field:flag column:project column:defer; do
	wait_map header_view_boxes "$key"
done
echo "  View menu: three tabs, mode pills, density pills, seven fields, eight columns"

echo "3. Compact density actually shrinks the row"
comfortable=$(row_height)
click_map header_view_boxes density:compact
wait_value density compact
for _ in $(seq 1 40); do
	compact=$(row_height)
	[ "$compact" -lt "$comfortable" ] && break
	sleep 0.1
done
if [ "$compact" -ge "$comfortable" ]; then
	echo "FAIL: compact row is $compact, comfortable was $comfortable" >&2
	exit 1
fi
echo "  row height: comfortable ${comfortable}px -> compact ${compact}px"

echo "4. One fluid field can be hidden and is remembered"
click_map header_view_boxes field:project
wait_has hidden_fields project
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "project" in d["hidden_fields"], d["hidden_fields"]
print("  project field is checked off in the menu")
PY

echo "5. Columns mode and one optional column"
click_map header_view_boxes mode:columns
wait_value layout columns
click_map header_view_boxes column:defer
wait_has columns defer
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["layout"] == "columns", d["layout"]
assert "Columns" in d["header_view_summary"], d["header_view_summary"]
# Layout changes stay quiet; only a sort or grouping change gets the dot.
assert d["header_view_dot"] is False, d["header_view_dot"]
assert "defer" in d["columns"], d["columns"]
assert {"project", "tags", "due", "flag"} <= set(d["columns"]), d["columns"]
print("  columns layout with the Defer column on")
PY
# Close the menu so the next screenshot shows the rows, not the card.
"$helpers/flowctl" key 53
wait_value header_view_menu False
shot columns

echo "6. The session keeps this view's layout"
python3 - "$lib/session.json" <<'PY'
import json, sys
session = json.load(open(sys.argv[1]))
options = session.get("view_options", {}).get("flagged", {})
assert options.get("layout") == "columns", options
assert options.get("density") == "compact", options
assert "project" in options.get("hidden_fields", []), options
assert "defer" in options.get("columns", []), options
print("  saved flagged view: columns, compact, project hidden, defer visible")
PY

echo "7. A relaunch restores the same view's layout"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
pid=""
launch
wait_value layout columns
wait_value density compact
wait_has hidden_fields project
wait_has columns defer
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["layout"] == "columns", d["layout"]
assert d["density"] == "compact", d["density"]
assert "project" in d["hidden_fields"], d["hidden_fields"]
assert "defer" in d["columns"], d["columns"]
print("  relaunch kept the flagged view's columns layout")
PY

echo "8. The Priorities dropdown selects and clears a priority"
click_map header_priority_boxes chip
wait_value header_priority_menu True
for key in chip priority:all priority:high priority:medium priority:low; do
	wait_map header_priority_boxes "$key"
done
click_map header_priority_boxes priority:high
wait_value priority_filter high
click_map header_priority_boxes priority:all
wait_value priority_filter ""
"$helpers/flowctl" key 53
wait_value header_priority_menu False
echo "  Priorities menu: High selected, All priorities restored"

if [ -n "${SHOT_DIR:-}" ]; then
	mkdir -p "$SHOT_DIR"
	cp "$shots/fluid.png" "$SHOT_DIR/gui-layout-fluid.png"
	cp "$shots/columns.png" "$SHOT_DIR/gui-layout-columns.png"
	echo "  copied screenshots to $SHOT_DIR"
fi

echo "layout probe: PASS"
