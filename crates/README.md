# crates

## `cyb-reserve`

Minimal package used to **register the `cyb` name on crates.io** (placeholder `0.0.1`).

The real GUI package lives in `../shell` with `[package] name = "cyb"`.

```bash
# one-time: cargo login
cd crates/cyb-reserve
cargo publish
```

Not a workspace member — publish from this directory only.
