#!/bin/sh
# Perspective rules probe: the OmniFocus-style contents tree. A new view starts
# with Availability: Remaining, rules can be added from the catalogue, turned
# off, re-edited and saved. The probe drives the real sheet with the same
# pointer and keyboard events a person would use.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-rules.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
"$bin" --data-dir "$lib" add 'Write the handover notes' >/dev/null
"$bin" --data-dir "$lib" add 'Read the strategy doc' >/dev/null
"$bin" --data-dir "$lib" project add 'Harbour plan' >/dev/null
"$bin" --data-dir "$lib" add 'Draft the brief' -p 'Harbour plan' >/dev/null

launch() {
  rm -f "$state"
  if [ ! -f "$lib/session.json" ]; then
    python3 - "$lib/session.json" <<'PY'
import json, sys
json.dump({"theme": "plain", "appearance": "light", "view": "inbox",
           "window": [1254.0, 1000.0], "inspector": False}, open(sys.argv[1], "w"))
PY
  fi
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
wait_value() {
  for _ in $(seq 1 80); do
    if [ "$(value "$1")" = "$2" ]; then return 0; fi
    sleep 0.1
  done
  echo "FAIL: $1 is '$(value "$1")', wanted '$2'" >&2
  exit 1
}
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
click_map() {
  map=$1; key=$2
  wait_map "$map" "$key"
  prev=""
  for i in $(seq 1 20); do
    point=$(python3 - "$state" "$map" "$key" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))[sys.argv[2]][sys.argv[3]]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
)
    if [ "$point" = "$prev" ]; then break; fi
    prev=$point
    if [ $((i % 2)) -eq 0 ]; then PX=600; PY=300; else PX=900; PY=500; fi
    "$helpers/flowctl" move $((WX + PX)) $((WY + PY))
    sleep 0.15
  done
  "$helpers/flowctl" mouse $((WX + ${point% *})) $((WY + ${point#* }))
  sleep 0.6
}
row_has() {
  expected=$1
  python3 - "$state" "$expected" <<'PY'
import json, sys
form = json.load(open(sys.argv[1])).get("view_form") or {}
rows = form.get("rows") or []
found = any(sys.argv[2] in (row.get("label") or "") for row in rows)
sys.exit(0 if found else 1)
PY
}
count_rows() {
  python3 - "$state" "$1" <<'PY'
import json, sys
rows = (json.load(open(sys.argv[1])).get("view_form") or {}).get("rows") or []
print(sum(1 for row in rows if sys.argv[2] in (row.get("label") or "")))
PY
}
wait_tab() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
form = json.load(open(sys.argv[1])).get("view_form") or {}
sys.exit(0 if form.get("tab") == sys.argv[2] else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: tab is not '$1'" >&2
  exit 1
}
wait_row() {
  for _ in $(seq 1 80); do
    if row_has "$1"; then return 0; fi
    sleep 0.1
  done
  echo "FAIL: no row containing '$1'" >&2
  python3 - "$state" <<'PY'
import json, sys
print(json.dumps((json.load(open(sys.argv[1])).get("view_form") or {}).get("rows"), indent=2))
PY
  exit 1
}
wait_editing() {
  for _ in $(seq 1 80); do
    if python3 - "$state" "$1" <<'PY'
import json, sys
form = json.load(open(sys.argv[1])).get("view_form") or {}
sys.exit(0 if str(form.get("editing")) == sys.argv[2] else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: editing is not '$1'" >&2
  exit 1
}

echo "1. A new perspective starts with Availability: Remaining"
launch
"$helpers/flowctl" key 35 cmdshift
wait_value modal "Perspective"
click_map view_boxes "modal-field:0"
"$helpers/flowctl" text "a"
wait_tab simple
wait_map view_boxes "perspective-resize"
point=$(python3 - "$state" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))["view_boxes"]["perspective-resize"]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
)
"$helpers/flowctl" drag $((WX + ${point% *})) $((WY + ${point#* })) $((WX + ${point% *} + 120)) $((WY + ${point#* } + 200))
sleep 0.5
python3 - "$state" <<'PY'
import json, sys
size = json.load(open(sys.argv[1])).get("perspective_size")
assert size is not None and size[0] > 760 and size[1] > 780, size
PY
click_map view_boxes "status"
wait_value view_menu "status"
click_map view_boxes "option:status:available"
wait_value view_menu ""
click_map view_boxes "modal-flagged"
click_map view_boxes "perspective-tab:comprehensive"
wait_tab comprehensive
wait_row "Is flagged"
wait_row "Availability is Remaining"

echo "2. The catalogue adds a rule beneath the top-level All"
click_map view_boxes "rule-plus:1"
python3 - "$state" <<'PY'
import json, sys
form = json.load(open(sys.argv[1])).get("view_form") or {}
assert form.get("add_menu") == 1, form.get("add_menu")
PY
wait_map view_boxes "rule-option:1:Status--Due-Soon"
point=$(python3 - "$state" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))["view_boxes"]["rule-option:1:Status--Due-Soon"]
print(int(b[0] + b[2] / 2), int(b[1] + b[3] / 2))
PY
)
before=$(value settings_scroll)
"$helpers/flowctl" scroll $((WX + ${point% *})) $((WY + ${point#* })) 8
sleep 0.4
after=$(value settings_scroll)
if [ "$before" != "$after" ]; then
  echo "FAIL: scrolling the rule menu moved the sheet from $before to $after" >&2
  exit 1
fi
if [ "$(value rule_menu_scroll)" = "0" ]; then
  echo "FAIL: the rule menu did not scroll" >&2
  exit 1
fi
"$helpers/flowctl" scroll $((WX + ${point% *})) $((WY + ${point#* })) -8
sleep 0.3
click_map view_boxes "rule-option:1:Status--Due-Soon"
wait_row "Status is Due Soon"
status_id=$(python3 - "$state" <<'PY'
import json, sys
rows = (json.load(open(sys.argv[1])).get("view_form") or {}).get("rows") or []
print(next(row["id"] for row in rows if "Due Soon" in (row.get("label") or "")))
PY
)
click_map view_boxes "rule-plus:$status_id"
click_map view_boxes "rule-option:1:Availability"
if [ "$(count_rows 'Availability is Remaining')" != "2" ]; then
  echo "FAIL: expected two Availability cards" >&2
  exit 1
fi

echo "3. A rule can be turned off and back on"
row_id=$(python3 - "$state" <<'PY'
import json, sys
rows = (json.load(open(sys.argv[1])).get("view_form") or {}).get("rows") or []
print(next(row["id"] for row in rows if "Due Soon" in (row.get("label") or "")))
PY
)
click_map view_boxes "rule-menu:$row_id"
click_map view_boxes "rule-menu-item:$row_id:turn-off"
python3 - "$state" "$row_id" <<'PY'
import json, sys
rows = (json.load(open(sys.argv[1])).get("view_form") or {}).get("rows") or []
row = next(row for row in rows if str(row["id"]) == sys.argv[2])
assert row["enabled"] is False, row
PY
click_map view_boxes "rule-menu:$row_id"
click_map view_boxes "rule-menu-item:$row_id:turn-on"

echo "4. Editing a rule changes its value"
click_map view_boxes "rule-select:$row_id"
wait_editing "$row_id"
click_map view_boxes "rule-state:$row_id:Flagged"
wait_row "Status is Flagged"
click_map view_boxes "rule-close:$row_id"

echo "5. Save writes the tree"
"$helpers/flowctl" key 36
sleep 1
python3 - "$lib" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1]) / "data.json"
data = json.loads(path.read_text())
views = data["db"].get("perspectives") or {}
view = next(
    value for value in views.values()
    if any(
        n.get("type") == "condition"
        and n.get("condition", {}).get("type") == "status_kind"
        and n["condition"].get("value") == "flagged"
        for n in value["filter"].get("nodes", [])
    )
)
nodes = view["filter"]["nodes"]
assert any(n.get("type") == "condition" and n["condition"]["type"] == "status_kind"
           and n["condition"]["value"] == "flagged" for n in nodes), nodes
PY
echo "perspective rules probe passed"
