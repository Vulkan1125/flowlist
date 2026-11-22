#!/bin/sh
# Title editor probe: a long inspector title must wrap while editing, so the
# caret cannot disappear past the edge of a single-line field.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-title.XXXXXX")
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
    if python3 - "$state" <<'PY'
import json, sys
try:
    box = json.load(open(sys.argv[1])).get("field_editor")
except (OSError, ValueError):
    box = None
sys.exit(0 if box else 1)
PY
    then
      return 0
    fi
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.1
  done
  echo "FAIL: title editor box never painted (field=$(value field), editor=$(value field_editor))" >&2
  python3 - "$state" <<'PY2'
import json, sys
state = json.load(open(sys.argv[1]))
print({key: state.get(key) for key in ("field", "focus", "selected", "window", "field_editor")})
PY2
  exit 1
}

"$helpers/flowctl" key 14
wait_field
"$helpers/flowctl" text " "
wait_editor_box
height=$(python3 - "$state" <<'PY'
import json, sys
box = json.load(open(sys.argv[1]))["field_editor"]
print(box[3])
PY
)
python3 - "$height" <<'PY'
import sys
height = float(sys.argv[1])
if height < 40:
    raise SystemExit(f"FAIL: title editor is {height}px tall, so it is still single-line")
print(f"title editor height: {height:.0f}px")
PY
