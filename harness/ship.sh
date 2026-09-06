#!/bin/bash
# ship — every version becomes a GitHub release, the erga way.
#
# One command: bump the version, commit through the fleet gate, tag,
# build the dmg (stamped with a CLEAN hash — a release never wears the
# dirty star), publish to GitHub with the dmg attached, install locally.
#
#   make ship                       # bump minor, title = last commit line
#   make ship V=0.3.0 T="headline"  # explicit version and title
#   N="extra notes" make ship       # prepended to the auto notes
#
# Notes are the commit log since the previous tag — the release IS the
# changelog, nothing to write twice.

set -euo pipefail
cd "$(dirname "$0")/.."

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
sed -i '' "s/^version = \"$CUR\"/version = \"$V\"/" shell/Cargo.toml
T_BIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
RUSTC="$T_BIN/rustc" "$T_BIN/cargo" check -p cyb >/dev/null 2>&1 || true # refresh lock
git add shell/Cargo.toml Cargo.lock
git commit -m "cyb $V"
git tag -a "$TAG" -m "cyb $V — $TITLE"

# ── build the artifact from the tagged, clean tree ──────────────────────
make dmg
DMG="target/release/cyb-$V.dmg"
cp target/release/cyb.dmg "$DMG"

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
  echo "unsigned build: after mounting, run"
  echo '```'
  echo "xattr -cr /Applications/cyb.app && codesign --force --deep -s - /Applications/cyb.app"
  echo '```'
} > "$NOTES_FILE"

# ── publish ─────────────────────────────────────────────────────────────
git push origin master --tags
gh release create "$TAG" "$DMG" \
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
