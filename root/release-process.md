# release process

draft. this document defines versioning, testing, and delivery for cyb.

## principles

cyb is a robot converging toward its final form. every release is a step toward freezing. the versioning system must express both the rate of change (semver) and the distance to completion (kelvin).

code has two layers: system code (the shell, the runtime) and application code (UI, pages, scripts). system code ships in binaries. application code should be updatable without redeployment — via cybergraph, IPFS, or on-chain contracts.

## versioning: kelvin + semver

kelvin versioning counts down. 100K means "far from done, expect breaking changes." 0K means "frozen forever, no changes possible." the number is a temperature — hot code cools as it stabilizes.

semver tracks what changed between releases. major.minor.patch describes the nature of the change. kelvin describes the maturity of the component.

combined format: `K100 v0.1.0` → kelvin 100, semver 0.1.0

each component has its own kelvin temperature:

| component | current K | meaning |
|-----------|-----------|---------|
| hemera (hash) | 0K target | frozen at genesis, never changes |
| tri-kernel (math) | 10K | core algorithm, rarely changes |
| nox (VM) | 20K | execution model, stabilizing |
| bevy shell | 200K | native runtime, actively evolving |
| react UI | 300K | web interface, high churn |
| leptos UI | 400K | early, experimental |
| nu scripting | 300K | active development |
| neural language | 100K | specification stabilizing |

kelvin drops when:
- a component passes a test suite milestone
- breaking changes become impossible by design (frozen interfaces)
- the component has been stable for N releases without changes

kelvin never goes up. if a frozen component needs changes, fork it under a new name.

semver increments normally:
- patch: bugfix, no behavior change
- minor: new feature, backward compatible
- major: breaking change (should be rare as K approaches 0)

a component at 0K has semver frozen too. 0K v1.0.0 forever.

## release tiers

### nightly (every push to master)

- automated CI build
- all platforms: web, macOS, Linux, Windows, Android
- artifacts published as GitHub Actions artifacts (7 day retention)
- no manual testing, no guarantees
- version: git commit hash

### canary (weekly)

- automated CI build + smoke tests
- published to GitHub Releases as pre-release
- available for brave users and internal testing
- version: `K{kelvin} v{semver}-canary.{date}`
- testing: dev team runs through critical paths on each platform

### stable (when ready, manual trigger, new moon target initially)

- full test suite passes
- manual QA on all platforms
- published to GitHub Releases as latest
- version: `K{kelvin} v{semver}`
- deployed to: cyb.ai (web), app stores (future), IPFS

## what triggers a release

- nightly: automatic on every master push
- canary: weekly
- stable: manual, after canary has been tested for N days with no blockers

## testing

### automated (CI)

- react: `deno task build` succeeds (build-time type check)
- bevy: `cargo check --workspace` + `cargo build --release`
- leptos: `trunk build --release`
- android: APK assembles
- future: unit tests per component, integration tests

### smoke tests (canary)

checklist per platform:

**web (all platforms)**
- [ ] oracle search works
- [ ] wallet connects (read-only + signing)
- [ ] cyberlinks visible
- [ ] navigation between pages
- [ ] avatar page shows correctly

**desktop (macOS/Linux/Windows)**
- [ ] app launches without crash
- [ ] WebView loads react mode
- [ ] mode switching works (Cmd+1/2/3/4)
- [ ] IPFS node starts

**android**
- [ ] APK installs on Pixel
- [ ] app launches, WebView loads
- [ ] basic navigation works
- [ ] works offline (local assets)

### user testing (stable)

- canary link shared in telegram/discord
- feedback collected for N days
- blockers filed as issues
- stable cut when no P0/P1 open

## code layers

### layer 0: frozen (0K)

cryptographic primitives. hash functions. field arithmetic. these never change after genesis. bugs here mean a full migration, not a patch.

examples: hemera parameters, goldilocks field constants, CID format

### layer 1: system (10-50K)

the runtime. bevy shell, wry WebView integration, nushell engine, IPC between modes. ships as a binary. updated via app store / manual download / auto-update.

changes here require a new binary release.

### layer 2: application (100-400K)

UI, pages, business logic. react components, leptos views, rune scripts. currently ships bundled in the binary (WebView loads from assets).

target architecture: application code loaded from cybergraph or IPFS at runtime. the shell fetches the latest UI from a content-addressed source. update without redeployment:

```
shell starts
  → reads pinned CID from local config (or on-chain pointer)
  → fetches UI bundle from IPFS / cybergraph
  → loads in WebView
  → user sees latest UI without app update
```

fallback: if IPFS unreachable, load bundled version from assets (the version shipped with the binary).

### layer 3: content (∞K, always hot)

knowledge graph pages, user data, cyberlinks. never frozen, always growing. lives in cybergraph, not in the binary.

## update channels

| layer | update mechanism | frequency |
|-------|-----------------|-----------|
| 0 (frozen) | never (or catastrophic migration) | never |
| 1 (system) | binary release (app store, GitHub, IPFS) | monthly |
| 2 (application) | OTA via IPFS CID pointer | weekly or continuous |
| 3 (content) | cybergraph transactions | continuous |

### OTA for application layer

the shell maintains a pointer to the current UI version:

```
~cyb/react/latest → CID of current react build
~cyb/leptos/latest → CID of current leptos build
```

on startup, shell resolves the pointer, fetches the bundle, verifies hash, loads. if the pointer updates (new cyberlink from the cyb neuron), next launch gets the new UI.

this means:
- UI bugs fixed without App Store review
- A/B testing by pointing different users to different CIDs
- rollback by pointing back to previous CID
- users can pin a specific version if they prefer

### signing and trust

OTA bundles must be signed. the shell trusts a specific neuron (or multisig) for UI updates. users can override: pin a CID, trust a different neuron, or disable OTA entirely.

## open questions

- how to handle semver across independent components? monorepo version vs per-component version?
- should kelvin be visible to users or only internal?
- OTA: how to handle breaking changes between shell and UI? (version negotiation protocol)
- testing: what's the minimum automated test coverage before cutting stable?
- android: Play Store vs sideload APK vs F-Droid?
- auto-update for desktop: Sparkle (macOS), WinSparkle (Windows), AppImage update?
