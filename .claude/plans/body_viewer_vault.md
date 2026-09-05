# body + particle viewer + vault

Approved direction (2026-09-05): three worlds, all real, all vanilla Bevy stack.

## body — the main page (real resources, real mining, PUSSY/day)

New `WorldState::Body`, becomes the default world and the first tab.

**Telemetry** (`worlds/body/telemetry.rs`) — background sampler thread, 1s cadence,
atomics/mutex snapshot into a Bevy resource:
- CPU %: total + top processes via one `ps axo pid,pcpu,pmem,comm -r` exec per tick.
- Memory: total `sysctl hw.memsize`, used via `vm_stat` page counts. Android: /proc/meminfo.
- Network: `netstat -ib` byte-counter deltas -> B/s up/down. Android: /proc/net/dev.
- GPU + watts: streaming `sudo -n powermetrics --samplers cpu_power,gpu_power -i 1000`
  child (passwordless sudo verified on this machine; pattern from xena-bench/src/power.rs).
  Parse CPU mW / GPU mW / GPU residency. No sudo -> GPU row shows "unavailable".
- Task attribution: top-N processes by cpu/mem; cyb's own tasks tagged (soma = thinking,
  erga = mining) since we know when they run.

**Mining** (`worlds/body/miner.rs`) — REAL, via erga (recon 2026-09-05):
- Spawn `<erga> mine --machine` as child (erga resolves pool+payout from its own
  ~/Library/Application Support/ai.cyber.erga/ config); parse `DEVICE`/`STAT` lines:
  `STAT <rate_khs> <height> <accepted> <rejected> <hashed> <donated> <build%> <next%> <status...>`.
  Binary path: /Applications/erga.app/Contents/MacOS/erga, else ~/cyber/erga target.
- NEVER link erga-miner in-process: erga CLAUDE.md forbids a second graphics context
  in one process (Metal + Metal aborts). Subprocess only. Kill child on app exit.
- Intensity buttons max/eco/min -> write the 3-byte intensity file; the running miner
  (ours or external) re-reads it every 500ms — live duty-cycle control.
- External erga detection (pgrep): show as "external - running", own start disabled.
- ERG/day = rate_mhs*1e6 / (difficulty/BLOCK_TIME) * (86400/BLOCK_TIME) * 3.0
  (erga's own formula); difficulty polled from herominers API via ureq every 10 min.

**PUSSY/day**: `~/cyb/rates.toml` — `[pussy] per_erg = N`, `per_usd = M` (user-editable;
spacepussy has no live market — the rate is a declared conversion, marked "est").
Panel shows native/day AND pussy/day per task + total.

## brain viewer — tap a node, read the particle

- `BrainIndex` += `hashes: Vec<[u8;32]>` (already at hand in insert_graph_config).
- Tap detection in Graph world: press/release within 8px & 400ms (mouse + touch);
  project all nodes with cam.view_proj() (same math as place_labels), pick nearest
  within max(14px, projected radius). Also set mir `WarpTarget` for the camera flight.
- Fullscreen overlay over the content area (GlobalZIndex(20), commander stays):
  title = label/short-hex, body = content::load() text (scrollable), stats line
  (focus, in/out degree), axon lists in/out — each neighbor row is a button that
  re-opens the viewer on that particle. Esc / [x] closes.
- Attention extends to pages: on close, cast particle("brain") -> viewed particle
  with dwell seconds — same rule as world transitions.

## vault — secrets that never touch the graph

- Store `~/cyb/vault.enc`: XChaCha20-Poly1305 (new dep chacha20poly1305), key =
  SHA-256("cyb-vault-v1" || mudra::seed::seed(mnemonic)) — the mnemonic IS the vault
  key; same words open the vault on any body. Plaintext = serde_json entries
  {name, kind: password|key|seed|otp|custom, value, note, created}.
- Input path bypasses the graph entirely: `vault add <name> <kind> <value...>` is
  intercepted in com BEFORE remember/cast — the raw line is never persisted, echo
  is masked. Also `vault rm <name>`.
- Vault world UI: rows name+kind; tap = copy to clipboard (arboard, already a dep) +
  auto-clear after 30s if clipboard still holds it; hold "reveal" chip shows value
  while pressed; otp entries render live TOTP (RFC 6238, hmac+sha1) + countdown,
  tap copies the code.

## Order
1. body (main page, biggest) 2. viewer 3. vault. Each lands with build + screenshot
verification, then dmg + installs at the end.
