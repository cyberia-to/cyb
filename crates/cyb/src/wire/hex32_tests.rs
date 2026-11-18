//! Tests for `wire`'s `hex32` parser. A child module of `wire`, so it can
//! reach `hex32` and `hex` while keeping them private to the crate.

use super::*;

#[test]
fn round_trips_hex() {
    let n = [9u8; 32];
    assert_eq!(hex32(&hex(&n)), Some(n));
}

#[test]
fn rejects_non_ascii_without_panicking() {
    // The repl's `follow <64-hex neuron>` command feeds operator-typed
    // stdin straight into hex32. "a" + a 3-byte '中' + 60 more ASCII bytes
    // is 64 bytes (passed the old length check) but the fixed byte-index
    // slicing (`&s[i*2..i*2+2]`) lands its first chunk inside '中' — a byte
    // index that is not a char boundary — and panics instead of returning
    // None.
    let s = format!("a中{}", "a".repeat(60));
    assert_eq!(s.len(), 64);
    assert_eq!(hex32(&s), None);
}

#[test]
fn rejects_wrong_length_and_bad_digits() {
    assert_eq!(hex32("00"), None);
    assert_eq!(hex32(&"zz".repeat(32)), None);
}
