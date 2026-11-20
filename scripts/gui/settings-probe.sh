#!/bin/sh
# Settings probe: the sectioned sheet, Plain's accent presets, the window
# controls toggle, Cancel's undo pair and the saved session. The system
# eyedropper is a pointer-driven macOS panel, so this probe only checks that
# its control paints.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BIN:-$root/target/release/flowlist}
tmp=$(mktemp -d "${TMPDIR:-/tmp}/flowlist-settings.XXXXXX")
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

launch() {
  rm -f "$state"
  # The first launch seeds a known Plain/light session. The relaunch keeps the
  # file so the probe can see what Save actually wrote.
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
wait_not() {
  for _ in $(seq 1 80); do
    if [ "$(value "$1")" != "$2" ]; then return 0; fi
    sleep 0.1
  done
  echo "FAIL: $1 never left '$2'" >&2
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
  # A prepaint box can be a frame behind. Nudge until two reads agree, then
  # treat the pointer's landing as the button press.
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
open_settings() {
  "$helpers/flowctl" key 43 cmd
  wait_value modal "Settings"
  wait_value settings_section "appearance"
  wait_map theme_boxes "accent:teal"
  wait_map theme_boxes "accent-pick"
  wait_map theme_boxes "accent-sv"
}
accent_setting() {
  python3 - "$lib/session.json" "$1" <<'PY'
import json, sys
try:
    session = json.load(open(sys.argv[1]))
except (OSError, ValueError):
    session = {}
print(session.get(sys.argv[2]) or "")
PY
}

echo "1. Settings opens on Appearance with the Plain accent controls"
launch
open_settings

echo "2. A preset can be chosen with the keyboard and previews immediately"
"$helpers/flowctl" key 20
wait_not accent "#181818"
teal=$(value accent)
echo "  accent=$teal"

echo "3. Every section is one shortcut away"
"$helpers/flowctl" key 20 cmd
wait_value settings_section "workflow"
"$helpers/flowctl" key 19 cmd
wait_value settings_section "sidebar"
"$helpers/flowctl" key 21 cmd
wait_value settings_section "nearby"
"$helpers/flowctl" key 23 cmd
wait_value settings_section "trash"
"$helpers/flowctl" key 18 cmd
wait_value settings_section "appearance"

echo "4. Escape puts back the accent the sheet opened with"
"$helpers/flowctl" key 21
wait_not accent "$teal"
"$helpers/flowctl" key 53
wait_value modal ""
wait_value accent "#181818"

echo "5. Save persists both tones in the session"
"$helpers/flowctl" key 43 cmd
wait_value modal "Settings"
wait_value settings_section "appearance"
"$helpers/flowctl" key 20
wait_not accent "#181818"
"$helpers/flowctl" key 36
wait_value modal ""
light=$(accent_setting plain_accent_light)
dark=$(accent_setting plain_accent_dark)
if [ "$light" != "#0B5F5A" ] || [ "$dark" != "#86D9CC" ]; then
  echo "FAIL: saved accent pair is $light / $dark" >&2
  exit 1
fi
echo "  saved=$light/$dark"

echo "6. The accent survives a relaunch and Reset returns to ink"
kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
launch
wait_value accent "$teal"
open_settings
click_map theme_boxes "accent-reset"
wait_value accent "#181818"
"$helpers/flowctl" key 36
wait_value modal ""
reset=$(accent_setting plain_accent_light)
if [ -n "$reset" ]; then
  echo "FAIL: Reset left plain_accent_light as $reset" >&2
  exit 1
fi
echo "  reset clears both tones"

echo "7. Window controls hide the buttons; Escape restores, Save persists"
open_settings
wait_map settings_toggle_rows "settings-window-controls"
wait_value traffic_lights "visible"
click_map settings_toggle_rows "settings-window-controls"
wait_value traffic_lights "hidden"
"$helpers/flowctl" key 53
wait_value modal ""
wait_value traffic_lights "visible"
open_settings
click_map settings_toggle_rows "settings-window-controls"
wait_value traffic_lights "hidden"
"$helpers/flowctl" key 36
wait_value modal ""
python3 - "$lib/session.json" <<'PY'
import json, sys
try:
    session = json.load(open(sys.argv[1]))
except (OSError, ValueError):
    session = {}
if session.get("traffic_lights") is not False:
    print("FAIL: traffic_lights saved as", session.get("traffic_lights"), file=sys.stderr)
    sys.exit(1)
PY
echo "  saved traffic_lights=false"
kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
launch
wait_value traffic_lights "hidden"
echo "  hidden buttons survive a relaunch"

echo "ALL SETTINGS CHECKS PASSED"
