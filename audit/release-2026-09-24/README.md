# Cyb candidate qualification — 2026-09-24

Verdict: RED. [GitHub draft candidate](https://github.com/cyberia-to/cyb/releases). This origin-only run covers macOS ARM64. The collector marks the
stack, other desktop targets and Android as missing. No executable was produced.
The full CI matrix is implemented in [PR #1403](https://github.com/cyberia-to/cyb/pull/1403),
whose merge is blocked by GitHub's required independent approval.

Source: `6c20559206d90f0474379e48706ba00db9630a37`; qualifier:
`c5cc2077df510610abb7b5bd69675af6048ed724`. The three versions and all dependency revisions are
in `candidate.json` and `sources.json`. Exact failures and commands are retained
in the platform archive and `release-validation.json`. The missing public Nu
checkout prevents resolving `nu-cli`; this captured Cyb revision also predates
`release/soft3.toml`.

Commands, run with the clean committed soft3 qualifier checked out above:

```sh
python3 release/train.py snapshot --manager . --component cyb --candidate candidate-20260924.1 --output /tmp/cyb-origin-candidate/snapshot
python3 release/train.py build --snapshot /tmp/cyb-origin-candidate/snapshot/sources.json --checkout /tmp/cyb-origin-candidate/sources --output /tmp/cyb-origin-candidate/receipt --target aarch64-apple-darwin
python3 release/train.py collect --snapshot /tmp/cyb-origin-candidate/snapshot/sources.json --artifacts /tmp/cyb-origin-candidate --output /tmp/cyb-origin-candidate/assembled
```

`SHA256SUMS` is the generated asset inventory; this audit README is additional
context. Asset hashes were verified before recording. Draft creation used `python3 release/train.py draft --output /tmp/cyb-origin-candidate/assembled`.
No version tag or public promotion was performed.

## Component presentation trace

`release-page.md` is the current draft presentation. `component-inputs.json`
traces the product's own manifest declarations from the exact captured Git
objects. It retains package names, workspace versions, optional/target/test
conditions, registry requirements, manifest hashes and source links. Registry
ownership is not a resolved dependency version. Missing declarations remain
explicit; the original candidate assets and qualification results are unchanged.

Generator: [soft3 `712af9f3`](https://github.com/cyberia-to/soft3/commit/712af9f302b999ccd848f3467bf471b25caa306d).
The generator's 18 integrity and declaration tests passed with:
`python3 -m unittest discover -s release -p 'test_*.py' -v`.

Reproduce using that generator revision and the captured origin source trees:

```sh
python3 /tmp/cyber-train-20260924/audit-soft3/release/train_components.py --sources /tmp/cyber-train-20260924/audit-cyb/audit/release-2026-09-24/sources.json --checkout /tmp/cyb-origin-candidate/sources --output /tmp/cyber-train-20260924/audit-cyb/audit/release-2026-09-24/component-inputs.json
python3 /tmp/cyber-train-20260924/audit-soft3/release/train_notes.py --directory /tmp/cyber-train-20260924/audit-cyb/audit/release-2026-09-24 --audit-url https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24 --output /tmp/cyber-train-20260924/audit-cyb/audit/release-2026-09-24/release-page.md
```

`presentation-verification.json` records the subsequent GitHub Markdown render,
draft update and unchanged asset digests. The full train inventory remains in
`soft3-dependencies.json`; the release page scopes its component table to this
product. Soft3's table covers its phase-1 qualification inventory and missing
inputs referenced by its manifests, excluding the downstream products.
