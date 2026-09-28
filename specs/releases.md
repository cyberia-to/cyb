# Cyb releases

Cyb selects one concrete [soft3 build](https://github.com/cyberia-to/soft3/blob/main/specs/releases.md)
through `release/soft3.toml`: build name, GitHub release ID, SHA256SUMS digest and
expected version/source revision. This assembly supplies Cyb's component source
revisions, including its embedded Nushell libraries, and the original stack
qualification. Cargo compiles those sources for Cyb's platform/features.

A product cut captures only Cyb's own default-branch source. It authenticates the
selected soft3 evidence, materializes its fixed component revisions and resolves
Cyb's sibling paths inside that assembly. External packages use Cargo.lock;
stack-owned crates must resolve from the assembly. The qualification engine's
immutable workflow revision is independent of the selected build.

Cyb runs locked Cargo checks/tests, fleet, macOS DMG, Linux release builds and
Android APK/signature acceptance. Shared stack qualification is inherited. A RED
soft3 build keeps Cyb RED, including when product checks pass. Missing platform
receipts, unavailable inputs and altered source/checksum data remain failures.

Every candidate carries `SHA256SUMS`, `sources.json`, `candidate.json`,
`release-validation.json`, `soft3-dependencies.json`, `soft3-dependencies.md`,
`soft3-build.json` and `soft3-build.tar.gz`. The last two retain the selected pin
and original upstream evidence. The release page links the build and components.

## embedded Nu

The terminal embeds the `nu-*` Rust crates selected by soft3's Nu source pin.
`shell/Cargo.toml` addresses them under the assembled `nu/crates/` directory.
The source is Nushell's upstream repository at the soft3-pinned commit. The
terminal creates its own engine and standard library inside the Cyb process.
The independently pinned `nu` executable in CI runs build scripts.

## operation

Friday 12:00 UTC or manual dispatch creates a draft `candidate-YYYYMMDD.N`.
`cut=false` retains rehearsal evidence. Public soft3 releases are readable by
product workflows; a cross-repository draft requires `SOFT3_READ_TOKEN` with
read access to soft3. A build update is an explicit reviewed pin change.

The owner promotes candidates, merges version/build bumps and pushes tags.
`make ship` remains an owner action. Default branches are frozen between cut
and verdict; fixes and receipts go to `release/<date>` and `audit/release-<date>/`.
