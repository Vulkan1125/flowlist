#!/bin/sh
# Subtask chevron probe: the one-circle gap before the flag is part of the
# expand/collapse target, not empty space.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-subtask-toggle.XXXXXX")
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
"$bin" --data-dir "$lib" add "Child" --parent "$parent" >/dev/null
python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({"view": "inbox", "window": [1254.0, 1000.0]}, open(sys.argv[1], "w"))
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

wait_json() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
sys.exit(0 if eval(sys.argv[2], {"state": state}) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: timed out waiting for $1" >&2
  python3 - "$state" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
print({k: state.get(k) for k in ("task_disclosures", "expanded", "collapsed")})
PY
  exit 1
}

wait_json "state['task_disclosures'].get('$parent')"
box=$(python3 - "$state" "$parent" <<'PY'
import json, sys
state = json.load(open(sys.argv[1]))
print(json.dumps(state["task_disclosures"][sys.argv[2]]))
PY
)
x=$(python3 -c 'import json,sys; b=json.loads(sys.argv[1]); print(int(b[0] + b[2] - 3))' "$box")
y=$(python3 -c 'import json,sys; b=json.loads(sys.argv[1]); print(int(b[1] + b[3] / 2))' "$box")
key="task:$parent"

echo "1. Clicking the gap after the chevron expands the branch"
"$helpers/flowctl" mouse $((WX + x)) $((WY + y))
wait_json "'$key' in state.get('expanded', [])"
echo "  expanded from a click in the padded gap"

echo "2. Clicking the same gap collapses it again"
"$helpers/flowctl" mouse $((WX + x)) $((WY + y))
wait_json "'$key' not in state.get('expanded', [])"
echo "  collapsed from the same padded gap"

echo "subtask toggle probe: PASS"
