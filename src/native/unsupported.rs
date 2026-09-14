//! Stub for desktop / web — this crate is mobile-only.

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::user::User;

pub(crate) fn initialize(_options: &FirebaseOptions) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn sign_in_with_email(_email: &str, _password: &str) -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn create_user_with_email(_email: &str, _password: &str) -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn update_display_name(_display_name: &str) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn send_password_reset_email(_email: &str) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn sign_out() -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn current_user() -> Result<Option<User>, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn id_token(_force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn use_emulator(_host: &str, _port: u16) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn subscribe_auth_state() -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}
