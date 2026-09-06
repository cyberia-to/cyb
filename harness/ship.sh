#!/bin/bash
# ship — every version becomes a GitHub release, the erga way.
#
# One command: bump the version, commit through the fleet gate, tag,
# build the dmg AND the signed APK from the clean tagged tree (a release
# never wears the dirty star), publish to GitHub with both attached,
# install locally. One binary, every body: apple silicon + android in
# every release; linux joins via CYB_LINUX_HOST (a build node reachable
# over ssh) when one is on the wire.
#
#   make ship                       # bump minor, title = last commit line
#   make ship V=0.3.0 T="headline"  # explicit version and title
#   N="extra notes" make ship       # prepended to the auto notes
#
# Notes are the commit log since the previous tag — the release IS the
# changelog, nothing to write twice.

set -euo pipefail
cd "$(dirname "$0")/.."
# apksigner is a java program; gradle found its jdk through the Makefile,
# the signer gets the same one.
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$PATH"

# A release is a statement about a tree, so the tree must be fully told.
if [ -n "$(git status --porcelain)" ]; then
  echo "ship: tree is dirty — commit or stash first"; git status --short | head; exit 1
fi

CUR=$(grep -m1 '^version' shell/Cargo.toml | sed 's/.*"\(.*\)"/\1/')
if [ "${V:-auto}" = "auto" ]; then
  V=$(echo "$CUR" | awk -F. '{printf "%d.%d.0", $1, $2 + 1}')
fi
TAG="v$V"
TITLE="${T:-$(git log -1 --pretty=%s)}"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "ship: $TAG already exists"; exit 1
fi

echo "ship: $CUR -> $V  ($TITLE)"

# ── bump + commit (the pre-commit fleet gate runs here) ─────────────────
# Idempotent: a ship that died after the bump resumes without re-bumping.
if [ "$V" != "$CUR" ]; then
  sed -i '' "s/^version = \"$CUR\"/version = \"$V\"/" shell/Cargo.toml
  T_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
  RUSTC="$T_BIN/rustc" "$T_BIN/cargo" check -p cyb >/dev/null 2>&1 || true # refresh lock
  git add shell/Cargo.toml Cargo.lock
  git commit -m "cyb $V"
else
  echo "ship: version already $V - resuming"
fi

# ── build the artifacts from the tagged, clean tree ─────────────────────
make dmg
DMG="target/release/cyb-$V.dmg"
cp target/release/cyb.dmg "$DMG"
ASSETS=("$DMG")

# Android: build, align, sign with the local release key. The keystore is
# self-signed and lives outside the repo (~/.cyb-release.keystore) — same
# trust model as the unsigned dmg, but installable on a phone.
KS="$HOME/.cyb-release.keystore"
if [ -f "$KS" ]; then
  echo "ship: building the android body..."
  make android
  BT="$HOME/Library/Android/sdk/build-tools/34.0.0"
  RAW="shell/gen/android/app/build/outputs/apk/release/app-release-unsigned.apk"
  APK="target/release/cyb-$V.apk"
  "$BT/zipalign" -f 4 "$RAW" "$APK.aligned"
  "$BT/apksigner" sign --ks "$KS" --ks-key-alias cyb \
    --ks-pass "file:$HOME/.cyb-release.keystore.pass" \
    --out "$APK" "$APK.aligned"
  rm -f "$APK.aligned" "$APK.idsig"
  "$BT/apksigner" verify "$APK"
  echo "ship: apk signed and verified"
  ASSETS+=("$APK")
else
  echo "ship: no $KS - skipping the android body"
fi

# Linux: built on a real linux machine over ssh when one is configured.
# The workspace spans sibling repos GitHub never sees, so CI cannot do
# this — a build node can. CYB_LINUX_HOST=user@host enables it.
if [ -n "${CYB_LINUX_HOST:-}" ]; then
  echo "ship: linux build on $CYB_LINUX_HOST..."
  if bash harness/build-linux.sh "$CYB_LINUX_HOST" "$V"; then
    ASSETS+=("target/release/cyb-$V-linux-x86_64.tar.gz")
  else
    echo "ship: linux build FAILED - shipping without it"
  fi
fi

# Every body built — NOW the version is real.
git tag -a "$TAG" -m "cyb $V — $TITLE"

# ── notes: what actually changed since the last release ─────────────────
PREV=$(git describe --tags --abbrev=0 "$TAG"^ 2>/dev/null || echo "")
NOTES_FILE=$(mktemp)
{
  [ -n "${N:-}" ] && printf '%s\n\n' "$N"
  echo "## changes"
  if [ -n "$PREV" ]; then
    git log --pretty='- %s' "$PREV".."$TAG" | grep -v '^- cyb [0-9]'
  else
    git log --pretty='- %s' -20
  fi
  echo
  echo "**macOS (apple silicon)**: mount the dmg, then"
  echo '```'
  echo "xattr -cr /Applications/cyb.app && codesign --force --deep -s - /Applications/cyb.app"
  echo '```'
  echo "**android**: \\`adb install cyb-$V.apk\\` (self-signed; allow unknown sources)"
} > "$NOTES_FILE"

# ── publish ─────────────────────────────────────────────────────────────
git push origin master --tags
gh release create "$TAG" "${ASSETS[@]}" \
  --title "cyb $V — $TITLE" \
  --notes-file "$NOTES_FILE"
rm -f "$NOTES_FILE"

# ── and run what we shipped ─────────────────────────────────────────────
rm -rf ~/Applications/cyb.app /Applications/cyb.app
cp -R target/release/cyb.app ~/Applications/cyb.app
cp -R target/release/cyb.app /Applications/cyb.app
xattr -cr ~/Applications/cyb.app /Applications/cyb.app
codesign --force --deep -s - ~/Applications/cyb.app 2>/dev/null || true
codesign --force --deep -s - /Applications/cyb.app 2>/dev/null || true

echo "ship: cyb $V is live — $(gh release view "$TAG" --json url -q .url)"
