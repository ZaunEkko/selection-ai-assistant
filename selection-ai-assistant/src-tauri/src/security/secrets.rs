use std::collections::HashMap;
use std::sync::Mutex;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("secret not found: {0}")]
    NotFound(String),
    #[error("secret storage failed: {0}")]
    Storage(String),
}

pub trait SecretStore: Send + Sync {
    fn set_secret(&self, key_ref: &str, value: &str) -> Result<(), SecretError>;
    fn get_secret(&self, key_ref: &str) -> Result<String, SecretError>;
    fn delete_secret(&self, key_ref: &str) -> Result<(), SecretError>;
}

pub struct MemorySecretStore {
    values: Mutex<HashMap<String, String>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemorySecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MemorySecretStore {
    fn set_secret(&self, key_ref: &str, value: &str) -> Result<(), SecretError> {
        self.values
            .lock()
            .map_err(|err| SecretError::Storage(err.to_string()))?
            .insert(key_ref.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key_ref: &str) -> Result<String, SecretError> {
        self.values
            .lock()
            .map_err(|err| SecretError::Storage(err.to_string()))?
            .get(key_ref)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(key_ref.to_string()))
    }

    fn delete_secret(&self, key_ref: &str) -> Result<(), SecretError> {
        self.values
            .lock()
            .map_err(|err| SecretError::Storage(err.to_string()))?
            .remove(key_ref);
        Ok(())
    }
}
