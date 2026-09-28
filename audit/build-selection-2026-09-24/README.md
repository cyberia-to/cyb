# cyb build selection

`release/soft3.toml` selects soft3 `candidate-20260924.1`, release ID 395482953,
source `c5cc2077df510610abb7b5bd69675af6048ed724`, by its original SHA256SUMS digest.
The workflow uses engine [soft3 `145a3a2c`](https://github.com/cyberia-to/soft3/commit/145a3a2ca8319e29461cf559ca286d7e8d91c59a).

[Shared verification](https://github.com/cyberia-to/soft3/blob/145a3a2ca8319e29461cf559ca286d7e8d91c59a/audit/build-selection-2026-09-24/README.md)
records the actual asset download, checksum verification and preserved RED
verdict, plus clean upstream Nu source verification. The shared implementation
passes 34 integrity/declaration/build-selection tests with
`python3 -m unittest discover -s release -p 'test_*.py' -v` at code revision
`838450a` (the subsequent engine commit adds the Nu pin and verification files).

`actionlint -shellcheck= .github/workflows/release-train.yml` passes with
Actionlint v1.7.12. Both products' TOML contracts are identical and parse through
`train_build.contract`; both workflow references equal the engine revision above.
These changes select and authenticate release inputs. Native product acceptance
and the selected upstream build remain RED; no candidate assets are replaced,
no candidate is promoted, and default branches remain frozen.
