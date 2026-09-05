//! The vault's disk form: one sealed file, opened by the owner's words.
//!
//! `~/cyb/vault.enc` is XChaCha20-Poly1305 over a JSON list of entries.
//! The key is derived from the same mnemonic that is the cyb identity
//! (`SHA-256("cyb-vault-v1" || BIP-39 seed)`) — the twelve words ARE the
//! vault key, so the same words open the same vault on any body, and
//! there is no second secret to back up. A random 24-byte nonce is drawn
//! fresh on every save; nonce reuse under a fixed key is the one way this
//! construction breaks, so uniqueness comes from the OS, not a counter.
//!
//! Nothing in this module touches the cybergraph. Secrets do not cast.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};

const MAGIC: &[u8] = b"CYBVLT1\n";
const NONCE_LEN: usize = 24;

/// What kind of secret an entry holds. The kind decides how the vault
/// page treats it: an `otp` entry renders a live code, everything else
/// copies its value.
pub const KINDS: [&str; 5] = ["password", "key", "seed", "otp", "custom"];

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub kind: String,
    pub value: String,
    /// Unix seconds when the entry was sealed.
    pub created: u64,
}

fn vault_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("vault.enc")
}

fn mnemonic_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("mnemonic")
}

/// The vault key, derived from the identity mnemonic. `None` when this
/// body has no identity file — the vault cannot exist without an owner.
pub fn key() -> Option<[u8; 32]> {
    let mnemonic = std::fs::read_to_string(mnemonic_path()).ok()?;
    let mnemonic = mnemonic.trim();
    if mnemonic.is_empty() {
        return None;
    }
    let seed = mudra::seed::seed(mnemonic, "").ok()?;
    let mut h = Sha256::new();
    h.update(b"cyb-vault-v1");
    h.update(seed);
    Some(h.finalize().into())
}

pub fn load(key: &[u8; 32]) -> Result<Vec<Entry>, String> {
    let raw = match std::fs::read(vault_path()) {
        Ok(r) => r,
        // No file yet is an empty vault, not an error.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let rest = raw.strip_prefix(MAGIC).ok_or("vault: not a vault file")?;
    if rest.len() < NONCE_LEN {
        return Err("vault: truncated".into());
    }
    let (nonce, ct) = rest.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(key.into());
    let plain = cipher
        .decrypt(XNonce::from_slice(nonce), ct)
        .map_err(|_| "vault: wrong key or corrupted file".to_string())?;
    parse(&plain)
}

pub fn save(key: &[u8; 32], entries: &[Entry]) -> Result<(), String> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|e| format!("vault: entropy: {e}"))?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let plain = serialize(entries);
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), plain.as_bytes())
        .map_err(|_| "vault: encrypt failed".to_string())?;

    let path = vault_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut blob = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
    blob.extend_from_slice(MAGIC);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ct);
    // Write-then-rename so a crash mid-save cannot eat the old vault.
    let tmp = path.with_extension("enc.tmp");
    std::fs::write(&tmp, &blob).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn serialize(entries: &[Entry]) -> String {
    let list: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "kind": e.kind,
                "value": e.value,
                "created": e.created,
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

fn parse(plain: &[u8]) -> Result<Vec<Entry>, String> {
    let v: serde_json::Value =
        serde_json::from_slice(plain).map_err(|e| format!("vault: decode: {e}"))?;
    let arr = v.as_array().ok_or("vault: not a list")?;
    let field = |o: &serde_json::Value, k: &str| -> String {
        o.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string()
    };
    Ok(arr
        .iter()
        .map(|o| Entry {
            name: field(o, "name"),
            kind: field(o, "kind"),
            value: field(o, "value"),
            created: o.get("created").and_then(|v| v.as_u64()).unwrap_or(0),
        })
        .collect())
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// RFC 6238 TOTP for an `otp` entry: (six digits, seconds until they die).
/// The secret is the usual base32 blob authenticator apps exchange.
pub fn totp(secret_b32: &str, unix: u64) -> Option<(String, u64)> {
    use hmac::{Hmac, Mac};
    let cleaned: String = secret_b32
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .collect();
    let key = base32::decode(
        base32::Alphabet::Rfc4648 { padding: false },
        &cleaned.to_ascii_uppercase(),
    )?;
    let step = unix / 30;
    let mut mac = <Hmac<sha1::Sha1> as Mac>::new_from_slice(&key).ok()?;
    mac.update(&step.to_be_bytes());
    let h = mac.finalize().into_bytes();
    let off = (h[19] & 0xf) as usize;
    let code = u32::from_be_bytes([h[off] & 0x7f, h[off + 1], h[off + 2], h[off + 3]]) % 1_000_000;
    Some((format!("{code:06}"), 30 - unix % 30))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_open_round_trips() {
        let key = [7u8; 32];
        let entries = vec![
            Entry { name: "gh".into(), kind: "password".into(), value: "hunter2".into(), created: 1 },
            Entry {
                name: "hot".into(),
                kind: "seed".into(),
                value: "abandon abandon about".into(),
                created: 2,
            },
        ];
        let blob = {
            // Round-trip through the serializers without touching disk.
            let cipher = XChaCha20Poly1305::new((&key).into());
            let nonce = [9u8; NONCE_LEN];
            let ct = cipher.encrypt(XNonce::from_slice(&nonce), serialize(&entries).as_bytes()).unwrap();
            let mut b = MAGIC.to_vec();
            b.extend_from_slice(&nonce);
            b.extend_from_slice(&ct);
            b
        };
        let rest = blob.strip_prefix(MAGIC).unwrap();
        let (nonce, ct) = rest.split_at(NONCE_LEN);
        let cipher = XChaCha20Poly1305::new((&key).into());
        let plain = cipher.decrypt(XNonce::from_slice(nonce), ct).unwrap();
        assert_eq!(parse(&plain).unwrap(), entries);
    }

    #[test]
    fn wrong_key_opens_nothing() {
        let cipher = XChaCha20Poly1305::new((&[1u8; 32]).into());
        let nonce = [0u8; NONCE_LEN];
        let ct = cipher.encrypt(XNonce::from_slice(&nonce), b"[]".as_slice()).unwrap();
        let other = XChaCha20Poly1305::new((&[2u8; 32]).into());
        assert!(other.decrypt(XNonce::from_slice(&nonce), ct.as_slice()).is_err());
    }

    /// RFC 6238 Appendix B test vector (SHA-1, ASCII key "12345678901234567890"
    /// = base32 GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ), T = 59s -> 94287082.
    #[test]
    fn totp_matches_rfc_6238() {
        let (code, left) = totp("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ", 59).unwrap();
        assert_eq!(code, "287082"); // six-digit truncation of 94287082
        assert_eq!(left, 1);
    }

    #[test]
    fn totp_rejects_garbage() {
        assert!(totp("not base32 at all!!!", 59).is_none());
    }
}
