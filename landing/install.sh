#!/bin/sh
# cyb dev preview installer — macOS arm64
# Downloads the latest cyb.dmg and installs cyb.app into /Applications.
# curl downloads carry no quarantine attribute, so Gatekeeper lets the
# (not yet notarized) preview run.
set -eu

URL="https://github.com/cyberia-to/cyb/releases/latest/download/cyb.dmg"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"; hdiutil detach "$TMP/mnt" >/dev/null 2>&1 || true' EXIT

echo "downloading cyb.dmg ..."
curl -fL --progress-bar -o "$TMP/cyb.dmg" "$URL"

echo "installing cyb.app -> /Applications"
mkdir -p "$TMP/mnt"
hdiutil attach "$TMP/cyb.dmg" -mountpoint "$TMP/mnt" -nobrowse -quiet
rm -rf /Applications/cyb.app
cp -R "$TMP/mnt/cyb.app" /Applications/
hdiutil detach "$TMP/mnt" -quiet
xattr -cr /Applications/cyb.app 2>/dev/null || true

echo "done. launch: open /Applications/cyb.app"
