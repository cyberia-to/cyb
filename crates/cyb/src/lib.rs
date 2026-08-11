//! cyb — immortal robot runtime (product crate).
//!
//! Full cell / money / graph surface is staged in follow-up versions as the
//! dependency train lands on crates.io. This crate owns the **name** and the
//! public product entrypoints.

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product identity.
pub fn identity() -> &'static str {
    "cyb"
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_nonzero() {
        assert!(!super::VERSION.is_empty());
    }
}
