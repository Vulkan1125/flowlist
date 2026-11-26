#!/bin/sh
# Calendar events probe: read-only events from the test fixture appear in the
# Forecast agenda and month grid even when the day has no tasks.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-calendar.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump(
    {
        "view": "forecast",
        "calendar_events": True,
        "window": [1254.0, 1000.0],
        "inspector": False,
        "vim_mode": False,
    },
    open(sys.argv[1], "w"),
)
PY
FLOWLIST_TEST_CALENDAR=1 FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
for _ in $(seq 1 30); do
  window=$("$helpers/window" "$pid" move 40 20 || true)
  rawx=$(printf '%s' "$window" | cut -f2)
  rawy=$(printf '%s' "$window" | cut -f3)
  wx=${rawx%.*}
  wy=${rawy%.*}
  if [ -n "$wx" ] && [ -n "$wy" ] && [ "$wx" -ge 0 ] && [ "$wx" -lt 1500 ] && [ "$wy" -ge 0 ] && [ "$wy" -lt 500 ]; then break; fi
  sleep 0.3
done
echo "window=$window"
sleep 1

for _ in $(seq 1 100); do
  if python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
sys.exit(0 if len(d.get("calendar_events", [])) == 3 else 1)
PY
  then break; fi
  "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
  sleep 0.1
done

python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
events = state.get("calendar_events", [])
assert len(events) == 3, events
assert state.get("calendar_events_enabled") is True
assert state.get("calendar_permission") == "Allowed", state.get("calendar_permission")
agenda = state.get("calendar_event_boxes", {})
for key in ("test-standup", "test-holiday", "test-review"):
    assert key in agenda, (key, agenda)
chips = state.get("calendar_chip_boxes", {})
for key in ("test-standup", "test-holiday", "test-review"):
    assert key in chips, (key, chips)
groups = state.get("content_groups", {})
assert "forecast:today" in groups, groups
assert "forecast:tomorrow" in groups, groups
print("  agenda rows and month chips are present")
print("  event-only Today and Tomorrow groups exist")
PY

echo "calendar events probe: PASS"
