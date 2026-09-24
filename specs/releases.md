# Cyb releases

Cyb consumes the shared [soft3 release contract](https://github.com/cyberia-to/soft3/blob/main/specs/releases.md).
`release/soft3.toml` pins the stack version and full source revision. The reusable
workflow is referenced at the same revision. Soft3 owns shared source pins and
component qualification; Cyb adds desktop and Android acceptance.

Friday 12:00 UTC, and manual dispatch, capture default-branch origin inputs and
create `candidate-YYYYMMDD.N` as a GitHub draft prerelease. Rehearsals use
`cut=false`. Native runners build macOS and Linux on ARM64/x64; Android adds an
ARM64 APK. The owner promotes candidates and pushes version tags.

Every candidate includes `SHA256SUMS`, `sources.json`, `candidate.json`,
`release-validation.json`, `soft3-dependencies.json` and `soft3-dependencies.md`.
Platform archives hold available binaries, source inventories and command logs.
The release description includes the inventory. Missing artifacts and failed
checks produce red receipts. A green Cyb candidate requires a green soft3 stack.

Product gates are locked Cargo checks/tests, `make fleet`, `make dmg` on macOS,
a locked release build on Linux, and `make android` plus APK signature
verification on Android. Unavailable forks, source changes during the build,
missing signatures and unresolved packages stay explicit failures. Source
checkout never copies a developer's local Nu or shader compiler fork.

This release dependency binds stack composition and qualification. Rust API
integration remains with component changes; a source pin alone adds no runtime
capability.

From candidate cut until owner verdict, the three default branches are frozen.
Candidate fixes and audit receipts go on `release/<date>`. Receipts belong in
`audit/release-<date>/`; the Cyber launch log records each candidate. Changes to
versions and pins follow the coordinated bump PR rule in `~/cyber/AGENTS.md`.
`make ship`, package publication, version tags and promotion remain owner actions.
