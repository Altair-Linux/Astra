use crate::CryptoError;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// an ed25519 key pair used for signing packages.
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// generates a fresh random key pair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// exports the keypair as raw bytes (secret key).
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// imports a keypair from secret key bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("key must be 32 bytes".into()))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        Ok(Self { signing_key })
    }

    /// returns a reference to the signing key.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// returns the public key for this keypair.
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// saves the keypair to a file as base64.
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.to_bytes());
        fs::write(path, encoded)?;
        Ok(())
    }

    /// loads a keypair from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, CryptoError> {
        let encoded = fs::read_to_string(path)?;
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded.trim())
                .map_err(|e| CryptoError::InvalidKey(format!("invalid base64: {e}")))?;
        Self::from_bytes(&bytes)
    }
}

/// an ed25519 public key for verifying package signatures.
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
    let encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key.as_bytes());
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
    /// creates a public key from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("public key must be 32 bytes".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| CryptoError::InvalidKey(format!("invalid public key: {e}")))?;
        Ok(Self { verifying_key })
    }

    /// returns a reference to the inner verifying key.
    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// exports as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.verifying_key.as_bytes()
    }

    /// exports as a base64 string.
    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.as_bytes())
    }

    /// imports from a base64 string.
    pub fn from_base64(s: &str) -> Result<Self, CryptoError> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s.trim())
            .map_err(|e| CryptoError::InvalidKey(format!("invalid base64: {e}")))?;
        Self::from_bytes(&bytes)
    }

    /// saves the public key to a file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        fs::write(path, self.to_base64())?;
        Ok(())
    }

    /// loads a public key from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, CryptoError> {
        let encoded = fs::read_to_string(path)?;
        Self::from_base64(&encoded)
    }
}

/// a collection of trusted public keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRing {
    keys: HashMap<String, PublicKey>,
}

impl KeyRing {
    /// creates an empty keyring.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// adds a named public key to the keyring.
    pub fn add(&mut self, name: String, key: PublicKey) {
        self.keys.insert(name, key);
    }

    /// removes a key by name.
    pub fn remove(&mut self, name: &str) -> Option<PublicKey> {
        self.keys.remove(name)
    }

    /// looks up a key by name.
    pub fn get(&self, name: &str) -> Option<&PublicKey> {
        self.keys.get(name)
    }

    /// lists all key names in the keyring.
    pub fn list(&self) -> Vec<&str> {
        self.keys.keys().map(|s| s.as_str()).collect()
    }

    /// returns all keys in the keyring.
    pub fn all_keys(&self) -> &HashMap<String, PublicKey> {
        &self.keys
    }

    /// returns true if the keyring has no keys.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// saves the keyring to a json file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), CryptoError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CryptoError::InvalidKey(format!("serialization error: {e}")))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// loads a keyring from a json file.
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
