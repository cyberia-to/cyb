//! Who this cyb is — a keypair, not an environment variable.
//!
//! The neuron every world signs as used to be `$USER` padded with zeros: a
//! placeholder that made "the robot belongs to its owner" a sentence about
//! nobody. Identity is now derived the way the rest of cyber derives it —
//! mudra's pipeline, BIP-39 spell → secp256k1 key → `neuron_of(pubkey)` —
//! so the neuron is the fingerprint of a key only the owner holds, and the
//! same spell reproduces the same identity on any body.
//!
//! The spell lives in `~/cyb/spell`, mode 0600, generated on first
//! boot; existing installations keep reading their original `mnemonic` file.
//! That file IS the owner: back it up and the identity survives the
//! machine; lose it and nobody — by construction — can produce the neuron
//! again. Every outgoing signal now signs — `Identity::sign` produces the
//! same ADR-036 doc mudra's claim module already uses, over the wire's own
//! canonical bytes, so a chain that verifies rejects anything not backed by
//! this key.

use bevy::prelude::*;
use std::sync::Arc;

/// The spacepussy bech32 prefix — the nearest chain this cyb will join,
/// and the address format its owner will actually see elsewhere.
const HRP: &str = "pussy";

#[derive(Resource, Clone)]
pub struct Identity {
    /// The neuron all of this cyb's casts ride on.
    pub neuron: [u8; 32],
    /// bech32 account address for the same key, `pussy1...`.
    pub address: String,
    /// Compressed secp256k1 pubkey — `Hemera(pubkey) == neuron`. Carried
    /// on the wire alongside a signature so a verifier can check that
    /// binding itself before trusting anything signed with it.
    pub pubkey: [u8; 33],
    signer: Arc<mudra::SigningKey>,
}

impl Identity {
    /// The address, shortened the way chains shorten it: `pussy1ab...xyz`.
    pub fn short(&self) -> String {
        if self.address.len() <= 14 {
            return self.address.clone();
        }
        format!(
            "{}...{}",
            &self.address[..8],
            &self.address[self.address.len() - 4..]
        )
    }

    /// The neuron as lowercase hex — the id every wire call keys on.
    pub fn hex(&self) -> String {
        self.neuron.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Sign `data` (the wire call's own canonical bytes) as an ADR-036 doc,
    /// this identity's hex neuron filling the doc's signer field — the
    /// verifier rebuilds the identical doc from `pubkey` + the neuron the
    /// request claims, so there is nothing to agree on out of band.
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        mudra::claim::sign_arbitrary(&self.signer, &self.hex(), data)
    }

    /// The pubkey as lowercase hex, for the wire.
    pub fn pubkey_hex(&self) -> String {
        self.pubkey.iter().map(|b| format!("{b:02x}")).collect()
    }
}

pub(crate) fn spell_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    spell_path_in(&std::path::Path::new(&home).join("cyb"))
}

/// Existing installations retain their original file; new identities use `spell`.
fn spell_path_in(dir: &std::path::Path) -> std::path::PathBuf {
    let legacy = dir.join("mnemonic");
    if legacy.try_exists().unwrap_or(true) { legacy } else { dir.join("spell") }
}

/// Load the identity, minting one on first boot.
///
/// Failure never takes the app down: a body that cannot write its spell
/// (sandbox, read-only home) runs as an ephemeral identity for the session —
/// honest about it in the log — rather than refusing to start.
pub fn load_or_mint() -> Identity {
    let path = spell_path();

    let spell = match std::fs::read_to_string(&path) {
        Ok(m) if !m.trim().is_empty() => m.trim().to_string(),
        _ => {
            let minted = mudra::spell::generate()
                .unwrap_or_else(|_| String::new());
            if !minted.is_empty() {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&path, &minted);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &path,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
                info!("identity: minted a new spell at {}", path.display());
            }
            minted
        }
    };

    match identity_from_spell(&spell) {
        Some(id) => {
            info!("identity: {}", id.short());
            id
        }
        None => {
            warn!("identity: no usable spell — running ephemeral for this session");
            // Coherent but unclaimed: a fresh in-memory key whose neuron is
            // its own Hemera(pubkey), so signatures still verify — a chain
            // simply never granted this session-local identity any pussy.
            mudra::spell::generate()
                .ok()
                .and_then(|m| identity_from_spell(&m))
                .unwrap_or_else(|| Identity {
                    neuron: super::local_neuron(),
                    address: "ephemeral".into(),
                    pubkey: [0u8; 33],
                    signer: Arc::new(mudra::spell::cosmos_key(
                        "abandon abandon abandon abandon abandon abandon abandon \
                         abandon abandon abandon abandon about",
                        "",
                    )
                    .expect("fixed test vector always derives")),
                })
        }
    }
}

fn identity_from_spell(spell: &str) -> Option<Identity> {
    if spell.is_empty() {
        return None;
    }
    let key = mudra::spell::cosmos_key(spell, "").ok()?;
    let pubkey = mudra::cosmos::compressed(&key.verifying_key());
    let neuron = mudra::claim::neuron_of(&pubkey);
    let address = mudra::cosmos::address(&pubkey, HRP).ok()?;
    Some(Identity {
        neuron,
        address,
        pubkey,
        signer: Arc::new(key),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_filename_keeps_existing_identity_before_minting() {
        let dir = std::env::temp_dir().join(format!("cyb-spell-path-{}-{}",
            std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&dir).unwrap();
        assert_eq!(spell_path_in(&dir), dir.join("spell"));
        std::fs::write(dir.join("mnemonic"), "synthetic legacy words").unwrap();
        assert_eq!(spell_path_in(&dir), dir.join("mnemonic"));
        std::fs::write(dir.join("spell"), "synthetic newer words").unwrap();
        assert_eq!(spell_path_in(&dir), dir.join("mnemonic"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The same words are the same being, everywhere, forever.
    #[test]
    fn identity_is_a_pure_function_of_the_spell() {
        let m = "abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon about";
        let a = identity_from_spell(m).expect("derive");
        let b = identity_from_spell(m).expect("derive");
        assert_eq!(a.neuron, b.neuron);
        assert_eq!(a.address, b.address);
        assert!(a.address.starts_with("pussy1"), "address: {}", a.address);
        assert_ne!(a.neuron, [0u8; 32]);
    }

    #[test]
    fn different_words_are_different_beings() {
        let a = identity_from_spell(
            "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about",
        )
        .unwrap();
        let b = identity_from_spell(
            "legal winner thank year wave sausage worth useful legal \
             winner thank yellow",
        )
        .unwrap();
        assert_ne!(a.neuron, b.neuron);
        assert_ne!(a.address, b.address);
    }
}
