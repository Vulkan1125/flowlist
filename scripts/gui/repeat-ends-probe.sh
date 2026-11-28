#!/bin/sh
# Repeat Ends probe: the inspector's Ends row switches Never / On date /
# After N. On date opens the calendar and applies the picked day; After N
# steps the count. Every step is checked against the command layer, so the
# test proves the controls write real repeat rules, not just repaint.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-repeat-ends.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
task=$("$bin" --data-dir "$lib" add "Repeat ends probe" --repeat "every week" --json |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "task=$task"

rm -f "$state"
python3 - "$lib/session.json" "inbox" "$task" <<'PY'
import json, sys
json.dump(
    {
        "view": sys.argv[2],
        "selected": [sys.argv[3]],
        "window": [1254.0, 950.0],
        "inspector": True,
        "vim_mode": False,
    },
    open(sys.argv[1], "w"),
)
PY
FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
wx=0
wy=0
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
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" 2>/dev/null || true

task_json() {
  "$bin" --data-dir "$lib" list --inbox --json |
    python3 -c 'import json,sys; print(json.dumps([x for g in json.load(sys.stdin)["groups"] for x in g["tasks"] if x["id"]==sys.argv[1]][0]))' "$task"
}
box_point() {
  python3 - "$state" "$1" "$2" <<'PY'
import json, sys
box = json.load(open(sys.argv[1])).get(sys.argv[2], {}).get(sys.argv[3])
if box:
    print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
}
click_box() {
  map=$1
  key=$2
  for _ in $(seq 1 80); do
    point=$(box_point "$map" "$key")
    if [ -n "$point" ]; then
      set -- $point
      "$helpers/flowctl" mouse $((wx + $1)) $((wy + $2))
      sleep 0.5
      # A hover move forces the next frame, so freshly revealed controls are
      # written to the probe state before we read them.
      "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
      sleep 0.3
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: control $map:$key never appeared" >&2
  exit 1
}
wait_repeat() {
  script=$1
  for _ in $(seq 1 80); do
    value=$(task_json)
    if python3 - "$value" "$script" <<'PY'
import json, sys
data = json.loads(sys.argv[1])
sys.exit(0 if eval(sys.argv[2], {"d": data}) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: repeat never matched $script (got $value)" >&2
  exit 1
}
wait_state() {
  field=$1
  script=$2
  for _ in $(seq 1 80); do
    if python3 - "$state" "$field" "$script" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
value = data.get(sys.argv[2])
sys.exit(0 if eval(sys.argv[3], {"d": value}) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: state.$field never matched $script" >&2
  exit 1
}

echo "1. The Ends row starts on Never"
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "task:repeat" in d.get("inspector_section_boxes", {}), d.get("inspector_section_boxes")
PY
# A task that already repeats opens its section; one that does not needs the
# header clicked. Never click a section that is already open, or it closes and
# the probe map keeps stale boxes.
already_open=$(python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
print("1" if "repeat-end-toggle" in d.get("notification_boxes", {}) else "0")
PY
)
if [ "$already_open" = "0" ]; then
  click_box inspector_section_boxes task:repeat
fi
for _ in $(seq 1 80); do
  if [ -n "$(box_point notification_boxes repeat-end-toggle)" ]; then break; fi
  "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
  sleep 0.1
done
wait_repeat 'd["repeat"] == "every week"'
echo "   Never selected, no end in the rule"

echo "2. On date opens the calendar and applies the picked day"
click_box notification_boxes repeat-end-toggle
click_box notification_boxes repeat-end-option:date
wait_repeat 'd["repeat"].startswith("every week until ")'
click_box theme_boxes repeat-end-date
wait_state date_picker_open 'd == "repeat-end"'
day=$(python3 - <<'PY'
import datetime
today = datetime.date.today()
next_day = today + datetime.timedelta(days=1)
print((next_day if next_day.month == today.month else today - datetime.timedelta(days=1)).isoformat())
PY
)
click_box date_picker_boxes "day:$day"
wait_state date_picker_open 'd is None'
wait_repeat "d[\"repeat\"].endswith(\"until $day\")"
echo "   calendar picked $day and the rule ends there"

echo "3. After N times accepts a typed count"
click_box notification_boxes repeat-end-toggle
click_box notification_boxes repeat-end-option:count
wait_repeat '"for 3 times" in d["repeat"]'
click_box theme_boxes field-box:repeat-end-count
"$helpers/flowctl" key 0 cmd
printf '4' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.3
"$helpers/flowctl" key 36
sleep 0.5
wait_repeat '"for 4 times" in d["repeat"]'
click_box theme_boxes field-box:repeat-end-count
"$helpers/flowctl" key 0 cmd
printf '2' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.3
"$helpers/flowctl" key 36
sleep 0.5
wait_repeat '"for 2 times" in d["repeat"]'
echo "   count went 3 -> 4 -> 2 from the input box"

echo "4. Never clears the end again"
click_box notification_boxes repeat-end-toggle
click_box notification_boxes repeat-end-option:never
wait_repeat 'd["repeat"] == "every week"'
echo "   end cleared"

echo "PASS: repeat Ends controls write Never, On date and After N times"
