#!/bin/sh
# Sign a release APK with a key kept outside the repository, so updates install
# over the previous build. The key and password survive deleting this folder.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
keydir=${FLOWLIST_ANDROID_KEYDIR:-$HOME/.config/flowlist}
store=$keydir/android-release.jks
secret=$keydir/android-release.pass
export JAVA_HOME=${JAVA_HOME:-/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home}

if [ ! -f "$store" ]; then
  mkdir -p "$keydir"
  chmod 700 "$keydir"
  pass=$(openssl rand -hex 24)
  printf '%s' "$pass" > "$secret"
  chmod 600 "$secret"
  keytool -genkeypair \
    -keystore "$store" -storetype PKCS12 \
    -alias flowlist -keyalg RSA -keysize 4096 -validity 10000 \
    -dname "CN=Flowlist, O=Flowlist, C=IN" \
    -storepass "$pass" -keypass "$pass"
  echo "Created a release key at $store"
fi

pass=$(cat "$secret")
cd "$root"
FLOWLIST_ANDROID_STORE=$store \
FLOWLIST_ANDROID_PASSWORD=$pass \
FLOWLIST_ANDROID_ALIAS=flowlist \
  ./gradlew :app:assembleRelease "$@"

mkdir -p "$root/dist"
version=$(sed -n 's/.*versionName = "\(.*\)"/\1/p' "$root/app/build.gradle.kts")
apk=$root/dist/Flowlist-Android-$version.apk
cp "$root/app/build/outputs/apk/release/app-release.apk" "$apk"
printf '\nAPK: %s\n' "$apk"
