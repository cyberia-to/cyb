#!/bin/bash
# build-linux — cyb for ubuntu, built where ubuntu actually is.
#
# The cyb workspace reaches into sibling repos (~/cyber/{mir,prysm,tru,
# glia,soma,zheng,...}) that GitHub never sees, so CI cannot build it.
# A build node can: this script rsyncs the workspace to CYB_LINUX_HOST,
# builds there, and brings the tarball home.
#
#   bash harness/build-linux.sh user@host 0.3.0
#
# Node prerequisites (once): rustup stable, plus
#   sudo apt install -y build-essential pkg-config libasound2-dev \
#     libudev-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
#     libwayland-dev libxkbcommon-dev
set -euo pipefail
HOST="${1:?user@host}"; V="${2:?version}"
cd "$(dirname "$0")/../.."   # ~/cyber — the workspace root

SIBLINGS=(cyb mir prysm tru glia soma zheng hemera mudra nox lens strata cybergraph foculus nu rune wysm)
echo "build-linux: syncing workspace to $HOST..."
for repo in "${SIBLINGS[@]}"; do
  [ -d "$repo" ] || continue
  rsync -a --delete --exclude target --exclude .git "$repo" "$HOST:cyber-build/" 
done

echo "build-linux: building on $HOST..."
ssh "$HOST" "cd cyber-build/cyb && ~/.cargo/bin/cargo build --release -p cyb"

echo "build-linux: fetching the artifact..."
ssh "$HOST" "cd cyber-build/cyb/target/release && tar czf cyb-$V-linux-x86_64.tar.gz cyb"
scp "$HOST:cyber-build/cyb/target/release/cyb-$V-linux-x86_64.tar.gz" "cyb/target/release/"
echo "build-linux: done"
