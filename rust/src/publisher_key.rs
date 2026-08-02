//! Publisher signing keys backed by the operating system credential store.

use crate::tap;
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use std::fs;

const SERVICE: &str = "org.hara-lang.publisher";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublisherKey {
    pub id: String,
    pub public_key: String,
    secret: [u8; 32],
}

impl PublisherKey {
    pub fn generate() -> Self {
        Self::from_signing_key(SigningKey::generate(&mut OsRng))
    }

    fn from_signing_key(signing_key: SigningKey) -> Self {
        let public = signing_key.verifying_key().to_bytes();
        Self {
            id: format!("sha256:{}", hex(&Sha256::digest(public))),
            public_key: hex(&public),
            secret: signing_key.to_bytes(),
        }
    }

    pub fn sign(&self, bytes: &[u8]) -> String {
        hex(&SigningKey::from_bytes(&self.secret).sign(bytes).to_bytes())
    }

    pub fn store_as_active(&self) -> Result<(), String> {
        keyring::Entry::new(SERVICE, &self.id)
            .map_err(keyring_error)?
            .set_secret(&self.secret)
            .map_err(keyring_error)?;
        let root = tap::config_root();
        fs::create_dir_all(&root).map_err(io_error)?;
        fs::write(root.join("active-publisher-key"), format!("{}\n", self.id)).map_err(io_error)
    }

    pub fn active() -> Result<Self, String> {
        let id = fs::read_to_string(tap::config_root().join("active-publisher-key"))
            .map_err(|_| "no active publisher key; run `hara id key create`".to_owned())?;
        let id = id.trim();
        let secret = keyring::Entry::new(SERVICE, id)
            .map_err(keyring_error)?
            .get_secret()
            .map_err(keyring_error)?;
        let secret: [u8; 32] = secret
            .try_into()
            .map_err(|_| "publisher keychain entry is not an Ed25519 seed".to_owned())?;
        let key = Self::from_signing_key(SigningKey::from_bytes(&secret));
        if key.id != id {
            return Err("publisher keychain entry does not match its key ID".into());
        }
        Ok(key)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn keyring_error(error: keyring::Error) -> String {
    format!("OS credential store: {error}")
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    #[test]
    fn generated_key_ids_bind_public_keys_and_signatures() {
        let key = PublisherKey::generate();
        assert_eq!(key.public_key.len(), 64);
        assert!(key.id.starts_with("sha256:"));
        let public: [u8; 32] = decode(&key.public_key).try_into().unwrap();
        let signature: [u8; 64] = decode(&key.sign(b"publish intent")).try_into().unwrap();
        VerifyingKey::from_bytes(&public)
            .unwrap()
            .verify(b"publish intent", &Signature::from_bytes(&signature))
            .unwrap();
    }

    fn decode(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
}
