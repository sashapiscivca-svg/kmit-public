//! Помилки API. Код стабільний (для оболонки й інтерфейсу), текст для людини українською.

use serde_json::{json, Value};
use storage::StorageError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(code: &'static str, message: &str) -> Self {
        ApiError {
            code,
            message: message.to_string(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({ "error": { "code": self.code, "message": self.message } })
    }

    pub(crate) fn no_profile_open() -> Self {
        ApiError::new("no_profile_open", "Спершу відкрий профіль")
    }

    pub(crate) fn bad_request(what: &str) -> Self {
        ApiError::new("bad_request", what)
    }

    pub(crate) fn wrong_phase() -> Self {
        ApiError::new("wrong_phase", "Зараз так зробити не можна")
    }
}

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        let code = match e {
            StorageError::WrongPin => "wrong_pin",
            StorageError::InvalidPin => "invalid_pin",
            StorageError::WrongRecoveryCode => "wrong_recovery_code",
            StorageError::WrongPassphrase => "wrong_passphrase",
            StorageError::DataLost => "profile_data_lost",
            StorageError::NewerFormat => "newer_format",
            StorageError::Locked(_) => "pin_locked",
            StorageError::NoVault | StorageError::NoSuchProfile => "no_such_profile",
            StorageError::TooManyProfiles => "too_many_profiles",
            StorageError::KeyHook(_) => "device_protection",
            _ => "storage",
        };
        ApiError {
            code,
            message: e.to_string(),
        }
    }
}
