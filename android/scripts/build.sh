#!/bin/sh
# Build the debug APK. The release key lives outside this folder and is only
# needed by release.sh.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
export JAVA_HOME=${JAVA_HOME:-/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home}
cd "$root"
./gradlew :app:assembleDebug "$@"
printf '\nAPK: %s\n' "$root/app/build/outputs/apk/debug/app-debug.apk"
