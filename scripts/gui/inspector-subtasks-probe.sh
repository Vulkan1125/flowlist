#!/bin/sh
# Inspector subtask probe: only the active step mode is listed, every child
# row carries its Separate and Delete controls, sequential children stay
# blocked, switching to one-off warns and converts, and switching back
# restores subtasks.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-inspector-subtasks.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
parent=$("$bin" --data-dir "$lib" add "Parent" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
first=$("$bin" --data-dir "$lib" add "First" --parent "$parent" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
second=$("$bin" --data-dir "$lib" add "Second" --parent "$parent" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
third=$("$bin" --data-dir "$lib" add "Third" --parent "$parent" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "parent=$parent first=$first second=$second third=$third"

python3 - "$lib/session.json" "$parent" <<'PY'
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

click_box() {
  python3 - "$state" "$1" "$WX" "$WY" <<'PY' >"$tmp/click.sh"
import json, sys
state = json.load(open(sys.argv[1]))
key, wx, wy = sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
box = state["inspector_subtasks"].get(key)
if not box:
    raise SystemExit(f"no box for {key}")
x, y, w, h = box
print(f"{wx + int(x + w / 2)} {wy + int(y + h / 2)}")
PY
  set -- $(cat "$tmp/click.sh")
  "$helpers/flowctl" mouse "$1" "$2"
}

wait_json() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
script = sys.argv[2]
state = json.load(open(sys.argv[1]))
sys.exit(0 if eval(script, {"state": state}) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: timed out waiting for $1" >&2
  python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
print({k: state.get(k) for k in ("task_steps", "modal", "toast", "focus")})
PY
  exit 1
}

echo "1. Parallel mode lists only child tasks, and every row has its controls"
wait_json 'state.get("task_steps") and len(state["task_steps"]["children"]) == 3'
python3 - "$state" "$first" "$second" "$third" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
steps = state["task_steps"]
assert not steps["one_off"] and not steps["sequential"]
assert len(steps["children"]) == 3 and not steps["checklist"]
boxes = state["inspector_subtasks"]
for child in sys.argv[2:]:
    for prefix in ("subtask:", "separate:", "delete:"):
        if prefix + child not in boxes:
            raise SystemExit(f"FAIL: {prefix + child} was not laid out")
print("  three parallel rows with Separate and Delete on each")
PY

echo "2. Sequential mode blocks every child after the first"
click_box "step:sequential"
wait_json 'state.get("task_steps") and state["task_steps"]["sequential"]'
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
children = state["task_steps"]["children"]
if children[0]["blocked_by"] is not None:
    raise SystemExit("FAIL: the first child is blocked")
if children[1]["blocked_by"] != "First" or children[2]["blocked_by"] != "First":
    raise SystemExit(f"FAIL: blockers are {[c['blocked_by'] for c in children]}")
print("  first is open; the rest point at First")
PY

echo "3. Ticking a blocked child is refused with the sequential warning"
click_box "subtask:$second"
wait_json '"sequential list" in (state.get("toast") or "")'
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
if state["task_steps"]["children"][1]["completed"]:
    raise SystemExit("FAIL: a blocked child was completed")
print(f"  toast: {state['toast']}")
PY

echo "4. Switching to one-off warns before converting"
click_box "step:one-off"
wait_json 'state.get("modal")'
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
modal = state["modal"] or ""
if "one-off" not in modal.lower():
    raise SystemExit(f"FAIL: wrong warning sheet {modal!r}")
if state["task_steps"]["one_off"]:
    raise SystemExit("FAIL: one-off was applied before confirming")
print(f"  warning: {modal}")
PY
"$helpers/flowctl" key 36
wait_json 'state.get("task_steps") and state["task_steps"]["one_off"]'
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
steps = state["task_steps"]
if steps["children"]:
    raise SystemExit("FAIL: child tasks survived the one-off conversion")
titles = [item["title"] for item in steps["checklist"]]
if titles != ["First", "Second", "Third"]:
    raise SystemExit(f"FAIL: checklist is {titles}")
print("  converted to First/Second/Third")
PY

echo "5. Switching back to Sequential restores subtasks"
click_box "step:sequential"
wait_json 'state.get("task_steps") and not state["task_steps"]["one_off"]'
python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
steps = state["task_steps"]
if steps["checklist"]:
    raise SystemExit("FAIL: one-off steps survived the conversion back")
if not steps["sequential"] or len(steps["children"]) != 3:
    raise SystemExit(f"FAIL: restored state is {steps}")
print("  three subtasks restored, still sequential")
PY

echo "inspector subtasks probe: PASS"
