//! Who this cyb is — a keypair, not an environment variable.
//!
//! The neuron every world signs as used to be `$USER` padded with zeros: a
//! placeholder that made "the robot belongs to its owner" a sentence about
//! nobody. Identity is now derived the way the rest of cyber derives it —
//! mudra's pipeline, BIP-39 mnemonic → secp256k1 key → `neuron_of(pubkey)` —
//! so the neuron is the fingerprint of a key only the owner holds, and the
//! same mnemonic reproduces the same identity on any body.
//!
//! The mnemonic lives in `~/cyb/mnemonic`, mode 0600, generated on first
//! boot. That file IS the owner: back it up and the identity survives the
//! machine; lose it and nobody — by construction — can produce the neuron
//! again. Signals are not yet signed (the chain format carries no signature
//! field today); what the keypair buys now is a neuron that is *claimable* —
//! the day chains verify, this identity already has a key behind it.

use bevy::prelude::*;

/// The spacepussy bech32 prefix — the nearest chain this cyb will join,
/// and the address format its owner will actually see elsewhere.
const HRP: &str = "pussy";

#[derive(Resource, Clone)]
pub struct Identity {
    /// The neuron all of this cyb's casts ride on.
    pub neuron: [u8; 32],
    /// bech32 account address for the same key, `pussy1...`.
    pub address: String,
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
}

fn mnemonic_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("mnemonic")
}

/// Load the identity, minting one on first boot.
///
/// Failure never takes the app down: a body that cannot write its mnemonic
/// (sandbox, read-only home) runs as an ephemeral identity for the session —
/// honest about it in the log — rather than refusing to start.
pub fn load_or_mint() -> Identity {
    let path = mnemonic_path();

    let mnemonic = match std::fs::read_to_string(&path) {
        Ok(m) if !m.trim().is_empty() => m.trim().to_string(),
        _ => {
            let minted = mudra::seed::generate_mnemonic()
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
                info!("identity: minted a new mnemonic at {}", path.display());
            }
            minted
        }
    };

    match identity_from_mnemonic(&mnemonic) {
        Some(id) => {
            info!("identity: {}", id.short());
            id
        }
        None => {
            warn!("identity: no usable mnemonic — running ephemeral for this session");
            Identity {
                neuron: super::local_neuron(),
                address: "ephemeral".into(),
            }
        }
    }
}

fn identity_from_mnemonic(mnemonic: &str) -> Option<Identity> {
    if mnemonic.is_empty() {
        return None;
    }
    let key = mudra::seed::cosmos_key(mnemonic, "").ok()?;
    let pubkey = mudra::cosmos::compressed(&key.verifying_key());
    let neuron = mudra::claim::neuron_of(&pubkey);
    let address = mudra::cosmos::address(&pubkey, HRP).ok()?;
    Some(Identity { neuron, address })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same words are the same being, everywhere, forever.
    #[test]
    fn identity_is_a_pure_function_of_the_mnemonic() {
        let m = "abandon abandon abandon abandon abandon abandon abandon \
                 abandon abandon abandon abandon about";
        let a = identity_from_mnemonic(m).expect("derive");
        let b = identity_from_mnemonic(m).expect("derive");
        assert_eq!(a.neuron, b.neuron);
        assert_eq!(a.address, b.address);
        assert!(a.address.starts_with("pussy1"), "address: {}", a.address);
        assert_ne!(a.neuron, [0u8; 32]);
    }

    #[test]
    fn different_words_are_different_beings() {
        let a = identity_from_mnemonic(
            "abandon abandon abandon abandon abandon abandon abandon \
             abandon abandon abandon abandon about",
        )
        .unwrap();
        let b = identity_from_mnemonic(
            "legal winner thank year wave sausage worth useful legal \
             winner thank yellow",
        )
        .unwrap();
        assert_ne!(a.neuron, b.neuron);
        assert_ne!(a.address, b.address);
    }
}
