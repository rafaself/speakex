use keyring::{Entry, Error as KeyringError};
use std::sync::Arc;

const SECRET_SERVICE_NAME: &str = "com.rafaself.speakex";
const GEMINI_API_KEY_ACCOUNT: &str = "gemini-api-key";

#[derive(Clone)]
pub struct SecretStoreService {
    backend: Arc<dyn SecretStorageBackend>,
}

impl Default for SecretStoreService {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStoreService {
    pub fn new() -> Self {
        Self::with_backend(Arc::new(KeyringSecretStorageBackend))
    }

    fn with_backend(backend: Arc<dyn SecretStorageBackend>) -> Self {
        Self { backend }
    }

    pub fn save_gemini_api_key(&self, api_key: &str) -> Result<(), String> {
        let normalized_api_key = api_key.trim();
        if normalized_api_key.is_empty() {
            return Err("Gemini API key cannot be empty".to_string());
        }

        self.backend
            .set_password(
                SECRET_SERVICE_NAME,
                GEMINI_API_KEY_ACCOUNT,
                normalized_api_key,
            )
            .map_err(|error| match error {
                SecretStorageBackendError::NoStorageAccess => {
                    "secure storage is unavailable".to_string()
                }
                SecretStorageBackendError::InvalidInput => {
                    "secure storage rejected the Gemini API key".to_string()
                }
                SecretStorageBackendError::Unexpected | SecretStorageBackendError::NotFound => {
                    "secure storage failed while saving the Gemini API key".to_string()
                }
            })
    }

    pub(crate) fn read_gemini_api_key(&self) -> Result<String, String> {
        match self
            .backend
            .get_password(SECRET_SERVICE_NAME, GEMINI_API_KEY_ACCOUNT)
        {
            Ok(api_key) => {
                let normalized_api_key = api_key.trim();
                if normalized_api_key.is_empty() {
                    Err("Gemini API key is not configured".to_string())
                } else {
                    Ok(normalized_api_key.to_string())
                }
            }
            Err(SecretStorageBackendError::NotFound) => {
                Err("Gemini API key is not configured".to_string())
            }
            Err(SecretStorageBackendError::NoStorageAccess) => {
                Err("secure storage is unavailable".to_string())
            }
            Err(SecretStorageBackendError::InvalidInput)
            | Err(SecretStorageBackendError::Unexpected) => {
                Err("secure storage failed while reading the Gemini API key".to_string())
            }
        }
    }

    pub fn has_gemini_api_key(&self) -> Result<bool, String> {
        match self
            .backend
            .get_password(SECRET_SERVICE_NAME, GEMINI_API_KEY_ACCOUNT)
        {
            Ok(api_key) => Ok(!api_key.trim().is_empty()),
            Err(SecretStorageBackendError::NotFound) => Ok(false),
            Err(SecretStorageBackendError::NoStorageAccess) => {
                Err("secure storage is unavailable".to_string())
            }
            Err(SecretStorageBackendError::InvalidInput)
            | Err(SecretStorageBackendError::Unexpected) => {
                Err("secure storage failed while checking the Gemini API key".to_string())
            }
        }
    }

    pub fn clear_gemini_api_key(&self) -> Result<bool, String> {
        match self
            .backend
            .delete_password(SECRET_SERVICE_NAME, GEMINI_API_KEY_ACCOUNT)
        {
            Ok(()) => Ok(true),
            Err(SecretStorageBackendError::NotFound) => Ok(false),
            Err(SecretStorageBackendError::NoStorageAccess) => {
                Err("secure storage is unavailable".to_string())
            }
            Err(SecretStorageBackendError::InvalidInput)
            | Err(SecretStorageBackendError::Unexpected) => {
                Err("secure storage failed while clearing the Gemini API key".to_string())
            }
        }
    }
}

trait SecretStorageBackend: Send + Sync {
    fn set_password(
        &self,
        service: &str,
        account: &str,
        password: &str,
    ) -> Result<(), SecretStorageBackendError>;

    fn get_password(
        &self,
        service: &str,
        account: &str,
    ) -> Result<String, SecretStorageBackendError>;

    fn delete_password(
        &self,
        service: &str,
        account: &str,
    ) -> Result<(), SecretStorageBackendError>;
}

#[derive(Debug)]
struct KeyringSecretStorageBackend;

impl KeyringSecretStorageBackend {
    fn entry(&self, service: &str, account: &str) -> Result<Entry, SecretStorageBackendError> {
        Entry::new(service, account).map_err(Self::map_error)
    }

    fn map_error(error: KeyringError) -> SecretStorageBackendError {
        match error {
            KeyringError::NoEntry => SecretStorageBackendError::NotFound,
            KeyringError::NoStorageAccess(_) => SecretStorageBackendError::NoStorageAccess,
            KeyringError::TooLong(_, _) | KeyringError::Invalid(_, _) => {
                SecretStorageBackendError::InvalidInput
            }
            KeyringError::PlatformFailure(_)
            | KeyringError::BadEncoding(_)
            | KeyringError::Ambiguous(_) => SecretStorageBackendError::Unexpected,
            _ => SecretStorageBackendError::Unexpected,
        }
    }
}

impl SecretStorageBackend for KeyringSecretStorageBackend {
    fn set_password(
        &self,
        service: &str,
        account: &str,
        password: &str,
    ) -> Result<(), SecretStorageBackendError> {
        self.entry(service, account)?
            .set_password(password)
            .map_err(Self::map_error)
    }

    fn get_password(
        &self,
        service: &str,
        account: &str,
    ) -> Result<String, SecretStorageBackendError> {
        self.entry(service, account)?
            .get_password()
            .map_err(Self::map_error)
    }

    fn delete_password(
        &self,
        service: &str,
        account: &str,
    ) -> Result<(), SecretStorageBackendError> {
        self.entry(service, account)?
            .delete_password()
            .map_err(Self::map_error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SecretStorageBackendError {
    NotFound,
    NoStorageAccess,
    InvalidInput,
    Unexpected,
}

#[cfg(test)]
mod tests {
    use super::{SecretStorageBackend, SecretStorageBackendError, SecretStoreService};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    #[derive(Default)]
    struct FakeSecretStorageBackend {
        password: Mutex<Option<String>>,
        set_error: Mutex<Option<SecretStorageBackendError>>,
        get_error: Mutex<Option<SecretStorageBackendError>>,
        delete_error: Mutex<Option<SecretStorageBackendError>>,
        set_calls: AtomicUsize,
    }

    impl SecretStorageBackend for FakeSecretStorageBackend {
        fn set_password(
            &self,
            _service: &str,
            _account: &str,
            password: &str,
        ) -> Result<(), SecretStorageBackendError> {
            self.set_calls.fetch_add(1, Ordering::Relaxed);
            if let Some(error) = self
                .set_error
                .lock()
                .expect("set_error lock should succeed")
                .take()
            {
                return Err(error);
            }

            *self.password.lock().expect("password lock should succeed") =
                Some(password.to_string());
            Ok(())
        }

        fn get_password(
            &self,
            _service: &str,
            _account: &str,
        ) -> Result<String, SecretStorageBackendError> {
            if let Some(error) = self
                .get_error
                .lock()
                .expect("get_error lock should succeed")
                .take()
            {
                return Err(error);
            }

            self.password
                .lock()
                .expect("password lock should succeed")
                .clone()
                .ok_or(SecretStorageBackendError::NotFound)
        }

        fn delete_password(
            &self,
            _service: &str,
            _account: &str,
        ) -> Result<(), SecretStorageBackendError> {
            if let Some(error) = self
                .delete_error
                .lock()
                .expect("delete_error lock should succeed")
                .take()
            {
                return Err(error);
            }

            let mut password = self.password.lock().expect("password lock should succeed");
            if password.take().is_some() {
                Ok(())
            } else {
                Err(SecretStorageBackendError::NotFound)
            }
        }
    }

    fn secret_store_service_for_tests() -> (SecretStoreService, Arc<FakeSecretStorageBackend>) {
        let backend = Arc::new(FakeSecretStorageBackend::default());
        let service = SecretStoreService::with_backend(backend.clone());
        (service, backend)
    }

    #[test]
    fn save_has_and_clear_gemini_api_key_flow() {
        let (service, backend) = secret_store_service_for_tests();

        service
            .save_gemini_api_key("  gemini-secret-token  ")
            .expect("saving Gemini API key should succeed");

        assert_eq!(
            backend
                .password
                .lock()
                .expect("password lock should succeed")
                .as_deref(),
            Some("gemini-secret-token")
        );
        assert!(service
            .has_gemini_api_key()
            .expect("checking Gemini API key presence should succeed"));
        assert!(service
            .clear_gemini_api_key()
            .expect("clearing Gemini API key should succeed"));
        assert!(!service
            .has_gemini_api_key()
            .expect("checking empty Gemini API key presence should succeed"));
    }

    #[test]
    fn save_gemini_api_key_rejects_empty_values_before_backend_access() {
        let (service, backend) = secret_store_service_for_tests();

        let error = service
            .save_gemini_api_key("   ")
            .expect_err("empty Gemini API key should be rejected");

        assert_eq!(error, "Gemini API key cannot be empty");
        assert_eq!(backend.set_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn clear_gemini_api_key_returns_false_when_missing() {
        let (service, _backend) = secret_store_service_for_tests();

        assert!(!service
            .clear_gemini_api_key()
            .expect("clearing a missing Gemini API key should succeed"));
    }

    #[test]
    fn read_gemini_api_key_returns_trimmed_secret() {
        let (service, _backend) = secret_store_service_for_tests();

        service
            .save_gemini_api_key("  gemini-secret-token  ")
            .expect("saving Gemini API key should succeed");

        assert_eq!(
            service
                .read_gemini_api_key()
                .expect("reading Gemini API key should succeed"),
            "gemini-secret-token"
        );
    }

    #[test]
    fn read_gemini_api_key_reports_missing_secret() {
        let (service, _backend) = secret_store_service_for_tests();

        let error = service
            .read_gemini_api_key()
            .expect_err("missing Gemini API key should be reported");

        assert_eq!(error, "Gemini API key is not configured");
    }

    #[test]
    fn has_gemini_api_key_reports_secure_storage_access_errors() {
        let (service, backend) = secret_store_service_for_tests();
        *backend
            .get_error
            .lock()
            .expect("get_error lock should succeed") =
            Some(SecretStorageBackendError::NoStorageAccess);

        let error = service
            .has_gemini_api_key()
            .expect_err("storage access failures should be returned");

        assert_eq!(error, "secure storage is unavailable");
    }
}
