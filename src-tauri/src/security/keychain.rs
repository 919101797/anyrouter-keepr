use keyring::Entry;
use thiserror::Error;

const SERVICE: &str = "anyrouter-claude-keeper";

#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain error: {0}")]
    Keyring(#[from] keyring::Error),
}

pub fn set_token(profile_id: &str, token: &str) -> Result<(), KeychainError> {
    Entry::new(SERVICE, profile_id)?.set_password(token)?;
    Ok(())
}

pub fn get_token(profile_id: &str) -> Result<Option<String>, KeychainError> {
    match Entry::new(SERVICE, profile_id)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn delete_token(profile_id: &str) -> Result<(), KeychainError> {
    match Entry::new(SERVICE, profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
