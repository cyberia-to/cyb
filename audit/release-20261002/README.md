# cyb 0.16.0 — prepared 2026-10-02

agent prepared the release. owner publishes with `make ship`.
the agent did not tag, push, or run `make ship`.

## tree

- cyb local `master` (this commit). not pushed.
- previous GitHub latest: `v0.15.1`
- soma kernel streaming lives on `soma` branch `feat/kernel-live-stream`
  (`4bdff43`). path dep. origin/main of soma still has no Rust.

## gates

- `make fleet`: 21 ok, 0 failed, GREEN (twice: before feat commit, at commit)
- `make dmg`: GREEN
- `make android`: GREEN, apksigner v2 verified, 1 signer
- node linux: GREEN (chip `v0.16.0  dev 10.02 09:08` — rsync drops `.git`)
- node windows: RED — `run` / `memmap2::UncheckedAdvice`. no windows zip
  attached. `harness/build-node.sh` now fails the ssh if cargo fails, so
  a later `make ship` cannot wrap a stale `cyb.exe`.

## artifacts (`cyb/target/release/`)

```
622be8a6a2d16699d3b67047636f193d6c621dcbbaad4c2ef1e7044953d59127  cyb-0.16.0.dmg
906e37fd4409bc641d8ee5c90e9eaddbce34057e9b357f5b3bad98c15914ab2d  cyb-0.16.0.apk
0147274d808245466f18ea687a9e88c3c70234e02e197c1a8cbc05ee398b3d76  cyb-0.16.0-linux-x86_64.tar.gz
```

chips:

- macOS `v0.16.0  e7a6756d 10.02 09:07` (built before the plist/node-script
  commits; `make ship` rebuilds so the chip matches the tag)
- android `v0.16.0 09:14`
- linux `v0.16.0  dev 10.02 09:08`

## owner

from a pushed, clean `master`:

```
make ship V=0.16.0 T="chat, live soma, a lived-in body"
```

`N=` is optional; `releases/v0.16.0.md` is the prose. ship will rebuild
dmg+apk from the tagged tree, skip a red windows node, and publish.
