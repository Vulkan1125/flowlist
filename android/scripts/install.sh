#!/bin/sh
# Build and install on the phone connected over USB (or wireless adb).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
export PATH=/opt/homebrew/share/android-commandlinetools/platform-tools:$PATH
"$root/scripts/build.sh" "$@"
adb install -r "$root/app/build/outputs/apk/debug/app-debug.apk"
adb shell am start -n dev.flowlist.android/.MainActivity
