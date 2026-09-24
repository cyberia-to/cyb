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
