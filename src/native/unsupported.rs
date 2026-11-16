//! Stub for desktop / web — this crate is mobile-only.

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::user::User;

pub(crate) fn initialize_app(_options: &FirebaseOptions) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn sign_in_with_email_and_password(
    _email: &str,
    _password: &str,
) -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn create_user_with_email_and_password(
    _email: &str,
    _password: &str,
) -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn update_display_name(_display_name: &str) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn update_photo_url(_photo_url: &str) -> Result<(), FirebaseError> {
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

pub(crate) fn get_id_token(_force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn use_auth_emulator(_host: &str, _port: u16) -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn auth_state_changes() -> Result<(), FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn sign_in_with_google() -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn link_with_google() -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

pub(crate) fn unlink(_provider_id: &str) -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
#[allow(dead_code)]
pub(crate) fn sign_in_with_apple() -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
#[allow(dead_code)]
pub(crate) fn link_with_apple() -> Result<User, FirebaseError> {
    Err(FirebaseError::UnsupportedPlatform)
}
