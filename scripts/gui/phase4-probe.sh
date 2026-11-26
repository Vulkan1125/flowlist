#!/bin/sh
# Phase 4 probe: the floating quick-capture panel.
#
# Opens the panel with the global shortcut, pastes a line of quick-entry
# syntax, and checks that the task lands where it should while the main
# window's view and selection stay put. Then it covers Esc, Shift+Enter,
# Command+Enter and the rule that a plain capture always goes to the Inbox.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=$root/target/release/flowlist
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-phase4.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; rm -rf "$tmp"; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
capture=$tmp/capture.json
project=$("$bin" --data-dir "$lib" --json project add 'Errands' | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "project=$project"

# The panel is a 620x150 card centred on the primary display.
screen=$(osascript -e 'tell application "Finder" to get bounds of window of desktop' 2>/dev/null || true)
width=$(printf '%s' "$screen" | tr ',' '\t' | cut -f3)
height=$(printf '%s' "$screen" | tr ',' '\t' | cut -f4)
[ -n "$width" ] || width=2560
[ -n "$height" ] || height=1440
PANEL_X=$((width / 2))
PANEL_Y=$((height / 2))
echo "panel=($PANEL_X,$PANEL_Y)"

python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({
    "view": "inbox",
    "window": [1254.0, 1000.0],
    "inspector": True,
    "selected": [],
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
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" 2>/dev/null || true
sleep 0.5

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
capture_value() {
  python3 - "$capture" "$1" <<'PY'
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
wait_capture() {
  key=$1; want=$2
  for _ in $(seq 1 60); do
    [ "$(capture_value "$key")" = "$want" ] && return 0
    sleep 0.1
  done
  echo "FAIL: capture $key never became '$want' (got '$(capture_value "$key")')" >&2
  cat "$capture" >&2 || true
  exit 1
}
wait_capture_contains() {
  key=$1; want=$2
  for _ in $(seq 1 60); do
    case "$(capture_value "$key")" in *"$want"*) return 0 ;; esac
    sleep 0.1
  done
  echo "FAIL: capture $key never contained '$want' (got '$(capture_value "$key")')" >&2
  exit 1
}
expect() {
  if [ "$2" != "$3" ]; then echo "FAIL: $1: got '$2', wanted '$3'" >&2; exit 1; fi
  echo "  $1: $2"
}
inbox_titles() {
  "$bin" --data-dir "$lib" list --inbox --json | python3 -c 'import json,sys; print(sorted(t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]))'
}
project_titles() {
  "$bin" --data-dir "$lib" list --project "$1" --json | python3 -c 'import json,sys; print(sorted(t["title"] for g in json.load(sys.stdin)["groups"] for t in g["tasks"]))'
}
task_project() {
  "$bin" --data-dir "$lib" list --project "$1" --json | python3 -c 'import json,sys; rows=[t for g in json.load(sys.stdin)["groups"] for t in g["tasks"] if t["title"]==sys.argv[1]]; print(rows[0]["project"] if rows else "")' "$2"
}
focus_main() { "$helpers/flowctl" mouse $((WX + 700)) $((WY + 200)); sleep 0.4; }
open_panel() {
  focus_main
  "$helpers/flowctl" key 49 ctrlalt
  wait_capture open True
  # This harness does not hand the brand-new panel the keyboard until it is
  # clicked; a real user's shortcut press does. Clicking also proves the
  # panel's hit area is where the window is drawn.
  panel=$("$helpers/window" "$pid" layers | awk -F'\t' '$2 >= 3 { print $3, $4, $5, $6; exit }')
  if [ -n "$panel" ]; then
    set -- $panel
    "$helpers/flowctl" mouse $((${1%.*} + ${3%.*} / 2)) $((${2%.*} + 40))
  else
    "$helpers/flowctl" mouse "$PANEL_X" "$PANEL_Y"
  fi
  sleep 0.4
}
paste() { printf '%s' "$1" | pbcopy; "$helpers/flowctl" key 9 cmd; sleep 0.4; }

echo "1. Enter captures into the view without moving the workspace"
view_before=$(value view); selected_before=$(value selected)
open_panel
paste 'Buy milk'
wait_capture_contains text 'Buy milk'
expect "live preview names the title" "$(capture_value title)" "Buy milk"
expect "footer names the destination" "$(capture_value destination)" "Inbox"
"$helpers/flowctl" key 36; sleep 0.8
wait_capture open False
expect "task is in the inbox" "$(inbox_titles)" "['Buy milk']"
expect "the main view did not move" "$(value view)" "$view_before"
expect "the main selection did not move" "$(value selected)" "$selected_before"

echo "2. Undo from the main window reverses the capture"
focus_main
"$helpers/flowctl" key 6 cmd; sleep 0.8
expect "capture is undone" "$(inbox_titles)" "[]"

echo "3. Escape closes without filing anything"
open_panel
paste 'discard me'
wait_capture_contains text 'discard me'
"$helpers/flowctl" key 53; sleep 0.8
wait_capture open False
expect "nothing was filed" "$(inbox_titles)" "[]"

echo "4. Shift+Enter files and keeps the panel open"
open_panel
paste 'keep open task'
wait_capture_contains text 'keep open task'
"$helpers/flowctl" key 36 shift; sleep 0.8
wait_capture open True
expect "task landed" "$(inbox_titles)" "['keep open task']"
expect "panel cleared its field" "$(capture_value text)" ""
wait_capture_contains flash 'Added to'
expect "flash names the destination" "$(capture_value flash)" "Added to Inbox"
"$helpers/flowctl" key 53; sleep 0.6
wait_capture open False

echo "5. Command+Enter files and reveals the task in the main window"
open_panel
paste 'reveal me #errands'
wait_capture_contains text 'reveal me'
"$helpers/flowctl" key 36 cmd; sleep 1.0
wait_capture open False
wait_value selected 1
expect "task is selected in the list" \
  "$(value content_rows | python3 -c 'import ast,sys; rows=ast.literal_eval(sys.stdin.read()); print(rows[-1]["title"] if rows else "")')" \
  "reveal me"

echo "6. A plain capture ignores the current view and still goes to the Inbox"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
pid=""
python3 - "$lib/session.json" "$project" <<'PY'
import json, sys
session = json.load(open(sys.argv[1]))
session["view"] = "project:" + sys.argv[2]
session["selected"] = []
json.dump(session, open(sys.argv[1], "w"))
PY
FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
pid=$!
for _ in $(seq 1 100); do [ "$(value view)" = "project:$project" ] && break; sleep 0.2; done
window=$("$helpers/window" "$pid" move 40 20 || true)
wx=$(printf '%s' "$window" | cut -f2); wy=$(printf '%s' "$window" | cut -f3)
WX=${wx%.*}; WY=${wy%.*}; WX=${WX%,*}; WY=${WY%,*}
open_panel
paste 'file from project view'
wait_capture_contains text 'file from project view'
expect "footer stays on the Inbox" "$(capture_value destination)" "Inbox"
"$helpers/flowctl" key 36; sleep 0.9
wait_capture open False
if ! inbox_titles | grep -q "file from project view"; then
  echo "FAIL: a plain capture from a project view did not land in the Inbox" >&2
  exit 1
fi
expect "task is not filed in the project" \
  "$(task_project 'Errands' 'file from project view')" ""
expect "the main view stayed on the project" "$(value view)" "project:$project"

echo "PHASE 4 PROBE PASSED"
