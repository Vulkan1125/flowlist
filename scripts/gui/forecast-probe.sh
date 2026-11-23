#!/bin/sh
# Forecast probe: the merged, collapsible, resizable month calendar.
#
# Seeds a small library with due dates, opens Forecast, then uses real mouse
# input to:
#   1. check that the calendar starts open with day cells and event chips;
#   2. fold it away and back from the content header chip;
#   3. click a date and confirm the list filters to that day, then clear it;
#   4. drag the bottom grip and confirm the height changes and is persisted;
#   5. drag past the minimum and confirm the clamp keeps the cells readable.
#
# The app writes FLOWLIST_TEST_STATE on every frame, and the probe reads
# painted boxes from it, so it never guesses at pixel coordinates. macOS,
# Swift and Accessibility permission are required, like the smoke test.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}

if [ "$(uname -s)" != "Darwin" ]; then
	echo "SKIP: the Forecast probe needs macOS"
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

tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-forecast.XXXXXX")
lib=$tmp/lib
state=$tmp/state.json
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

"$bin" --data-dir "$lib" add 'Plan launch due:2026-11-18' >/dev/null
"$bin" --data-dir "$lib" add 'Write brief due:2026-11-20' >/dev/null
"$bin" --data-dir "$lib" add 'Call Ana due:2026-11-20' >/dev/null
"$bin" --data-dir "$lib" add 'Book flight due:2026-11-24' >/dev/null
"$bin" --data-dir "$lib" add 'Overdue thing due:2026-11-10' >/dev/null
"$bin" --data-dir "$lib" add 'Later thing due:2026-12-15' >/dev/null

python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({
    "view": "forecast",
    "window": [1400.0, 1100.0],
    "inspector": True,
    "selected": [],
    "vim_mode": True,
    "forecast_calendar_open": True,
}, open(sys.argv[1], "w"))
PY

FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done

window=""
for _ in $(seq 1 30); do
	window=$("$helpers/window" "$pid" move 40 20 || true)
	wx=$(printf '%s' "$window" | cut -f2)
	wy=$(printf '%s' "$window" | cut -f3)
	WX=${wx%.*}
	WY=${wy%.*}
	WX=${WX%,*}
	WY=${WY%,*}
	if [ "$WX" -ge 0 ] && [ "$WX" -lt 1500 ] && [ "$WY" -ge 0 ] && [ "$WY" -lt 500 ]; then break; fi
	sleep 0.3
done
echo "window=$window"

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
	key=$1
	want=$2
	for _ in $(seq 1 60); do
		[ "$(value "$key")" = "$want" ] && return 0
		sleep 0.1
	done
	echo "FAIL: $key never became '$want' (got '$(value "$key")')" >&2
	exit 1
}
expect() {
	if [ "$2" != "$3" ]; then
		echo "FAIL: $1: got '$2', wanted '$3'" >&2
		exit 1
	fi
	echo "  $1: $2"
}
box_center() {
	python3 - "$state" "$1" <<'PY'
import json, sys
box = json.load(open(sys.argv[1]))[sys.argv[2]]
x, y, w, h = box
print(int(x + w / 2), int(y + h / 2))
PY
}
click_box() {
	coords=$(box_center "$1")
	set -- $coords
	"$helpers/flowctl" mouse $((WX + $1)) $((WY + $2))
	sleep 0.9
}

# 1. The calendar is open and painted: five or six week rows, event pills on
#    the seeded days, and the toggle chip and grip have boxes.
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["view"] == "forecast", d["view"]
assert d["forecast_calendar_open"] is True
assert len(d["forecast_days"]) in (35, 42), len(d["forecast_days"])
assert len(d["forecast_chips"]) == 5, len(d["forecast_chips"])
assert d["forecast_toggle_anchor"], "no toggle chip"
assert d["forecast_grip_anchor"], "no resize grip"
assert d["forecast_calendar_height"] is None, d["forecast_calendar_height"]
print("  calendar: %d day cells, %d chips, natural height" % (len(d["forecast_days"]), len(d["forecast_chips"])))
PY

# 2. The Calendar chip only hides and restores the calendar; it remembers the
#    last mode. The chevron beside the month name folds to the week strip, and
#    the strip's chevron opens the full month again.
click_box forecast_toggle_anchor
wait_value forecast_calendar_open False
click_box forecast_toggle_anchor
wait_value forecast_calendar_open True
wait_value forecast_calendar_expanded True
click_box forecast_calendar_collapse
wait_value forecast_calendar_expanded False
for _ in $(seq 1 40); do
  count=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["forecast_strip"]))' "$state")
  [ "$count" = "7" ] && break
  sleep 0.1
done
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d["forecast_week"], "no collapsed week"
assert len(d["forecast_strip"]) == 7, len(d["forecast_strip"])
assert d["forecast_strip_expand"], "no expand control"
print("  week strip: %s, %d days" % (d["forecast_week"], len(d["forecast_strip"])))
PY
cell=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
box = d["forecast_strip"]["2026-11-25"]
print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
)
set -- $cell
"$helpers/flowctl" mouse $((WX + $1)) $((WY + $2))
sleep 0.9
wait_value forecast_day 2026-11-25
click_box forecast_toggle_anchor
wait_value forecast_calendar_open False
click_box forecast_toggle_anchor
wait_value forecast_calendar_open True
wait_value forecast_calendar_expanded False
click_box forecast_strip_expand
wait_value forecast_calendar_expanded True
echo "  modes: chip remembers full and week, both chevrons switch, a day works"

# 3. Click 20 November: the list keeps that day's two tasks; hiding the
#    calendar clears the filter and restores the full agenda.
cell=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
box = d["forecast_days"]["2026-11-20"]
# The day number sits in the top-left corner; the cell centre can land on an
# event chip, which selects the task instead of toggling the day.
print(int(box[0] + 10), int(box[1] + 10))
PY
)
set -- $cell
"$helpers/flowctl" mouse $((WX + $1)) $((WY + $2))
sleep 0.9
wait_value forecast_day 2026-11-20
titles=$(python3 -c 'import json,sys; print(",".join(sorted(r["title"] for r in json.load(open(sys.argv[1]))["content_rows"])))' "$state")
expect "day filter" "$titles" "Call Ana,Write brief"

# Hiding the calendar always leaves the full agenda, even with a day picked.
click_box forecast_toggle_anchor
wait_value forecast_calendar_open False
wait_value forecast_day ""
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
titles = {row["title"] for row in d["content_rows"]}
assert {"Book flight", "Later thing"} <= titles, titles
PY
click_box forecast_toggle_anchor
wait_value forecast_calendar_open True
echo "  hide: the picked day clears and the full agenda returns"

# 4. Drag the grip down 120px. The height follows, and the session keeps it
#    after the debounced write.
grip=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
box = d["forecast_grip_anchor"]
print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
)
set -- $grip
"$helpers/flowctl" drag $((WX + $1)) $((WY + $2)) $((WX + $1)) $((WY + $2 + 120))
sleep 1.2
grew=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
height = d["forecast_calendar_height"]
assert height and height > 400, height
print(int(height))
PY
)
echo "  resize: dragged to ${grew}px"
expect "state height" "$(value forecast_calendar_height | cut -d. -f1)" "$grew"
saved=$(python3 -c 'import json,sys; print(int(json.load(open(sys.argv[1]))["forecast_calendar_height"]))' "$lib/session.json")
expect "saved height" "$saved" "$grew"

# 5. Drag far past the top. The clamp stops at readable cell height rather
#    than letting the month collapse.
grip=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
box = d["forecast_grip_anchor"]
print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
)
set -- $grip
"$helpers/flowctl" drag $((WX + $1)) $((WY + $2)) $((WX + $1)) $((WY + 40))
sleep 1.2
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
height = d["forecast_calendar_height"]
days = len(d["forecast_days"])
weeks = days // 7
# 44px per week row plus the heading chrome, with a little rounding slack.
assert weeks * 44 + 62 - 2 <= height <= weeks * 44 + 62 + 2, (height, weeks)
print("  clamp: %dpx for %d week rows" % (height, weeks))
PY

echo "OK: Forecast calendar collapses, filters by day and resizes"
