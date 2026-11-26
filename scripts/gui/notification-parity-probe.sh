#!/bin/sh
# Inspector notification and section probe: the OmniFocus-style Notifications
# section removes and re-adds a default from the Add Notification menu, adds a
# relative alert and a custom notification, moves the full-screen reminder into
# the section, and the disclosure header collapses and expands the section.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-notifications.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
today=$(date +%F)
tomorrow=$(date -v+1d +%F)
project=$("$bin" --data-dir "$lib" project add "Notify project" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
task=$("$bin" --data-dir "$lib" add "Notify task" --project "Notify project" --due "$today 17:00" --estimate 1h --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "project=$project task=$task"

python3 - "$lib/session.json" "$project" "$task" <<'PY'
import json, sys
json.dump(
    {
        "view": "project:" + sys.argv[2],
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
wx=40
wy=40
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
  "$bin" --data-dir "$lib" list --project "Notify project" --json |
    python3 -c 'import json,sys; t=[x for g in json.load(sys.stdin)["groups"] for x in g["tasks"] if x["id"]==sys.argv[1]][0]; print(json.dumps(t))' "$task"
}
box_now() {
  python3 - "$state" "$1" "$2" <<'PY'
import json, sys
box = json.load(open(sys.argv[1])).get(sys.argv[2], {}).get(sys.argv[3])
if box:
    print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
}
settled_box() {
  for _ in $(seq 1 80); do
    cur=$(box_now "$1" "$2")
    if [ -n "$cur" ]; then
      y=${cur#* }
      if [ "$y" -gt 80 ] && [ "$y" -lt 1400 ]; then
        echo "$cur"
        return 0
      fi
    fi
    sleep 0.15
  done
  echo "control $1:$2 never settled" >&2
  return 1
}
click_box() {
  set -- $(settled_box "$1" "$2")
  "$helpers/flowctl" mouse $((wx + $1)) $((wy + $2))
  sleep 0.9
}
wait_for() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get(sys.argv[2]) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: $1 never appeared" >&2
  exit 1
}

echo "1. A default notification is removed and re-added from the menu"
click_box notification_boxes notification-default:due:remove
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert "due" in d["removed_notifications"], d; print("   default due removed")'
click_box notification_boxes notification-add
wait_for notification_menu
click_box notification_boxes notification-add:due
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["removed_notifications"]==[], d; print("   default due re-added")'

echo "2. A relative alert can be added without an Alerts text field"
click_box notification_boxes notification-add
wait_for notification_menu
click_box notification_boxes notification-add:before-due
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("modal") == "Relative notification", d.get("modal")
print("   relative sheet opened")
PY
"$helpers/flowctl" key 36
sleep 0.9
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["alerts_text"]=="due:-15m", d; print("   relative alert added")'

echo "3. A custom notification can be added from the same menu"
click_box notification_boxes notification-add
wait_for notification_menu
click_box notification_boxes notification-add:custom
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("modal") == "Custom notification", d.get("modal")
print("   custom sheet opened")
PY
printf '%s 10:00' "$tomorrow" | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.2
"$helpers/flowctl" key 36
sleep 1
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["custom_notifications"])==1 and "10:00" in d["custom_notifications"][0], d; print("   custom notification added")'

echo "4. Full-screen reminder lives inside Notifications"
click_box notification_boxes notification-fullscreen
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["fullscreen_reminder"] is True, d; print("   full-screen reminder enabled")'
click_box notification_boxes notification-fullscreen
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["fullscreen_reminder"] is False, d; print("   full-screen reminder disabled")'

echo "5. The Notifications header collapses and expands"
wait_collapsed() {
  wanted=$1
  for _ in $(seq 1 80); do
    if python3 - "$state" "$wanted" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
sys.exit(0 if d.get("inspector_sections", {}).get("task:notifications") is (sys.argv[2] == "true") else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: Notifications section did not become $wanted" >&2
  exit 1
}
click_box inspector_section_boxes task:notifications
wait_collapsed true
echo "   collapsed"
click_box inspector_section_boxes task:notifications
wait_collapsed false
echo "   expanded"

echo "6. The relative and custom rows remove from the same list"
click_box notification_boxes notification-alert-remove:0
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["alerts_text"]=="", d; print("   relative alert removed")'
click_box notification_boxes notification-custom-remove:0
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["custom_notifications"]==[], d; print("   custom notification removed")'

echo "notification parity probe: PASS"
