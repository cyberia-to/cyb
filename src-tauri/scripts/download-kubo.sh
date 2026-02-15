#!/bin/bash
# Downloads Kubo (IPFS) binary for the target platform into src-tauri/bin/
# Usage: ./download-kubo.sh [version]
# Auto-detects platform or uses TAURI_TARGET_TRIPLE env var

set -euo pipefail

VERSION="${1:-v0.34.1}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$SCRIPT_DIR/../bin"

mkdir -p "$BIN_DIR"

# Map target triple to Kubo platform naming
get_kubo_platform() {
    local triple="$1"
    case "$triple" in
        x86_64-apple-darwin)        echo "darwin-amd64" ;;
        aarch64-apple-darwin)       echo "darwin-arm64" ;;
        x86_64-unknown-linux-gnu)   echo "linux-amd64" ;;
        aarch64-unknown-linux-gnu)  echo "linux-arm64" ;;
        x86_64-pc-windows-msvc)     echo "windows-amd64" ;;
        aarch64-linux-android)      echo "linux-arm64" ;;
        *)
            echo "ERROR: Unsupported target triple: $triple" >&2
            exit 1
            ;;
    esac
}

# Auto-detect current platform triple
detect_triple() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *)       echo "ERROR: Unknown macOS arch: $arch" >&2; exit 1 ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *)       echo "ERROR: Unknown Linux arch: $arch" >&2; exit 1 ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            echo "ERROR: Unknown OS: $os" >&2; exit 1
            ;;
    esac
}

# Use TAURI_TARGET_TRIPLE if set, otherwise auto-detect
TARGET_TRIPLE="${TAURI_TARGET_TRIPLE:-$(detect_triple)}"
KUBO_PLATFORM="$(get_kubo_platform "$TARGET_TRIPLE")"

# Determine binary name (with .exe for Windows)
case "$TARGET_TRIPLE" in
    *windows*)
        SIDECAR_NAME="ipfs-${TARGET_TRIPLE}.exe"
        ARCHIVE_EXT="zip"
        ;;
    *)
        SIDECAR_NAME="ipfs-${TARGET_TRIPLE}"
        ARCHIVE_EXT="tar.gz"
        ;;
esac

OUTPUT_PATH="$BIN_DIR/$SIDECAR_NAME"

# Skip if already downloaded
if [ -f "$OUTPUT_PATH" ]; then
    echo "[download-kubo] $SIDECAR_NAME already exists, skipping download"
    exit 0
fi

# Download
DOWNLOAD_URL="https://dist.ipfs.tech/kubo/${VERSION}/kubo_${VERSION}_${KUBO_PLATFORM}.${ARCHIVE_EXT}"
TMPDIR="$(mktemp -d)"
ARCHIVE_PATH="$TMPDIR/kubo.${ARCHIVE_EXT}"

echo "[download-kubo] Downloading Kubo ${VERSION} for ${TARGET_TRIPLE}..."
echo "[download-kubo] URL: ${DOWNLOAD_URL}"
curl -fsSL "$DOWNLOAD_URL" -o "$ARCHIVE_PATH"

# Extract
echo "[download-kubo] Extracting..."
if [ "$ARCHIVE_EXT" = "zip" ]; then
    unzip -qo "$ARCHIVE_PATH" -d "$TMPDIR"
else
    tar -xzf "$ARCHIVE_PATH" -C "$TMPDIR"
fi

# Move binary to sidecar location
if [ "$ARCHIVE_EXT" = "zip" ]; then
    mv "$TMPDIR/kubo/ipfs.exe" "$OUTPUT_PATH"
else
    mv "$TMPDIR/kubo/ipfs" "$OUTPUT_PATH"
fi

chmod +x "$OUTPUT_PATH"

# Cleanup
rm -rf "$TMPDIR"

echo "[download-kubo] Installed: $OUTPUT_PATH"
ls -lh "$OUTPUT_PATH"
