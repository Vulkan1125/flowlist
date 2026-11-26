#!/bin/sh
# Notification parity probe: the inspector removes and restores a default
# notification, adds an absolute custom notification through the modal, and
# removes it again.
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
box = json.load(open(sys.argv[1]))[sys.argv[2]].get(sys.argv[3])
if box:
    print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
}
# Wait for a probe box to stop moving. The inspector reflows once its fields
# finish painting, so a single read can be one frame too early.
settled_box() {
  prev=""
  stable=0
  for _ in $(seq 1 80); do
    cur=$(box_now "$1" "$2")
    if [ -n "$cur" ] && [ "$cur" = "$prev" ]; then
      stable=$((stable + 1))
    else
      stable=0
    fi
    if [ "$stable" -ge 8 ]; then
      y=${cur#* }
      if [ "$y" -gt 80 ] && [ "$y" -lt 1350 ]; then
        echo "$cur"
        return 0
      fi
    fi
    prev=$cur
    sleep 0.25
  done
  echo "control $1:$2 never settled" >&2
  return 1
}
click_box() {
  set -- $(settled_box "$1" "$2")
  "$helpers/flowctl" mouse $((wx + $1)) $((wy + $2))
  sleep 0.9
}

echo "1. Removing and restoring the default due notification"
click_box notification_boxes notification-default:due
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert "due" in d["removed_notifications"], d; print("   default due removed")'

echo "2. Adding an absolute custom notification"
click_box notification_boxes notification-add-custom
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("modal") == "Custom notification", d.get("modal")
print("   custom modal opened")
PY
printf '%s 10:00' "$tomorrow" | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.2
"$helpers/flowctl" key 36
sleep 1
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert len(d["custom_notifications"])==1 and "10:00" in d["custom_notifications"][0], d; print("   custom notification added")'

echo "3. Removing the custom notification keeps the removed default"
click_box notification_boxes notification-custom-remove:0
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["custom_notifications"]==[], d; assert "due" in d["removed_notifications"], d; print("   custom removed, default due still off")'
click_box notification_boxes notification-default:due
task_json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d["removed_notifications"]==[], d; print("   default due restored")'

echo "notification parity probe: PASS"
