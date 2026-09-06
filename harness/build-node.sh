#!/bin/bash
# build-node — cyb for ubuntu and windows, built on the cybernode.
#
# The cyb workspace reaches into ~21 sibling repos GitHub never sees, so
# CI cannot build it. deimos (the quietest cybernode: 8 cores, 62G) can:
# this script rsyncs the workspace there, builds linux natively and
# windows via cargo-xwin (MSVC cross, SDK auto-fetched), and brings both
# artifacts home.
#
#   bash harness/build-node.sh <host> <version> [linux|windows|all]
#
# Node prerequisites (bootstrapped once): rustup stable +
# x86_64-pc-windows-msvc target, clang/lld, cargo-xwin, and the bevy apt
# set (alsa, udev, x11, wayland, xkbcommon).
#
# Builds run under nice: the node's day job (bostrom) keeps its cores.

set -euo pipefail
HOST="${1:?host}"; V="${2:?version}"; WHAT="${3:-all}"
cd "$(dirname "$0")/../.."   # ~/cyber — the workspace root

# The exact sibling set comes from cargo metadata (path deps outside cyb).
SIBLINGS=(cyb bbg cybergraph evy foculus glia hemera honeycrisp inf lens
          mir mudra nox nu prysm rune soma strata tape tok tru zheng)

echo "build-node: syncing ${#SIBLINGS[@]} repos to $HOST..."
for repo in "${SIBLINGS[@]}"; do
  [ -d "$repo" ] || { echo "build-node: MISSING $repo"; exit 1; }
  rsync -a --delete --exclude target --exclude .git --exclude gen \
    "$repo" "$HOST:cyber-build/"
done

OUT="cyb/target/release"
mkdir -p "$OUT"

if [ "$WHAT" = "linux" ] || [ "$WHAT" = "all" ]; then
  echo "build-node: linux build..."
  ssh "$HOST" "cd cyber-build/cyb && nice -n 10 ~/.cargo/bin/cargo build --release -p cyb 2>&1 | tail -3"
  ssh "$HOST" "cd cyber-build/cyb/target/release && tar czf cyb-$V-linux-x86_64.tar.gz cyb"
  scp -q "$HOST:cyber-build/cyb/target/release/cyb-$V-linux-x86_64.tar.gz" "$OUT/"
  echo "build-node: linux artifact home"
fi

if [ "$WHAT" = "windows" ] || [ "$WHAT" = "all" ]; then
  echo "build-node: windows build (cargo-xwin)..."
  ssh "$HOST" "cd cyber-build/cyb && nice -n 10 ~/.cargo/bin/cargo xwin build --release -p cyb --target x86_64-pc-windows-msvc 2>&1 | tail -3"
  ssh "$HOST" "cd cyber-build/cyb/target/x86_64-pc-windows-msvc/release && zip -q cyb-$V-windows-x86_64.zip cyb.exe"
  scp -q "$HOST:cyber-build/cyb/target/x86_64-pc-windows-msvc/release/cyb-$V-windows-x86_64.zip" "$OUT/"
  echo "build-node: windows artifact home"
fi

echo "build-node: done"
