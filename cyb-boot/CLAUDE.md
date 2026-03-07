# cyb-boot — project rules

## What cyb-boot IS

A thin installer (~3MB) that bootstraps the cyb desktop app from the content-addressed network. It is NOT the app itself.

Target flow (from design doc):
1. Import wallet from boot.dat (mnemonic + referrer)
2. Connect to iroh bootstrap nodes
3. Fetch version registry particle (hardcoded CID)
4. Resolve latest cyb CID for user's platform from registry
5. Download cyb binary by CID (hash-verified by iroh)
6. Install cyb
7. Launch cyb → first network sync registers referral as cyberlink

Current implementation is simplified — uses HTTP download from GitHub instead of iroh. See README.md "Current vs Target" table.

## Key concepts

- **Two apps**: cyb-boot (installer) downloads and installs cyb (the actual app), then exits
- **Version registry**: a particle in the knowledge graph at a known CID, maps platforms to cyb binary CIDs
- **Referral embedding**: server patches a 64-byte slot in the pre-built binary with the referrer address (no recompilation)
- **macOS notarization**: .app bundle must be pre-signed+notarized in CI. Server cannot modify the binary — only appends boot.dat alongside in the zip
- **boot.dat**: AES-256-GCM encrypted payload with user's mnemonic + referrer, bundled by the server per-request

## Source layout

```
cyb-boot/
├── src/main.rs          # cyb-boot Rust binary (shipped to users)
├── Cargo.toml
├── server/
│   ├── main.go          # distribution server (Go, runs on Cyberproxy)
│   └── go.mod
```

## Build

```bash
# cyb-boot binary
cargo build --release --target aarch64-apple-darwin

# Distribution server (cross-compile for Cyberproxy)
cd server && GOOS=linux GOARCH=amd64 go build -o cyb-boot-server .
```

## Infrastructure

- Server runs on Cyberproxy (167.235.28.94:8098), locally proxied via cyb.ai/api/boot
- Server binary: `/home/cyber/cyb-boot/server/cyb-boot-server`
- Artifacts: `/home/cyber/cyb-boot/artifacts/` (platform binaries + boot-cyb.zip)
- Systemd service: `cyb-boot-server.service`
- Artifacts are auto-deployed via webhook on master/tag pushes (secret: `BOOT_DEPLOY_WEBHOOK_SECRET`). Manual SCP still works as fallback
- boot-cyb.zip is an internal server artifact, NOT a GitHub Release asset

## Rules

- Never put boot-cyb.zip in GitHub Releases — it's an internal server template
- The distribution server's only job is bundling pre-built binaries with boot.dat
- cyb-boot does the smart work: iroh networking, registry resolution, app download
- The version registry lives in the knowledge graph (particle at known CID), not on the server
- Keep cyb-boot minimal — it should rarely need updates
