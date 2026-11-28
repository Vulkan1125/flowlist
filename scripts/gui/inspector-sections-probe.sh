#!/bin/sh
# Inspector sections probe: the Repeat section uses OmniFocus-style controls,
# the Review section edits the next review and cadence, and section disclosure
# headers persist their state.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-sections.XXXXXX")
lib=$tmp/lib
helpers=$tmp/bin
mkdir -p "$lib" "$helpers"
pid=""
cleanup() { [ -n "$pid" ] && kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
swiftc -O "$root/scripts/gui/flowctl.swift" -o "$helpers/flowctl"
swiftc -O "$root/scripts/gui/window.swift" -o "$helpers/window"

state=$tmp/state.json
task=$("$bin" --data-dir "$lib" add "Section task" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
project=$("$bin" --data-dir "$lib" project add "Section project" --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "task=$task project=$project"

launch() {
  view=$1
  selected=$2
  rm -f "$state"
  python3 - "$lib/session.json" "$view" "$selected" <<'PY'
import json, sys
json.dump(
    {
        "view": sys.argv[2],
        "selected": [] if sys.argv[3] == "-" else [sys.argv[3]],
        "window": [1254.0, 950.0],
        "inspector": True,
        "vim_mode": False,
    },
    open(sys.argv[1], "w"),
)
PY
  FLOWLIST_TEST_STATE="$state" "$bin" --data-dir "$lib" gui >"$tmp/out.log" 2>"$tmp/err.log" &
  pid=$!
  for _ in $(seq 1 100); do [ -s "$state" ] && break; sleep 0.2; done
  for _ in $(seq 1 30); do
    window=$("$helpers/window" "$pid" move 40 20 || true)
    rawx=$(printf '%s' "$window" | cut -f2)
    rawy=$(printf '%s' "$window" | cut -f3)
    wx=${rawx%.*}
    wy=${rawy%.*}
    if [ -n "$wx" ] && [ -n "$wy" ] && [ "$wx" -ge 0 ] && [ "$wx" -lt 1500 ] && [ "$wy" -ge 0 ] && [ "$wy" -lt 500 ]; then break; fi
    sleep 0.3
  done
  echo "window=$window"
  sleep 1
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" 2>/dev/null || true
}

task_json() {
  "$bin" --data-dir "$lib" list --json |
    python3 -c 'import json,sys; t=[x for g in json.load(sys.stdin)["groups"] for x in g["tasks"] if x["id"]==sys.argv[1]][0]; print(json.dumps(t))' "$task"
}
project_json() {
  "$bin" --data-dir "$lib" project list --json |
    python3 -c 'import json,sys; p=[x for x in json.load(sys.stdin) if x["id"]==sys.argv[1]][0]; print(json.dumps(p))' "$project"
}
box_now() {
  python3 - "$state" "$1" "$2" <<'PY'
import json, sys
box = json.load(open(sys.argv[1])).get(sys.argv[2], {}).get(sys.argv[3])
if box:
    print(int(box[0] + box[2] / 2), int(box[1] + box[3] / 2))
PY
}
click_box() {
  for _ in $(seq 1 80); do
    point=$(box_now "$1" "$2")
    if [ -n "$point" ]; then
      set -- $point
      "$helpers/flowctl" mouse $((wx + $1)) $((wy + $2))
      sleep 0.5
      # A hover move forces the next frame, so freshly revealed controls are
      # written to the probe state before we read them.
      "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
      sleep 0.3
      return 0
    fi
    sleep 0.15
  done
  echo "FAIL: control $1:$2 never appeared" >&2
  exit 1
}
wait_json() {
  name=$1
  field=$2
  script=$3
  for _ in $(seq 1 80); do
    if [ "$name" = task ]; then value=$(task_json); else value=$(project_json); fi
    if python3 - "$value" "$field" "$script" <<'PY'
import json, sys
data = json.loads(sys.argv[1])
sys.exit(0 if eval(sys.argv[3], {"d": data}) else 1)
PY
    then return 0; fi
    sleep 0.1
  done
  echo "FAIL: $name.$field never matched $script (got $value)" >&2
  exit 1
}

echo "1. Repeat starts collapsed and expands from its header"
launch "inbox" "$task"
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "task:repeat" in d.get("inspector_section_boxes", {}), d.get("inspector_section_boxes")
assert "repeat-toggle" not in d.get("theme_boxes", {})
PY
click_box inspector_section_boxes task:repeat
for _ in $(seq 1 80); do
  if [ -n "$(box_now theme_boxes repeat-toggle)" ]; then break; fi
  sleep 0.1
done
echo "   expanded"

echo "2. The repeat controls build a rule"
click_box theme_boxes repeat-toggle
wait_json task repeat 'd["repeat"] == "every week"'
click_box theme_boxes field-box:repeat-every
"$helpers/flowctl" key 0 cmd
printf '2' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.3
"$helpers/flowctl" key 36
sleep 0.5
wait_json task repeat_every 'd["repeat"].startswith("every 2 weeks")'
click_box notification_boxes repeat-unit-toggle
click_box notification_boxes repeat-unit-option:months
wait_json task repeat_unit 'd["repeat"].startswith("every 2 months")'
click_box notification_boxes repeat-pattern-toggle
click_box notification_boxes repeat-pattern-option:third
wait_json task repeat_pattern '"the third Monday of every 2 months" in d["repeat"]'
click_box notification_boxes repeat-pattern-toggle
click_box notification_boxes repeat-pattern-option:last
wait_json task repeat_weekend '"the last weekend day of every 2 months" in d["repeat"]'
click_box notification_boxes repeat-pattern-toggle
click_box notification_boxes repeat-pattern-option:none
wait_json task repeat_pattern_cleared 'd["repeat"].startswith("every 2 months")'
echo "   monthly pattern picks an nth weekday and the last weekend day"
click_box notification_boxes repeat-unit-toggle
click_box notification_boxes repeat-unit-option:days
for _ in $(seq 1 80); do
  if [ -n "$(box_now theme_boxes repeat-weekday:1)" ]; then break; fi
  "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
  sleep 0.1
done
click_box theme_boxes repeat-weekday:1
click_box theme_boxes repeat-weekday:3
wait_json task repeat_weekdays '"Monday and Wednesday" in d["repeat"]'
for _ in $(seq 1 80); do
  if [ -n "$(box_now theme_boxes repeat-custom)" ]; then break; fi
  "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
  sleep 0.1
done
echo "   week strip picks Monday and Wednesday; interval shows custom"
click_box theme_boxes repeat-schedule:completion
wait_json task repeat_schedule '"after completion" in d["repeat"]'
click_box theme_boxes repeat-mode:defer
wait_json task repeat_mode '"defer another" in d["repeat"]'
echo "   weekdays, after completion, defer until"

echo "2b. Hourly units need a clock time"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
"$bin" --data-dir "$lib" edit "$task" --due today >/dev/null
launch "today" "$task"
click_box notification_boxes repeat-unit-toggle
click_box notification_boxes repeat-unit-option:hours
for _ in $(seq 1 80); do
  if python3 - "$state" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get("toast") == "Add a time to the due date first" else 1)
PY
  then break; fi
  sleep 0.1
done
wait_json task repeat_rejected '"Monday and Wednesday" in d["repeat"]'
echo "   date-only task rejected with the due-date message"

kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
"$bin" --data-dir "$lib" edit "$task" --due "today 9am" >/dev/null
launch "today" "$task"
click_box notification_boxes repeat-unit-toggle
click_box notification_boxes repeat-unit-option:hours
wait_json task repeat_hours '"every hour" in d["repeat"]'
echo "   timed task accepts an hourly rule"

echo "3. Tags use the OmniFocus chip box"
click_box inspector_section_boxes task:tags
for _ in $(seq 1 80); do
  if python3 - "$state" <<'PY'
import json, sys
boxes = json.load(open(sys.argv[1])).get("theme_boxes", {})
sys.exit(0 if "tag-box" in boxes and "tag-add" in boxes else 1)
PY
  then break; fi
  "$helpers/flowctl" move $((wx + 600)) $((wy + 300))
  sleep 0.1
done
python3 - "$state" <<'PY'
import json, sys
boxes = json.load(open(sys.argv[1]))["theme_boxes"]
box = boxes["tag-box"]
add = boxes["tag-add"]
box_x = box[0] + box[2] / 2
add_x = add[0] + add[2] / 2
assert abs(box_x - add_x) < 5, (box, add)
print("   plus is centred in the empty tag box")
PY
click_box theme_boxes tag-add
printf 'errand' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.3
"$helpers/flowctl" key 36
sleep 0.6
wait_json task tag_added 'd["tags"] == ["errand"]'
echo "   tag added from the chip box"

echo "4. The Review section steps the cadence"
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
launch "project:$project" "-"
for _ in $(seq 1 80); do
  if [ -n "$(box_now theme_boxes review-every-plus)" ]; then break; fi
  sleep 0.1
done
click_box theme_boxes review-every-plus
wait_json project review_days 'd["review_interval_days"] == 14'
echo "   review every 2 weeks"

echo "5. The Review sheet edits Next Review"
click_box theme_boxes project-review-edit
python3 - "$state" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("modal") == "Review", d.get("modal")
PY
for _ in $(seq 1 80); do
  if python3 - "$state" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get("modal_field") else 1)
PY
  then break; fi
  sleep 0.1
done
python3 - "$state" > "$tmp/point" <<'PY'
import json, sys
field = json.load(open(sys.argv[1]))["modal_field"]
print(int(field[0] + field[2] / 2), int(field[1] + field[3] / 2))
PY
set -- $(cat "$tmp/point")
"$helpers/flowctl" mouse $((wx + $1)) $((wy + $2))
sleep 0.3
"$helpers/flowctl" key 0 cmd
printf '2027-01-15' | pbcopy
"$helpers/flowctl" key 9 cmd
sleep 0.2
"$helpers/flowctl" key 36
sleep 0.8
wait_json project next_review 'd["next_review"] == "2027-01-15"'
wait_json project review_days_kept 'd["review_interval_days"] == 14'
echo "   next review set to 2027-01-15"

echo "6. Review section disclosure collapses and expands"
click_box inspector_section_boxes project:review
for _ in $(seq 1 80); do
  if python3 - "$state" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get("inspector_sections", {}).get("project:review") is True else 1)
PY
  then break; fi
  sleep 0.1
done
click_box inspector_section_boxes project:review
for _ in $(seq 1 80); do
  if python3 - "$state" <<'PY'
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get("inspector_sections", {}).get("project:review") is False else 1)
PY
  then break; fi
  sleep 0.1
done
echo "   collapsed and expanded"

echo "inspector sections probe: PASS"
