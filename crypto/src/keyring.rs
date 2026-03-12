use crate::CryptoError;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// An Ed25519 key pair for signing packages.
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new random key pair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Export the full keypair as bytes (secret key bytes).
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Import a keypair from secret key bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("key must be 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Self { signing_key })
    }

    /// Get the signing key reference.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Get the public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// Save key pair to a file (base64-encoded secret key).
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            self.to_bytes(),
        );
        fs::write(path, encoded)?;
        Ok(())
    }

    /// Load key pair from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, CryptoError> {
        let encoded = fs::read_to_string(path)?;
        let bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            encoded.trim(),
        )
        .map_err(|e| CryptoError::InvalidKey(format!("invalid base64: {e}")))?;
        Self::from_bytes(&bytes)
    }
}

/// An Ed25519 public key for verifying signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicKey {
    #[serde(
        serialize_with = "serialize_verifying_key",
        deserialize_with = "deserialize_verifying_key"
    )]
    verifying_key: VerifyingKey,
}

fn serialize_verifying_key<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        key.as_bytes(),
    );
    serializer.serialize_str(&encoded)
}

fn deserialize_verifying_key<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &s)
        .map_err(serde::de::Error::custom)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| serde::de::Error::custom("key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
}

impl PublicKey {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("public key must be 32 bytes".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid public key: {e}")))?;
        Ok(Self { verifying_key })
    }

    /// Get the verifying key reference.
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Export as bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.verifying_key.as_bytes()
    }

    /// Export as base64 string.
    pub fn to_base64(&self) -> String {
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            self.as_bytes(),
        )
    }

    /// Import from base64 string.
    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s.trim())
            .map_err(|e| CryptoError::InvalidKey(format!("invalid base64: {e}")))?;
        Self::from_bytes(&bytes)
    }

    /// Save public key to a file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        fs::write(path, self.to_base64())?;
        Ok(())
    }

    /// Load public key from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, CryptoError> {
        let encoded = fs::read_to_string(path)?;
        Self::from_base64(&encoded)
    }
}

/// A collection of trusted public keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRing {
    keys: HashMap<String, PublicKey>,
}

impl KeyRing {
    /// Create an empty keyring.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Add a named public key.
    pub fn add(&mut self, name: String, key: PublicKey) {
        self.keys.insert(name, key);
    }

    /// Remove a key by name.
    pub fn remove(&mut self, name: &str) -> Option<PublicKey> {
        self.keys.remove(name)
    }

    /// Get a key by name.
    pub fn get(&self, name: &str) -> Option<&PublicKey> {
        self.keys.get(name)
    }

    /// List all key names.
    pub fn list(&self) -> Vec<&str> {
        self.keys.keys().map(|s| s.as_str()).collect()
    }

    /// Get all keys.
    pub fn all_keys(&self) -> &HashMap<String, PublicKey> {
        &self.keys
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Save keyring to a file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CryptoError::InvalidKey(format!("serialization error: {e}")))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load keyring from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, CryptoError> {
        let json = fs::read_to_string(path)?;
        let keyring: Self = serde_json::from_str(&json)
            .map_err(|e| CryptoError::InvalidKey(format!("deserialization error: {e}")))?;
        Ok(keyring)
    }
}

impl Default for KeyRing {
    fn default() -> Self {
        Self::new()
    }
}
