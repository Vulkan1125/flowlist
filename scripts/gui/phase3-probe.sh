#!/bin/sh
# Phase 3 probe: a repeating task names the next occurrence, and deleting one
# asks whether this occurrence or the whole series goes to Trash.
set -eu
root=/Users/mahendrabasala/Projects/flowlist
bin=$root/target/release/flowlist
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-phase3.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; rm -rf "$tmp"; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
task=$("$bin" --data-dir "$lib" add 'Week 1 review' --repeat 'every week' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "task=$task"

python3 - "$lib/session.json" "$task" <<'PY'
import json, sys
json.dump({
    "view": "inbox",
    "window": [1254.0, 1000.0],
    "inspector": True,
    "selected": [sys.argv[2]],
    "vim_mode": True,
}, open(sys.argv[1], "w"))
PY
FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
for _ in $(seq 1 30); do
  window=$("$helpers/window" "$pid" move 40 20 || true)
  wx=$(printf '%s' "$window" | cut -f2); wy=$(printf '%s' "$window" | cut -f3)
  WX=${wx%.*}; WY=${wy%.*}; WX=${WX%,*}; WY=${WY%,*}
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
  key=$1; want=$2
  for _ in $(seq 1 60); do
    [ "$(value "$key")" = "$want" ] && return 0
    sleep 0.1
  done
  echo "FAIL: $key never became '$want' (got '$(value "$key")')" >&2
  exit 1
}
open_titles() {
  # Inbox is deliberately unscheduled-only, so a dated next occurrence is not
  # in the Inbox view. Check every project-less open task instead.
  "$bin" --data-dir "$lib" list --json | python3 -c 'import json,sys; print([t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"] if t["project"] is None and not t["completed"]])'
}
completed_titles() {
  "$bin" --data-dir "$lib" list --completed --json | python3 -c 'import json,sys; print(sorted(t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]))'
}
trash_titles() {
  "$bin" --data-dir "$lib" list --trash --json | python3 -c 'import json,sys; print(sorted(t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]))'
}
series_id() {
  "$bin" --data-dir "$lib" list "$1" --json | python3 -c 'import json,sys; print([t["repeat_series"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]][0])'
}
open_series_id() {
  "$bin" --data-dir "$lib" list --json | python3 -c 'import json,sys; print([t["repeat_series"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"] if t["project"] is None and not t["completed"]][0])'
}
expect() {
  if [ "$2" != "$3" ]; then echo "FAIL: $1: got '$2', wanted '$3'" >&2; exit 1; fi
  echo "  $1: $2"
}

echo "1. Completing a repeating task names the next occurrence"
"$helpers/flowctl" mouse $((WX + 700)) $((WY + 120)); sleep 0.5
"$helpers/flowctl" key 49; sleep 1.2
expect "open list has the next occurrence" "$(open_titles)" "['Week 2 review']"
expect "completed history keeps the first" "$(completed_titles)" "['Week 1 review']"
expect "next occurrence shares the series" "$(open_series_id)" "$task"
expect "completed occurrence shares the series" "$(series_id --completed)" "$task"

echo "2. Delete defaults to this occurrence"
"$helpers/flowctl" key 38; sleep 0.3
"$helpers/flowctl" key 51; sleep 0.6
wait_value modal "Delete repeating task"
expect "default choice" "$(value delete_series)" "False"
"$helpers/flowctl" key 36; sleep 1.0
expect "only the open occurrence is trashed" "$(trash_titles)" "['Week 2 review']"
expect "completed history is untouched" "$(completed_titles)" "['Week 1 review']"

"$helpers/flowctl" key 6 cmd; sleep 1.0
expect "undo restores the occurrence" "$(trash_titles)" "[]"
expect "open list is back" "$(open_titles)" "['Week 2 review']"

echo "3. Tab switches to the whole series"
"$helpers/flowctl" key 38; sleep 0.3
"$helpers/flowctl" key 51; sleep 0.6
wait_value modal "Delete repeating task"
"$helpers/flowctl" key 48; sleep 0.3
expect "switched choice" "$(value delete_series)" "True"
"$helpers/flowctl" key 36; sleep 1.0
expect "the whole series is trashed" "$(trash_titles)" "['Week 1 review', 'Week 2 review']"
expect "open list is empty" "$(open_titles)" "[]"
expect "completed history is gone" "$(completed_titles)" "[]"

"$helpers/flowctl" key 6 cmd; sleep 1.0
expect "one undo restores the series" "$(trash_titles)" "[]"
expect "inbox is back" "$(open_titles)" "['Week 2 review']"
expect "completed history is back" "$(completed_titles)" "['Week 1 review']"

echo "PHASE 3 PROBE PASSED"
