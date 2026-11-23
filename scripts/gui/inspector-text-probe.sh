#!/bin/sh
# Inspector text probe: the add-subtask field and the wrapping title editor
# accept mouse placement, a double-click selects a word, and a drag paints a
# character selection. The caret is an overlay, so the text and selection do
# not shift.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-inspector-text.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
title="A deliberately long inspector title that needs two lines so the caret stays visible while editing"
task=$("$bin" --data-dir "$lib" add "$title" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
python3 - "$lib/session.json" "$task" <<'PY'
import json, sys
json.dump(
    {
        "view": "inbox",
        "selected": [sys.argv[2]],
        "inspector": True,
        "window": [1254.0, 1000.0],
    },
    open(sys.argv[1], "w"),
)
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
wait_state() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get(sys.argv[2]) else 1)
PY
    then
      return 0
    fi
    sleep 0.1
  done
  echo "FAIL: $1 never appeared" >&2
  exit 1
}
wait_field() {
  for _ in $(seq 1 60); do
    if [ "$(value field)" = "Title" ]; then return 0; fi
    sleep 0.1
  done
  echo "FAIL: title editor did not open (field=$(value field))" >&2
  exit 1
}
wait_editor_box() {
  for i in $(seq 1 80); do
    if [ -n "$(value field_editor)" ]; then return 0; fi
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.1
  done
  echo "FAIL: title editor box never painted" >&2
  exit 1
}
box_xy() {
  python3 -c 'import ast,sys; box=ast.literal_eval(sys.argv[1]); print(int(box[0]), int(box[1]))' "$1"
}

echo "1. The add-subtask field takes a click and selects a word"
wait_state subtask_add
add=$(value subtask_add)
set -- $(box_xy "$add")
AX=$1; AY=$2
"$helpers/flowctl" mouse $((WX + AX + 40)) $((WY + AY + 14))
sleep 0.4
if [ "$(value focus)" != "Checklist" ]; then
  echo "FAIL: add-subtask field did not take focus ($(value focus))" >&2
  exit 1
fi
for key in 46 34 37 40; do
  "$helpers/flowctl" key "$key"
  sleep 0.08
done
sleep 0.3
"$helpers/flowctl" dblclick $((WX + AX + 23)) $((WY + AY + 14))
sleep 0.3
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
if state["checklist_text"] != "milk":
    raise SystemExit(f"FAIL: add-subtask text was {state['checklist_text']!r}")
start, end = state["checklist_selection"]
word = state["checklist_text"][start:end]
if word != "milk":
    raise SystemExit(f"FAIL: add-field double-click selected {word!r}")
print(f"  selected word {word!r} in the add field")
PY
"$helpers/flowctl" key 53
sleep 0.3

echo "2. A click places the caret inside the edited title"
"$helpers/flowctl" key 14
wait_field
"$helpers/flowctl" text " "
wait_editor_box
box=$(value field_editor)
set -- $(box_xy "$box")
EX=$1; EY=$2
CLICK_X=$((WX + EX + 26))
CLICK_Y=$((WY + EY + 12))
"$helpers/flowctl" mouse "$CLICK_X" "$CLICK_Y"
sleep 0.3
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
cursor = state["field_cursor"]
length = len(state["field_text"])
if not 0 < cursor < length:
    raise SystemExit(f"FAIL: click left the caret at {cursor} of {length}")
print(f"  caret at character {cursor} of {length}")
PY

echo "3. A double-click selects the word under the pointer"
"$helpers/flowctl" dblclick "$CLICK_X" "$CLICK_Y"
sleep 0.3
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
start, end = state["field_selection"]
word = state["field_text"][start:end]
if start == end:
    raise SystemExit("FAIL: double-click did not select anything")
if any(ch.isspace() for ch in word):
    raise SystemExit(f"FAIL: double-click selected across whitespace: {word!r}")
print(f"  selected word {word!r}")
PY

echo "4. A drag paints a selection across the title"
"$helpers/flowctl" mouse "$CLICK_X" "$CLICK_Y"
sleep 0.2
"$helpers/flowctl" drag "$CLICK_X" "$CLICK_Y" $((CLICK_X + 150)) "$CLICK_Y"
sleep 0.3
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
start, end = state["field_selection"]
if end - start < 8:
    raise SystemExit(f"FAIL: drag selected only {end - start} characters")
print(f"  dragged a {end - start}-character selection")
PY

echo "inspector text probe: PASS"
