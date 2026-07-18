use crate::ResomerError;

const KEYCHAIN_SERVICE: &str = "Resomer";
const KEYCHAIN_ACCOUNT: &str = "api_key";

pub struct KeychainManager;

impl KeychainManager {
    pub fn save_api_key(api_key: &str) -> Result<(), ResomerError> {
        keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|e| ResomerError::Keychain(format!("Failed to create entry: {}", e)))?
            .set_password(api_key)
            .map_err(|e| ResomerError::Keychain(format!("Failed to save: {}", e)))
    }

    pub fn get_api_key() -> Result<Option<String>, ResomerError> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|e| ResomerError::Keychain(format!("Failed to create entry: {}", e)))?;

        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ResomerError::Keychain(format!("Failed to retrieve: {}", e))),
        }
    }

    pub fn delete_api_key() -> Result<(), ResomerError> {
        keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|e| ResomerError::Keychain(format!("Failed to create entry: {}", e)))?
            .delete_password()
            .map_err(|e| {
                if let keyring::Error::NoEntry = e {
                    ResomerError::Keychain("Key not found".to_string())
                } else {
                    ResomerError::Keychain(format!("Failed to delete: {}", e))
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_constants() {
        assert_eq!(KEYCHAIN_SERVICE, "Resomer");
        assert_eq!(KEYCHAIN_ACCOUNT, "api_key");
    }
}
