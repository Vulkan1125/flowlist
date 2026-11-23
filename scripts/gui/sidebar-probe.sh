#!/bin/sh
# Sidebar and navigation probe: folders launch folded, rail sections fold, the
# title-bar arrows walk view history, and Settings controls what the rail shows
# and counts.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-sidebar.XXXXXX")
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
project=$("$bin" --data-dir "$lib" project add 'Client handoff' --folder 'Work' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
tag=$("$bin" --data-dir "$lib" tag add 'Focus' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
busy_tag=$("$bin" --data-dir "$lib" tag add 'Busy' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
"$bin" --data-dir "$lib" add 'Draft the notes' --project 'Client handoff' --tag 'Focus' >/dev/null
"$bin" --data-dir "$lib" add 'Busy one' --tag 'Busy' >/dev/null
"$bin" --data-dir "$lib" add 'Busy two' --tag 'Busy' >/dev/null
# A project whose work is all ticked should not inflate the folder badge.
ticked=$("$bin" --data-dir "$lib" project add 'Ticked off' --folder 'Work' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
ticked_task=$("$bin" --data-dir "$lib" add 'Finished work' --project 'Ticked off' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
"$bin" --data-dir "$lib" done "$ticked_task" >/dev/null
"$bin" --data-dir "$lib" add 'Inbox note' >/dev/null
"$bin" --data-dir "$lib" add 'Due today' due:today >/dev/null
# A saved perspective with work and one that matches nothing exercise the
# count badge and the bare tick that replaces it when the list is empty.
filled=$("$bin" --data-dir "$lib" perspective add 'Handoff work' --filter "{\"type\":\"condition\",\"condition\":{\"type\":\"project\",\"id\":\"$project\"}}" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
empty=$("$bin" --data-dir "$lib" perspective add 'Nothing here' --filter '{"type":"condition","condition":{"type":"text","value":"nothing-matches-this"}}' --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "folder=$folder project=$project ticked=$ticked tag=$tag busy_tag=$busy_tag filled=$filled empty=$empty"

launch() {
  rm -f "$state"
  python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({"view": "inbox", "window": [1254.0, 1000.0], "inspector": False}, open(sys.argv[1], "w"))
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
# nudge the pointer to force that frame.
wait_map() {
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
wait_count() {
  for _ in $(seq 1 60); do
    if python3 - "$state" "$1" "$2" <<'PY'
import json, sys
try:
    value = json.load(open(sys.argv[1])).get("sidebar_count_values", {})
except (OSError, ValueError):
    value = {}
sys.exit(0 if str(value.get(sys.argv[2])) == sys.argv[3] else 1)
PY
    then
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: count '$1' is not $2" >&2
  exit 1
}
wait_above() {
  for _ in $(seq 1 60); do
    if python3 - "$state" "$1" "$2" "$3" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get(sys.argv[2], {})
except (OSError, ValueError):
    boxes = {}
upper = boxes.get(sys.argv[3])
lower = boxes.get(sys.argv[4])
sys.exit(0 if upper and lower and upper[1] < lower[1] else 1)
PY
    then
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: '$2' does not sit above '$3' in $1" >&2
  exit 1
}
wait_no_map() {
  map=$1; key=$2
  for _ in $(seq 1 60); do
    if python3 - "$state" "$map" "$key" <<'PY'
import json, sys
try:
    boxes = json.load(open(sys.argv[1])).get(sys.argv[2], {})
except (OSError, ValueError):
    boxes = {}
sys.exit(0 if sys.argv[3] not in boxes else 1)
PY
    then
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: $map still has a box for '$key'" >&2
  exit 1
}
click_map() {
  map=$1; key=$2
  wait_map "$map" "$key"
  # A box measured before the last reflow can still be in the state file for a
  # frame. Nudge and wait until two reads agree before aiming the pointer.
  prev=""
  for _ in $(seq 1 20); do
    box=$(python3 - "$state" "$map" "$key" <<'PY'
import json, sys
try:
    b = json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]
except (OSError, ValueError, KeyError):
    b = []
print(",".join(str(v) for v in b))
PY
)
    if [ -n "$box" ] && [ "$box" = "$prev" ]; then break; fi
    prev=$box
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.15
  done
  python3 - "$state" "$map" "$key" <<'PY' >"$tmp/pt"
import json, sys
b = json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
  set -- $(cat "$tmp/pt"); RX=$1; RY=$2
  "$helpers/flowctl" mouse $((WX + RX)) $((WY + RY)); sleep 0.6
}
open_sidebar_group() {
  click_map settings_toggle_rows "sidebar-group-$1"
  # The rows inside the accordion prepaint on the next frame. A scroll makes
  # that frame happen immediately so the following click can find them.
  "$helpers/flowctl" scroll $((WX + 600)) $((WY + 600)) 1
  sleep 0.4
}
wait_value() {
  for _ in $(seq 1 60); do
    [ "$(value "$1")" = "$2" ] && return 0
    sleep 0.15
  done
  echo "FAIL: $1 is '$(value "$1")', wanted '$2'" >&2
  exit 1
}
wait_flag() {
  for _ in $(seq 1 60); do
    [ "$(value "$1" | tr '[:upper:]' '[:lower:]')" = "$2" ] && return 0
    sleep 0.15
  done
  echo "FAIL: $1 is '$(value "$1")', wanted '$2'" >&2
  exit 1
}
# `has <state key> <value>` checks membership in a JSON list.
has() {
  python3 - "$state" "$1" "$2" <<'PY'
import json, sys
try:
    values = json.load(open(sys.argv[1])).get(sys.argv[2], [])
except (OSError, ValueError):
    values = []
sys.exit(0 if sys.argv[3] in values else 1)
PY
}
lacks() {
  ! has "$1" "$2"
}
wait_has() {
  for _ in $(seq 1 60); do has "$1" "$2" && return 0; sleep 0.15; done
  echo "FAIL: $1 never contained '$2'" >&2
  exit 1
}
wait_lacks() {
  for _ in $(seq 1 60); do lacks "$1" "$2" && return 0; sleep 0.15; done
  echo "FAIL: $1 still contains '$2'" >&2
  exit 1
}
wait_setting() {
  for _ in $(seq 1 60); do
    [ "$(python3 - "$state" "$1" <<'PY'
import json, sys
value = json.load(open(sys.argv[1])).get("sidebar_settings", {}).get(sys.argv[2], "missing")
print(str(value).lower())
PY
)" = "$2" ] && return 0
    sleep 0.15
  done
  echo "FAIL: sidebar setting $1 is not $2" >&2
  exit 1
}
wait_modal_setting() {
  for _ in $(seq 1 60); do
    [ "$(python3 - "$state" "$1" <<'PY'
import json, sys
value = json.load(open(sys.argv[1])).get("modal_sidebar", {}).get(sys.argv[2], "missing")
print(str(value).lower())
PY
)" = "$2" ] && return 0
    sleep 0.15
  done
  echo "FAIL: sheet sidebar setting $1 is not $2" >&2
  exit 1
}

launch
wait_value view inbox
wait_has sidebar_visible "folder:${folder}"
wait_has sidebar_visible "tag:${tag}"
wait_lacks sidebar_visible "project:${project}"
wait_lacks expanded "folder:${folder}"
# Counts are quiet by default: only the library count is on before anyone
# opens Settings.
wait_setting counts_perspectives false
wait_setting counts_priorities false
wait_setting counts_library true
wait_setting counts_tags false
wait_flag can_go_back false
wait_flag can_go_forward false
# Today keeps its number even with the quiet default, and Perspectives has no
# fold of its own.
wait_map sidebar_counts "builtin:today"
wait_no_map sidebar_counts "builtin:inbox"
wait_no_map sidebar_sections "perspectives"
# The busier tag sits above the quieter one, and a fully ticked project no
# longer inflates its folder's badge.
wait_map sidebar_rows "tag:${busy_tag}"
wait_above sidebar_rows "tag:${busy_tag}" "tag:${tag}"
wait_count "folder:${folder}" 1
echo "1. Folders launch folded, counts stay quiet, tags sort by usage and ticked projects stay out of the folder badge"

# Clicking the folder opens its view and unfolds the tree in one gesture.
click_map sidebar_rows "folder:${folder}"
wait_value view "folder:${folder}"
wait_has expanded "folder:${folder}"
wait_has sidebar_visible "project:${project}"
wait_has sidebar_visible "project:${ticked}"
wait_count "folder:${folder}" 1
echo "2. Opening a folder shows the projects inside it, including the ticked one"

# A second click folds the tree again.
click_map sidebar_rows "folder:${folder}"
wait_lacks sidebar_visible "project:${project}"
echo "3. A second click folds the folder back up"

# Section headings fold their rows without hiding the section itself.
click_map sidebar_sections "tags"
wait_lacks sidebar_visible "tag:${tag}"
wait_has collapsed "section:tags"
echo "4. The Tags heading folds its rows"
click_map sidebar_sections "tags"
wait_has sidebar_visible "tag:${tag}"
echo "5. The same heading opens them again"

# History: Archive then Inbox, then walk back and forward with the arrows.
click_map sidebar_rows "builtin:archive"
wait_value view archive
click_map sidebar_rows "builtin:inbox"
wait_value view inbox
wait_flag can_go_back true
click_map history_rows "history-back"
wait_value view archive
wait_flag can_go_forward true
click_map history_rows "history-forward"
wait_value view inbox
echo "6. The title-bar arrows walk back to Archive and forward again"

# Settings: hide one built-in row, save, and confirm the rail drops just it.
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group perspectives
click_map settings_toggle_rows "sidebar-item-builtin:flagged"
"$helpers/flowctl" key 36
wait_value modal ""
wait_lacks sidebar_visible "builtin:flagged"
echo "7. Settings hides the Flagged row"

# The preference lives in the library, so a relaunch keeps it hidden.
kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
launch
wait_value view inbox
wait_lacks sidebar_visible "builtin:flagged"
echo "8. The sidebar choice survives a relaunch"

# Put the row back, and turn a quiet count on while the sheet is open.
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group perspectives
click_map settings_toggle_rows "sidebar-item-builtin:flagged"
click_map settings_toggle_rows "sidebar-count-builtin:inbox"
"$helpers/flowctl" key 36
wait_value modal ""
wait_has sidebar_visible "builtin:flagged"
wait_map sidebar_counts "builtin:inbox"
wait_count "builtin:inbox" 3
echo "9. A hidden row and a hidden count can both be turned back on"

# Saved perspectives report their count, while an empty one shows a bare tick
# with no count box behind it.
wait_map sidebar_counts "custom:${filled}"
wait_count "custom:${filled}" 1
wait_no_map sidebar_counts "custom:${empty}"
echo "10. Empty saved perspectives keep a bare tick; filled ones show a count"

# The three anchors have no Hide box at all and stay reachable.
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group perspectives
wait_has sidebar_visible "builtin:inbox"
wait_has sidebar_visible "builtin:today"
wait_has sidebar_visible "builtin:trash"
"$helpers/flowctl" key 36
wait_value modal ""
echo "11. Inbox, Today and Trash stay, with no Hide box"

# A count can be hidden while its row stays in place.
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group perspectives
click_map settings_toggle_rows "sidebar-count-builtin:inbox"
"$helpers/flowctl" key 36
wait_value modal ""
# Count boxes accumulate in the live state map, so relaunch to see the rail
# the setting actually produces.
kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
launch
wait_value view inbox
wait_no_map sidebar_counts "builtin:inbox"
wait_has sidebar_visible "builtin:inbox"
echo "12. A count can be hidden while its row stays"

# The matrix is an accordion: opening Priorities reveals just its rows.
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group priorities
click_map settings_toggle_rows "sidebar-item-priority:high"
"$helpers/flowctl" key 36
wait_value modal ""
wait_lacks sidebar_visible "priority:high"
echo "13. Opening the Priorities heading reveals its rows"

"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
click_map settings_toggle_rows "settings-section-sidebar"
wait_value settings_section sidebar
open_sidebar_group priorities
click_map settings_toggle_rows "sidebar-item-priority:high"
"$helpers/flowctl" key 36
wait_value modal ""
wait_has sidebar_visible "priority:high"
echo "14. The priority row comes back"

echo "ALL SIDEBAR CHECKS PASSED"
