#!/bin/sh
# Phase 2 probe: attachment website links and the folder-delete cascade,
# plus folder dragging.
set -eu
root=/Users/mahendrabasala/Projects/flowlist
bin=$root/target/release/flowlist
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-phase2.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; rm -rf "$tmp"; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
work=$("$bin" --data-dir "$lib" folder add 'Work' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
client=$("$bin" --data-dir "$lib" folder add 'Work/Client' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
inner=$("$bin" --data-dir "$lib" folder add 'Work/Client/Inner' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
personal=$("$bin" --data-dir "$lib" folder add 'Personal' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
site=$("$bin" --data-dir "$lib" project add 'Site' --folder 'Work/Client/Inner' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
task=$("$bin" --data-dir "$lib" add 'Read the brief' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
project_task=$("$bin" --data-dir "$lib" add 'Publish the site' --project 'Site' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "work=$work client=$client inner=$inner personal=$personal site=$site task=$task project_task=$project_task"

python3 - "$lib/session.json" "$task" <<'PY'
import json, sys
json.dump({
    "view": "inbox",
    "window": [1254.0, 1000.0],
    "inspector": True,
    "selected": [sys.argv[2]],
}, open(sys.argv[1], "w"))
PY
FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
for _ in $(seq 1 30); do
  window=$("$helpers/window" "$pid" move 40 20 || true)
  wx=$(printf '%s' "$window" | cut -f2); wy=$(printf '%s' "$window" | cut -f3)
  WX=${wx%.*}; WY=${wy%.*}
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
# Prepaint boxes reach the state file a frame after they are measured, so nudge
# the pointer while waiting to force that frame.
wait_box() {
  map=$1; key=$2
  for i in $(seq 1 80); do
    if python3 - "$state" "$map" "$key" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get(sys.argv[2], {})
except (OSError, ValueError):
    boxes = {}
sys.exit(0 if sys.argv[3] in boxes else 1)
PY
    then return 0; fi
    if [ $((i % 2)) -eq 0 ]; then PX=500; PY=300; else PX=900; PY=520; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.1
  done
  echo "FAIL: $map box '$key' never painted" >&2
  exit 1
}
box_center() {
  map=$1; key=$2
  python3 - "$state" "$map" "$key" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
}
click_box() {
  map=$1; key=$2
  wait_box "$map" "$key"
  set -- $(box_center "$map" "$key"); RX=$1; RY=$2
  "$helpers/flowctl" mouse $((WX + RX)) $((WY + RY)); sleep 0.7
}
drag_boxes() {
  map=$1; from=$2; to=$3
  wait_box "$map" "$from"; wait_box "$map" "$to"
  set -- $(box_center "$map" "$from"); SX=$1; SY=$2
  set -- $(box_center "$map" "$to"); TX=$1; TY=$2
  "$helpers/flowctl" drag $((WX + SX)) $((WY + SY)) $((WX + TX)) $((WY + TY))
  sleep 0.9
}
folder_paths() {
  "$bin" --data-dir "$lib" folder list --json | python3 -c 'import json,sys; print(" | ".join(sorted(f["path"] for f in json.load(sys.stdin))))'
}
expect() {
  if [ "$2" != "$3" ]; then echo "FAIL: $1: got '$2', wanted '$3'" >&2; exit 1; fi
  echo "  $1: $2"
}

echo "1. Attaching a website link from the inspector"
click_box inspector_section_boxes task:notes
click_box view_boxes attach-link
expect "modal title" "$(value modal)" "Attach link"
field_label=$(python3 - "$state" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))["modal_fields"][0]["label"])
PY
)
expect "field label" "$field_label" "Address"
printf 'example.com/brief' | pbcopy
"$helpers/flowctl" key 9 cmd; sleep 0.4
field_text=$(python3 - "$state" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))["modal_fields"][0]["text"])
PY
)
expect "typed address" "$field_text" "example.com/brief"
"$helpers/flowctl" key 36; sleep 0.6
expect "sheet closed" "$(value modal)" ""
attachments=$("$bin" --data-dir "$lib" list --inbox --json | python3 -c 'import json,sys
for group in json.load(sys.stdin)["groups"]:
    for task in group["tasks"]:
        print(task["attachments"])')
expect "stored link" "$attachments" "['https://example.com/brief']"
badge=$("$bin" --data-dir "$lib" list --attachments --json | python3 -c 'import json,sys; print([t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]])')
expect "attachments view" "$badge" "['Read the brief']"

echo "2. Folder rows drag like project rows"
click_box sidebar_toggles "folder:$work"
wait_box sidebar_rows "folder:$client"
drag_boxes sidebar_rows "folder:$client" "folder:$personal"
expect "Client nests under Personal" "$(folder_paths)" "Personal | Personal / Client | Personal / Client / Inner | Work"

wait_box sidebar_rows "folder:$personal"
# Expand Personal so the invalid target (its own child) is reachable.
click_box sidebar_toggles "folder:$personal"
wait_box sidebar_rows "folder:$client"
drag_boxes sidebar_rows "folder:$personal" "folder:$client"
expect "no folder lands inside its own subtree" "$(folder_paths)" "Personal | Personal / Client | Personal / Client / Inner | Work"

wait_box sidebar_rows "folder:$client"
# Mid-drag: the target paints before the mouse is released.
set -- $(box_center sidebar_rows "folder:$client"); SX=$1; SY=$2
set -- $(box_center sidebar_rows library); TX=$1; TY=$2
"$helpers/flowctl" down $((WX + SX)) $((WY + SY)); sleep 0.2
"$helpers/flowctl" dragto $((WX + TX)) $((WY + TY)); sleep 0.5
expect "LIBRARY highlights as a drop target" "$(value sidebar_drag_target)" "library"
"$helpers/flowctl" up $((WX + TX)) $((WY + TY)); sleep 0.9
expect "Client returns to the root" "$(folder_paths)" "Client | Client / Inner | Personal | Work"

"$helpers/flowctl" key 6 cmd; sleep 0.8
expect "one undo restores the nested folder" "$(folder_paths)" "Personal | Personal / Client | Personal / Client / Inner | Work"

echo "3. Deleting a folder takes its tree with it, as one undoable step"
# Personal now holds Client, Inner and Site after the drags above.
click_box sidebar_toggles "folder:$personal"
click_box sidebar_rows "folder:$personal"
"$helpers/flowctl" key 51 cmd; sleep 0.6
expect "delete sheet" "$(value modal)" "Delete folder"
"$helpers/flowctl" key 36; sleep 0.8
expect "folders gone" "$(folder_paths)" "Work"
projects=$("$bin" --data-dir "$lib" project list --json | python3 -c 'import json,sys; print([p["name"] for p in json.load(sys.stdin)])')
expect "projects gone" "$projects" "[]"
trash=$("$bin" --data-dir "$lib" list --trash --json | python3 -c 'import json,sys; print([t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]])')
expect "tasks in Trash" "$trash" "['Publish the site']"

"$helpers/flowctl" key 6 cmd; sleep 0.8
expect "one undo restores the folders" "$(folder_paths)" "Personal | Personal / Client | Personal / Client / Inner | Work"
projects=$("$bin" --data-dir "$lib" project list --json | python3 -c 'import json,sys; print([p["name"] for p in json.load(sys.stdin)])')
expect "project restored" "$projects" "['Site']"
trash=$("$bin" --data-dir "$lib" list --trash --json | python3 -c 'import json,sys; print([t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]])')
expect "task restored" "$trash" "[]"

echo "PHASE 2 PROBE PASSED"
