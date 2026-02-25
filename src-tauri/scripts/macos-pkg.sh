#!/usr/bin/env bash
# Build a signed & notarized macOS .pkg installer from a Tauri .app bundle.
#
# Usage:
#   ./scripts/macos-pkg.sh [OPTIONS]
#
# Options:
#   --app-path PATH       Path to .app bundle (default: auto-detect from target/)
#   --output-dir DIR      Output directory for .pkg (default: target/release/bundle/pkg)
#   --skip-notarize       Skip notarization even if credentials are available
#
# Environment variables:
#   PKG_SIGNING_IDENTITY  "Developer ID Installer: ..." identity for .pkg signing
#                         If unset, auto-detects from Keychain. If none found, builds unsigned.
#   APPLE_API_KEY         App Store Connect API Key ID (for notarization)
#   APPLE_API_ISSUER      App Store Connect API Issuer UUID
#   APPLE_API_KEY_PATH    Path to .p8 private key file

set -euo pipefail

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${BLUE}[pkg]${NC} $*"; }
ok()    { echo -e "${GREEN}[pkg]${NC} $*"; }
warn()  { echo -e "${YELLOW}[pkg]${NC} $*"; }
err()   { echo -e "${RED}[pkg]${NC} $*" >&2; }

# Defaults
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_PATH=""
OUTPUT_DIR=""
SKIP_NOTARIZE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --app-path)    APP_PATH="$2"; shift 2 ;;
        --output-dir)  OUTPUT_DIR="$2"; shift 2 ;;
        --skip-notarize) SKIP_NOTARIZE=true; shift ;;
        *) err "Unknown option: $1"; exit 1 ;;
    esac
done

# Auto-detect .app path
if [[ -z "$APP_PATH" ]]; then
    APP_PATH="$(find "$PROJECT_ROOT/target" -path "*/bundle/macos/cyb.app" -maxdepth 6 2>/dev/null | head -1)"
    if [[ -z "$APP_PATH" ]]; then
        err ".app bundle not found. Build first with: cargo tauri build --bundles app"
        exit 1
    fi
fi

if [[ ! -d "$APP_PATH" ]]; then
    err ".app not found at: $APP_PATH"
    exit 1
fi

# Output directory
if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="$PROJECT_ROOT/target/release/bundle/pkg"
fi
mkdir -p "$OUTPUT_DIR"

APP_NAME="$(basename "$APP_PATH" .app)"
PKG_PATH="$OUTPUT_DIR/${APP_NAME}.pkg"

# ============================================================================
# Step 1: Verify .app signature
# ============================================================================
info "Verifying .app signature..."
if codesign --verify --deep --strict "$APP_PATH" 2>/dev/null; then
    ok ".app signature valid"
else
    warn ".app is not signed or has ad-hoc signature (OK for local builds)"
fi

# ============================================================================
# Step 2: Resolve PKG signing identity
# ============================================================================
if [[ -z "${PKG_SIGNING_IDENTITY:-}" ]]; then
    info "PKG_SIGNING_IDENTITY not set, searching Keychain..."
    PKG_SIGNING_IDENTITY="$(security find-identity -v -p basic 2>/dev/null \
        | grep 'Developer ID Installer' \
        | head -1 \
        | sed 's/.*"\(.*\)".*/\1/' || true)"
    if [[ -n "$PKG_SIGNING_IDENTITY" ]]; then
        ok "Found: $PKG_SIGNING_IDENTITY"
    else
        warn "No Developer ID Installer certificate found — building unsigned .pkg"
    fi
fi

# ============================================================================
# Step 3: Build .pkg
# ============================================================================
info "Building .pkg..."
PRODUCTBUILD_ARGS=(
    --component "$APP_PATH" /Applications
    --timestamp
)

if [[ -n "${PKG_SIGNING_IDENTITY:-}" ]]; then
    PRODUCTBUILD_ARGS+=(--sign "$PKG_SIGNING_IDENTITY")
fi

PRODUCTBUILD_ARGS+=("$PKG_PATH")

productbuild "${PRODUCTBUILD_ARGS[@]}"
ok "Built: $PKG_PATH"

# ============================================================================
# Step 4: Notarize
# ============================================================================
if [[ "$SKIP_NOTARIZE" == true ]]; then
    warn "Notarization skipped (--skip-notarize)"
elif [[ -z "${APPLE_API_KEY:-}" || -z "${APPLE_API_ISSUER:-}" ]]; then
    warn "Notarization skipped (APPLE_API_KEY or APPLE_API_ISSUER not set)"
else
    info "Submitting for notarization..."

    NOTARY_ARGS=(
        xcrun notarytool submit "$PKG_PATH"
        --key-id "$APPLE_API_KEY"
        --issuer "$APPLE_API_ISSUER"
        --wait
        --timeout 1800
    )

    # API key file path
    if [[ -n "${APPLE_API_KEY_PATH:-}" ]]; then
        NOTARY_ARGS+=(--key "$APPLE_API_KEY_PATH")
    else
        # Default location used by notarytool
        DEFAULT_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_${APPLE_API_KEY}.p8"
        if [[ -f "$DEFAULT_KEY_PATH" ]]; then
            NOTARY_ARGS+=(--key "$DEFAULT_KEY_PATH")
        else
            # Check runner temp (CI)
            RUNNER_KEY_PATH="${RUNNER_TEMP:-/tmp}/private_keys/AuthKey_${APPLE_API_KEY}.p8"
            if [[ -f "$RUNNER_KEY_PATH" ]]; then
                NOTARY_ARGS+=(--key "$RUNNER_KEY_PATH")
            else
                err "API key file not found at $DEFAULT_KEY_PATH or $RUNNER_KEY_PATH"
                exit 1
            fi
        fi
    fi

    "${NOTARY_ARGS[@]}"
    ok "Notarization complete"

    # ========================================================================
    # Step 5: Staple
    # ========================================================================
    info "Stapling notarization ticket..."
    xcrun stapler staple "$PKG_PATH"
    ok "Stapled: $PKG_PATH"
fi

# ============================================================================
# Summary
# ============================================================================
echo ""
ok "macOS .pkg ready: $PKG_PATH"
PKG_SIZE="$(du -h "$PKG_PATH" | cut -f1)"
ok "Size: $PKG_SIZE"

if [[ -n "${PKG_SIGNING_IDENTITY:-}" ]]; then
    ok "Signed: $PKG_SIGNING_IDENTITY"
    echo ""
    info "Verify with:"
    echo "  pkgutil --check-signature \"$PKG_PATH\""
    echo "  spctl --assess --type install \"$PKG_PATH\""
fi
