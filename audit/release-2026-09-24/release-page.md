## 🚦 cyb · candidate-20260924.1

| Readiness | Distribution |
|---|---|
| 🔴 RED · qualification incomplete | 📝 Draft prerelease |
| ⛔ No installable binary in this candidate | Promotion: owner decision |

🟢 Pass · 🔴 Fail · 🟠 Blocked by a prerequisite · ⚪ No receipt · ➖ Gate not selected.

## 🔗 Versions and sources

| Product | Version | Captured source |
|---|---|---|
| [soft3](https://github.com/cyberia-to/soft3) | 0.10.0 | [`c5cc2077`](https://github.com/cyberia-to/soft3/commit/c5cc2077df510610abb7b5bd69675af6048ed724) |
| [cyber](https://github.com/cyberia-to/cyber) | 0.8.0 | [`083871d2`](https://github.com/cyberia-to/cyber/commit/083871d20d30adfd1e31615b1615940ed4b9141f) |
| [cyb](https://github.com/cyberia-to/cyb) | 0.15.1 | [`6c205592`](https://github.com/cyberia-to/cyb/commit/6c20559206d90f0474379e48706ba00db9630a37) |

Qualifier: [soft3 c5cc2077](https://github.com/cyberia-to/soft3/tree/c5cc2077df510610abb7b5bd69675af6048ed724/release). Exact versions and revisions: [candidate.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/candidate.json).

## ✨ Changes at the captured revisions

1. cyber: [fix: consume soft3 qualification for Cyber draft releases](https://github.com/cyberia-to/cyber/pull/111).
2. soft3: [fix: link candidate input updates to their pull requests](https://github.com/cyberia-to/soft3/pull/29).

🟠 cyb: captured product HEAD has no merged pull request.

## 💻 Platforms

| Platform | Qualification | Binary | Evidence |
|---|---|---|---|
| 🧩 Shared stack | ⚪ No receipt | ➖ Stack checks | — |
| 🍎 macOS · ARM64 | 🔴 Fail | ⛔ Unavailable | [📦 Logs + receipt](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) |
| 🍎 macOS · x64 | ⚪ No receipt | ⛔ Unavailable | — |
| 🐧 Linux · ARM64 | ⚪ No receipt | ⛔ Unavailable | — |
| 🐧 Linux · x64 | ⚪ No receipt | ⛔ Unavailable | — |
| 🤖 Android · ARM64 | ⚪ No receipt | ⛔ Unavailable | — |

Missing required receipts keep the candidate RED. Archives without binaries contain qualification evidence only.

## 🧱 Main blockers

| Check | Finding | Evidence |
|---|---|---|
| 🔴 Fail · `origin-checkouts` | Origin checkout unavailable: nu | [Log / receipt ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) |
| 🔴 Fail · `soft3-dependency` | This captured revision has no release/soft3.toml contract. | [Log / receipt ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) |
| 🔴 Fail · `cyb-check` | nu-cli cannot resolve because the Nu origin is unavailable. | [Log / receipt ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/logs/aarch64-apple-darwin/cyb-check.txt) |

<details>
<summary>🧪 Complete qualification matrix — every recorded gate</summary>

| Gate | 🧩 Shared stack | 🍎 macOS · ARM64 | 🍎 macOS · x64 | 🐧 Linux · ARM64 | 🐧 Linux · x64 | 🤖 Android · ARM64 |
|---|---|---|---|---|---|---|
| `origin-checkouts` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `phase1-pins` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `release-notes-source` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `soft3-dependency` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `cyb-check` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/logs/aarch64-apple-darwin/cyb-check.txt) | ⚪ | ⚪ | ⚪ | ⚪ |
| `cyb-tests` | ⚪ | [🟠 Blocked](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `cyb-fleet` | ⚪ | [🟠 Blocked](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `cyb-dmg` | ⚪ | [🟠 Blocked](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `source-inputs-unchanged` | ⚪ | [🟢 Pass](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |
| `package-resolution` | ⚪ | [🔴 Fail](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/cyb-candidate-20260924.1-aarch64-apple-darwin.tar.gz?raw=true) | ⚪ | ⚪ | ⚪ | ⚪ |

Cell links open recorded logs or release downloads. ➖ means the gate was not selected for that job; ⚪ means its platform receipt is missing.

</details>

<details>
<summary>📚 Full soft3 source inventory</summary>

Pin status compares source revisions only. Component test results are in the qualification matrix above.

| Component | Captured revision | Source pin | Primary declared package |
|---|---|---|---|
| [bbg](https://github.com/cyberia-to/bbg) | [`a3c9a0c2`](https://github.com/cyberia-to/bbg/commit/a3c9a0c22bfb78ef891265f9e2cd25838ef77c2d) | 🟢 Matches | bbg @ 0.2.1 |
| [cyb](https://github.com/cyberia-to/cyb) | [`6c205592`](https://github.com/cyberia-to/cyb/commit/6c20559206d90f0474379e48706ba00db9630a37) | 📌 Product source | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [cyber](https://github.com/cyberia-to/cyber) | [`083871d2`](https://github.com/cyberia-to/cyber/commit/083871d20d30adfd1e31615b1615940ed4b9141f) | 📌 Product source | true-cyber @ 0.8.0 |
| [cybergraph](https://github.com/cyberia-to/cybergraph) | [`aa374ced`](https://github.com/cyberia-to/cybergraph/commit/aa374ced57eed6bbf42de0b77d29b4c5412fd30d) | 🟢 Matches | cybergraph @ 0.1.1 |
| [evy](https://github.com/cyberia-to/evy) | [`2f23bfb9`](https://github.com/cyberia-to/evy/commit/2f23bfb97fe2904e3bcba3f116b473ed6cbec483) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [file](https://github.com/cyberia-to/file) | [`fc666a58`](https://github.com/cyberia-to/file/commit/fc666a5880ff874adae655150414b3a45832baf0) | 🟢 Matches | cyber-file @ 0.1.0 |
| [foculus](https://github.com/cyberia-to/foculus) | [`177fad4c`](https://github.com/cyberia-to/foculus/commit/177fad4c614a8e002aac28c9ca2a8196ce69b6cb) | 🟢 Matches | foculus @ 0.1.3 |
| [glia](https://github.com/cyberia-to/glia) | [`89adc925`](https://github.com/cyberia-to/glia/commit/89adc925c47205ba8ce0438ffb984f3e1466a92a) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [hemera](https://github.com/cyberia-to/hemera) | [`23f3bbcf`](https://github.com/cyberia-to/hemera/commit/23f3bbcff910ea6d504ceb505680a539260869da) | 🟢 Matches | cyber-hemera @ 0.3.1 |
| [honeycrisp](https://github.com/cyberia-to/honeycrisp) | [`9593c721`](https://github.com/cyberia-to/honeycrisp/commit/9593c7218e14e2b5b816d9008b45ab60b9580cf3) | 🟢 Matches | honeycrisp · workspace version |
| [inf](https://github.com/cyberia-to/inf) | [`bf379879`](https://github.com/cyberia-to/inf/commit/bf379879b60d724cb34fead91d703e0090acc08e) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [lens](https://github.com/cyberia-to/lens) | [`392abe75`](https://github.com/cyberia-to/lens/commit/392abe752793c3e0b98f2d1d57245ba1e3173673) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [mir](https://github.com/cyberia-to/mir) | [`b9c832d5`](https://github.com/cyberia-to/mir/commit/b9c832d5011d5a719bec6021afef123be35490ab) | 🟢 Matches | mir @ 0.1.0 |
| [mudra](https://github.com/cyberia-to/mudra) | [`e2d1a63b`](https://github.com/cyberia-to/mudra/commit/e2d1a63bccd760b96ba55f448ef338b7ab027c37) | 🟢 Matches | cyber-mudra @ 0.1.0 |
| [neuron](https://github.com/cyberia-to/neuron) | [`c8754112`](https://github.com/cyberia-to/neuron/commit/c87541120caae4fb5d5bb401653d1cbcbeffeb7d) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [nox](https://github.com/cyberia-to/nox) | [`2612673c`](https://github.com/cyberia-to/nox/commit/2612673cae64239cb3511b13027d9ffaba77f48e) | 🟢 Matches | cyber-nox @ 0.2.0 |
| [nu](https://github.com/cyberia-to/nu) | ⚪ Unavailable | 🔴 Missing / drifted | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [optica](https://github.com/cyberia-to/optica) | [`9f0c352c`](https://github.com/cyberia-to/optica/commit/9f0c352cfea30e1ac8ce0feb0d735ffb9fa47ace) | 🟢 Matches | optica @ 0.2.0 |
| [prysm](https://github.com/cyberia-to/prysm) | [`f66cbd82`](https://github.com/cyberia-to/prysm/commit/f66cbd82cada5f5e6315b674818bce705b962eb6) | 🟢 Matches | prysm @ 0.1.0 |
| [radio](https://github.com/cyberia-to/radio) | [`db1d62e2`](https://github.com/cyberia-to/radio/commit/db1d62e2cd1e4b2f309fa19bcc158bd29b44753d) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [rune](https://github.com/cyberia-to/rune) | [`67281db7`](https://github.com/cyberia-to/rune/commit/67281db727070c1160c12be341c6fd85e39e5380) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [soft3](https://github.com/cyberia-to/soft3) | [`c5cc2077`](https://github.com/cyberia-to/soft3/commit/c5cc2077df510610abb7b5bd69675af6048ed724) | 📌 Product source | soft3 @ 0.10.0 |
| [soma](https://github.com/cyberia-to/soma) | [`7cfa6f8a`](https://github.com/cyberia-to/soma/commit/7cfa6f8a602367ceedf45e0bd7e120884c9a6ae3) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [spark](https://github.com/cyberia-to/spark) | [`400cd9a1`](https://github.com/cyberia-to/spark/commit/400cd9a178792d02ec577ae74fc728fe6ee49ecd) | 🟢 Matches | cyber-spark @ 0.1.0 |
| [strata](https://github.com/cyberia-to/strata) | [`56aedb2d`](https://github.com/cyberia-to/strata/commit/56aedb2d12b3126c601eb333419136d403614dbb) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [tade](https://github.com/cyberia-to/tade) | [`b1f1c2ca`](https://github.com/cyberia-to/tade/commit/b1f1c2ca70cb4d65d32272f8ee15cf0657c9d485) | 🟢 Matches | [Declarations ↗](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) |
| [tok](https://github.com/cyberia-to/plumb) | [`e27da0df`](https://github.com/cyberia-to/plumb/commit/e27da0dff72b948418a5e40a5ed6290c0fd167f3) | 🟢 Matches | cyber-tok @ 0.1.1 |
| [tru](https://github.com/cyberia-to/tru) | [`89416291`](https://github.com/cyberia-to/tru/commit/89416291dd4cc993a79d63cbd3e2445294b173d3) | 🟢 Matches | cyber-tru @ 0.1.1 |
| [vault](https://github.com/cyberia-to/vault) | [`c9805574`](https://github.com/cyberia-to/vault/commit/c980557471184a78d43366e1c52fc978a3e82444) | 🟢 Matches | cyber-vault @ 0.1.0 |
| [zheng](https://github.com/cyberia-to/zheng) | [`44bd5bfe`](https://github.com/cyberia-to/zheng/commit/44bd5bfec5cb6764123ea513a33a26c0877a488c) | 🟢 Matches | zheng @ 0.3.3 |

Declared versions come from source manifests. Resolved package closure, resolution failures and all nested declarations are retained in [soft3-dependencies.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json).

</details>

## 📎 Evidence and downloads

[GitHub release assets](https://github.com/cyberia-to/cyb/releases) · linked audit copies retain the original checksums.

| Asset | Contents |
|---|---|
| [SHA256SUMS](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/SHA256SUMS) | Checksums for the original candidate assets |
| [candidate.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/candidate.json) | Product versions, exact source revisions and available binaries |
| [sources.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/sources.json) | Origin source capture, pins and change attribution |
| [release-validation.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/release-validation.json) | Every platform, command, verdict and log hash |
| [soft3-dependencies.json](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.json) | Machine-readable source and package inventories |
| [soft3-dependencies.md](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/soft3-dependencies.md) | Original readable dependency inventory |
| [release-notes.md](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/release-notes.md) | Original generated notes captured with this candidate |

🗂️ [Audit and reproduction commands](https://github.com/cyberia-to/cyb/blob/release/2026-09-24/audit/release-2026-09-24/README.md)

The verdict, source revisions and qualification data above come from the linked candidate receipts. This presentation does not change their results.
