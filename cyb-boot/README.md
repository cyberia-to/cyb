# cyb-boot: Self-Bootstrapping Installer for cyb

Referral-aware thin installer (~3MB) that bootstraps cyb from the content-addressed network.

## Architecture

```
User visits cyb.ai/mining
        ↓
Clicks "Download" → server serves cyb-boot binary
  (pre-built, patched with user's wallet + referrer)
        ↓
User runs cyb-boot:
  1. Imports wallet from boot.dat (mnemonic + referrer)
  2. Connects to iroh bootstrap nodes
  3. Fetches version registry particle (hardcoded CID)
  4. Resolves latest cyb CID for user's platform
  5. Downloads cyb binary by CID (hash-verified by iroh)
  6. Installs cyb
  7. Launches cyb → first network sync announces {introduced_by: referrer}
        ↓
Referral recorded as cyberlink in knowledge graph:
  referrer_pubkey → new_user_pubkey
```

cyb-boot is intentionally thin — its only job is to speak iroh and fetch a CID. All update logic lives in cyb itself.

## Two apps

| App | Size | Purpose |
|-----|------|---------|
| **cyb-boot** | ~3MB | Thin installer: imports wallet, downloads cyb via iroh, installs |
| **cyb** | ~100MB+ | The actual Tauri desktop app (browser, mining, knowledge graph) |

cyb-boot is NOT the app. It downloads and installs the app, then exits.

## Version Registry

The registry is a particle in the cyb network at a known CID:

```json
{
  "version": "0.3.1",
  "stable": "bafk2bzacec...",
  "platforms": {
    "x86_64-unknown-linux-musl":  "bafk2bzacec...",
    "aarch64-unknown-linux-musl": "bafk2bzacec...",
    "x86_64-apple-darwin":        "bafk2bzacec...",
    "aarch64-apple-darwin":       "bafk2bzacec...",
    "x86_64-pc-windows-msvc":     "bafk2bzacec..."
  },
  "min_boot_version": "0.1.0",
  "changelog_cid": "bafk2bzacec..."
}
```

To release a new cyb version: update the registry particle, publish to the network. cyb-boot always fetches the same registry CID. Only when the registry format breaks does cyb-boot need an update.

## Referral Embedding

Binaries are pre-compiled in CI. The server patches a reserved 64-byte slot in the binary before serving — no recompilation needed:

```rust
// Reserved at compile time
static REF_SLOT: [u8; 64] = *b"__CYB_REF_PLACEHOLDER_\0\0\0\0\0\0...";
```

The server finds `__CYB_REF_PLACEHOLDER_` marker and overwrites with the referrer address. Takes microseconds per request.

## Current Implementation vs Target

The current `main.rs` is a **simplified stepping stone**:

| Feature | Current | Target |
|---------|---------|--------|
| Wallet import | boot.dat (AES-256-GCM encrypted) | boot.dat (same) |
| Referrer | In boot.dat payload | Binary patch (REF_SLOT) |
| App download | HTTP from GitHub Releases (cyb.pkg) | iroh fetch by CID |
| Version registry | Hardcoded URL | Particle at known CID via iroh |
| Referral registration | Via litium-refer contract | Cyberlink in knowledge graph |
| Install method | Opens .pkg installer | Direct binary install |

## Platform Matrix

| Target Triple | Platform | Notes |
|---|---|---|
| `aarch64-apple-darwin` | macOS Apple Silicon | M1/M2/M3 |
| `x86_64-apple-darwin` | macOS Intel | |
| `x86_64-pc-windows-msvc` | Windows x64 | |
| `x86_64-unknown-linux-musl` | Linux x64 | Static binary (musl), no glibc dep |
| `aarch64-unknown-linux-musl` | Linux ARM64 | Raspberry Pi, cloud ARM |

## macOS Notarization

macOS Gatekeeper blocks unsigned apps. For macOS, CI:
1. Builds the cyb-boot binary
2. Wraps it in an `.app` bundle (`boot cyb.app`)
3. Signs with Developer ID Application certificate + hardened runtime
4. Notarizes with Apple (xcrun notarytool)
5. Staples the notarization ticket
6. Zips as `boot-cyb.zip`

The server stores `boot-cyb.zip` as a template. On each macOS request, it clones the zip and appends `boot.dat` alongside the `.app`. The binary inside cannot be modified (would break signature), but `boot.dat` sits next to the `.app` in the zip.

For Windows/Linux, the server zips the bare binary + `boot.dat` directly.

## Source Layout

```
cyb-boot/
├── src/main.rs          # cyb-boot binary (shipped to users)
├── Cargo.toml
├── server/
│   ├── main.go          # distribution server (Go, runs on Cyberproxy)
│   └── go.mod
├── README.md            # this file
└── CLAUDE.md            # project rules for AI assistants
```

## Build

```bash
# cyb-boot binary (Rust)
cargo build --release --target aarch64-apple-darwin

# Distribution server (Go, cross-compile for Cyberproxy)
cd server && GOOS=linux GOARCH=amd64 go build -o cyb-boot-server .
```

## Distribution Server

Go HTTP server on Cyberproxy. Its only job: serve pre-built cyb-boot binaries bundled with user's `boot.dat`.

**API:** `POST /api/boot` with `{"platform": "<target-triple>", "data": "<base64-encrypted-bootstrap>"}`

**macOS:** reads `boot-cyb.zip` template from artifacts, appends `boot.dat`, returns combined zip.
**Windows/Linux:** reads bare binary from artifacts, zips with `boot.dat`, returns zip.

### Server artifacts (on Cyberproxy)

```
/home/cyber/cyb-boot/artifacts/
├── boot-cyb.zip                      # notarized macOS .app template
├── cyb-boot-aarch64-apple-darwin        # bare macOS ARM binary
├── cyb-boot-x86_64-apple-darwin         # bare macOS Intel binary
├── cyb-boot-x86_64-pc-windows-msvc      # Windows binary
├── cyb-boot-x86_64-unknown-linux-musl   # Linux x64 binary
└── cyb-boot-aarch64-unknown-linux-musl  # Linux ARM binary
```

## Infrastructure

### Request flow

```
POST https://cyb.ai/api/boot
  → Cyberproxy nginx (167.235.28.94:443)
    location = /api/boot → http://127.0.0.1:8098
  → cyb-boot-server (:8098, runs locally on Cyberproxy)
    reads /home/cyber/cyb-boot/artifacts/
```

### Systemd service (Cyberproxy)

```ini
# /etc/systemd/system/cyb-boot-server.service
[Unit]
Description=cyb-boot distribution server
After=network.target

[Service]
Type=simple
User=cyber
ExecStart=/home/cyber/cyb-boot/server/cyb-boot-server
Environment=CYB_BOOT_ARTIFACTS=/home/cyber/cyb-boot/artifacts
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Deployment (server binary)

```bash
# 1. Cross-compile server for Linux
cd cyb-boot/server
GOOS=linux GOARCH=amd64 go build -o /tmp/cyb-boot-server .

# 2. Upload to Cyberproxy
scp -P 8333 /tmp/cyb-boot-server cyber@cyberproxy:/home/cyber/cyb-boot/server/cyb-boot-server.new

# 3. Swap and restart (on Cyberproxy)
cd /home/cyber/cyb-boot/server
mv cyb-boot-server cyb-boot-server.old
mv cyb-boot-server.new cyb-boot-server
chmod +x cyb-boot-server
sudo systemctl restart cyb-boot-server
```

### Updating artifacts

**Automated (CI):** Pushes to `master` or tag pushes trigger a webhook that deploys boot artifacts to Cyberproxy. CI uploads artifacts to GitHub Actions, then fires a webhook. On Cyberproxy, the deploy script downloads artifacts via GitHub API and swaps them atomically.

**CI deploy flow:**
```
CI builds artifacts → uploads to GitHub Actions artifacts
  → CI triggers webhook: POST https://cyb.ai/hooks/boot-deploy (HMAC-SHA1)
  → webhook-deploy (port 9000) validates HMAC, runs deploy script
  → script uses GitHub API + PAT to download artifacts
  → extracts to /home/cyber/cyb-boot/artifacts/ (atomic rename)
```

**Required GitHub secret:** `BOOT_DEPLOY_WEBHOOK_SECRET` — HMAC-SHA1 secret for webhook auth.

**Required on Cyberproxy:** GitHub fine-grained PAT at `/home/cyber/cyb-boot/.github-pat` (Actions read-only on `cyberia-to/cyb-ts`).

**Manual fallback:** You can still upload artifacts manually:

```bash
# Upload new macOS .app bundle template
scp -P 8333 boot-cyb.zip cyber@cyberproxy:/home/cyber/cyb-boot/artifacts/

# Upload new platform binary
scp -P 8333 cyb-boot-aarch64-apple-darwin cyber@cyberproxy:/home/cyber/cyb-boot/artifacts/
```

No server restart needed — artifacts are read from disk on each request.

### Nginx config (Cyberproxy)

`/etc/nginx/sites-enabled/cyb.ai`:
```nginx
location = /api/boot {
    proxy_pass http://127.0.0.1:8098;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_read_timeout 60s;
    client_max_body_size 1k;
}
```

## CI

`.github/workflows/macos_release.yml` produces:
- **Release artifacts** (GitHub Releases): `cyb.dmg`, `cyb.pkg` — the actual cyb app
- **Boot server artifacts** (auto-deployed to Cyberproxy via webhook on master/tag push): `boot-cyb.zip`, `cyb-boot-aarch64-apple-darwin`

## Open Questions

1. **iroh bootstrap stability** — bootstrap nodes are hardcoded. Handle churn via embedded list + DNS TXT fallback?
2. **Registry before iroh** — until iroh is ready, use HTTP fallback for registry lookup?
3. **Windows signing** — SmartScreen blocks unsigned binaries. Code signing cert ~$400/year.
4. **cyb-boot self-update** — no auto-update path. Keep it minimal. `min_boot_version` in registry warns outdated installs.
