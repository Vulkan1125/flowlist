#!/bin/sh
# Archive probe: finished projects get their own sidebar view, grouped by
# folder, with Reopen and Delete actions that are undoable and a calm empty
# state.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-archive.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
folder=$("$bin" --data-dir "$lib" folder add 'Work' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
client=$("$bin" --data-dir "$lib" project add 'Client handoff' --folder 'Work' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
shipped=$("$bin" --data-dir "$lib" project add 'Shipped' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
dropped=$("$bin" --data-dir "$lib" project add 'Old experiment' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
live=$("$bin" --data-dir "$lib" project add 'Still running' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
task=$("$bin" --data-dir "$lib" add 'Publish the notes' --project 'Shipped' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
"$bin" --data-dir "$lib" done "$task" >/dev/null
"$bin" --data-dir "$lib" project status "$client" done >/dev/null
"$bin" --data-dir "$lib" project status "$shipped" done >/dev/null
"$bin" --data-dir "$lib" project status "$dropped" dropped >/dev/null
echo "folder=$folder client=$client shipped=$shipped dropped=$dropped live=$live"

launch() {
  rm -f "$state"
  python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({"view": "archive", "window": [1254.0, 1000.0], "inspector": False}, open(sys.argv[1], "w"))
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
# Prepaint boxes reach the state file the frame after they are measured, so
# nudge the pointer and back it with a harmless Tab to force that frame.
wait_box() {
  for i in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get("archive_rows", {})
except (OSError, ValueError):
    boxes = {}
sys.exit(0 if sys.argv[2] in boxes else 1)
PY
    then
      return 0
    fi
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.1
  done
  echo "FAIL: archive box '$1' never painted" >&2
  exit 1
}
click_box() {
  wait_box "$1"
  python3 - "$state" "$1" <<'PY' >"$tmp/pt"
import json, sys
b = json.load(open(sys.argv[1]))["archive_rows"][sys.argv[2]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
  set -- $(cat "$tmp/pt"); RX=$1; RY=$2
  "$helpers/flowctl" mouse $((WX + RX)) $((WY + RY)); sleep 0.6
}
wait_header_box() {
  for i in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get("history_rows", {})
except (OSError, ValueError):
    boxes = {}
sys.exit(0 if sys.argv[2] in boxes else 1)
PY
    then
      return 0
    fi
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.1
  done
  echo "FAIL: history box '$1' never painted" >&2
  exit 1
}
click_header_box() {
  wait_header_box "$1"
  python3 - "$state" "$1" <<'PY' >"$tmp/pt"
import json, sys
b = json.load(open(sys.argv[1]))["history_rows"][sys.argv[2]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
  set -- $(cat "$tmp/pt"); RX=$1; RY=$2
  "$helpers/flowctl" mouse $((WX + RX)) $((WY + RY)); sleep 0.6
}
wait_archive_ids() {
  for _ in $(seq 1 60); do
    if python3 - "$state" "$@" <<'PY'
import json, sys
try:
    ids = [p["id"] for p in json.load(open(sys.argv[1])).get("archive_projects", [])]
except (OSError, ValueError):
    ids = []
wanted = sys.argv[2:]
sys.exit(0 if ids == wanted else 1)
PY
    then
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: archive is $(value archive_projects), wanted $*" >&2
  exit 1
}
wait_groups() {
  for _ in $(seq 1 60); do
    if python3 - "$state" <<'PY'
import json, sys
try:
    groups = json.load(open(sys.argv[1])).get("archive_groups", [])
except (OSError, ValueError):
    groups = []
titles = [group["title"] for group in groups]
sys.exit(0 if titles == ["Work", "Library"] else 1)
PY
    then
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: archive groups are $(value archive_groups)" >&2
  exit 1
}
wait_value() {
  for _ in $(seq 1 60); do
    [ "$(value "$1")" = "$2" ] && return 0
    sleep 0.15
  done
  echo "FAIL: $1 is '$(value "$1")', wanted '$2'" >&2
  exit 1
}
project_status() {
  python3 - "$lib/data.json" "$1" <<'PY'
import json, sys
project = json.load(open(sys.argv[1]))["db"]["projects"][sys.argv[2]]
print(project["status"])
PY
}

launch

wait_value view archive
wait_archive_ids "$client" "$shipped" "$dropped"
wait_groups
wait_box "project:${client}"
wait_box "reopen:${shipped}"
# Remember the restored three-row layout: after the Undo below, the archive
# should land back on exactly these coordinates.
box_of() {
  python3 - "$state" "$1" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))["archive_rows"].get(sys.argv[2], ""))
PY
}
initial_dropped=$(box_of "project:${dropped}")
echo "1. Archive lists the finished projects, grouped by folder, and leaves active ones out"

python3 - "$state" "$live" <<'PY'
import json, sys
ids = [p["id"] for p in json.load(open(sys.argv[1])).get("archive_projects", [])]
sys.exit(0 if sys.argv[2] not in ids else 1)
PY
echo "2. A live project never appears in the Archive"

# Reopen one project; it leaves the archive and the view stays put.
click_box "reopen:${shipped}"
for _ in $(seq 1 60); do
  [ "$(value view)" = "archive" ] && [ "$(project_status "$shipped")" = "active" ] && break
  sleep 0.15
done
if [ "$(value view)" != "archive" ]; then
  echo "FAIL: Reopen left the Archive for '$(value view)'" >&2
  exit 1
fi
if [ "$(project_status "$shipped")" != "active" ]; then
  echo "FAIL: Reopen did not set the project Active" >&2
  exit 1
fi
wait_archive_ids "$client" "$dropped"
echo "3. Reopen brings a project back without leaving the Archive"

# The change is a normal command, so Undo puts it straight back.
"$helpers/flowctl" key 6 cmd
for _ in $(seq 1 60); do
  [ "$(project_status "$shipped")" = "done" ] && break
  sleep 0.15
done
wait_archive_ids "$client" "$shipped" "$dropped"
for i in $(seq 1 80); do
  [ "$(box_of "project:${dropped}")" = "$initial_dropped" ] && break
  if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
  "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
  sleep 0.1
done
if [ "$(box_of "project:${dropped}")" != "$initial_dropped" ]; then
  echo "FAIL: the archive did not return to its three-row layout after Undo" >&2
  exit 1
fi
echo "4. Undo restores the project to the Archive"

# A row click opens the project dashboard.
click_box "project:${dropped}"
wait_value view "project:${dropped}"
echo "5. A row opens the finished project"

# Delete is a normal, guarded command: the confirmation sheet asks first, and
# Undo puts the project back exactly where it was.
click_header_box "history-back"
wait_value view archive
wait_box "delete:${client}"
click_box "delete:${client}"
wait_value modal "Delete project"
"$helpers/flowctl" key 36
wait_archive_ids "$shipped" "$dropped"
echo "6. Delete removes a finished project after confirmation"
"$helpers/flowctl" key 6 cmd
wait_archive_ids "$client" "$shipped" "$dropped"
wait_groups
echo "7. Undo restores a deleted archive project and its group"

# Second launch, back on the Archive, with every project reopened by the CLI,
# so the calm empty state is what the pane paints.
kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
for id in "$client" "$shipped" "$dropped"; do
  "$bin" --data-dir "$lib" project status "$id" active >/dev/null
done
launch
wait_value view archive
wait_archive_ids
wait_value empty_note "No archived projects"
echo "8. Reopening the last project shows the calm empty state"

echo "ALL ARCHIVE CHECKS PASSED"
