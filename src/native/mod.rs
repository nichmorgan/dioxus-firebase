//! Platform dispatch for native Firebase Auth calls.

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::user::{ProviderCredential, User};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod android_plugin;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
mod ios_plugin;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod unsupported;

macro_rules! native_call {
    ($method:ident ( $($arg:expr),* $(,)? )) => {{
        #[cfg(target_os = "android")]
        {
            android::$method($($arg),*)
        }
        #[cfg(target_os = "ios")]
        {
            ios::$method($($arg),*)
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            unsupported::$method($($arg),*)
        }
    }};
}

pub(crate) fn initialize_app(options: &FirebaseOptions) -> Result<(), FirebaseError> {
    native_call!(initialize_app(options))
}

pub(crate) fn sign_in_with_email_and_password(
    email: &str,
    password: &str,
) -> Result<User, FirebaseError> {
    native_call!(sign_in_with_email_and_password(email, password))
}

pub(crate) fn create_user_with_email_and_password(
    email: &str,
    password: &str,
) -> Result<User, FirebaseError> {
    native_call!(create_user_with_email_and_password(email, password))
}

pub(crate) fn update_display_name(display_name: &str) -> Result<(), FirebaseError> {
    native_call!(update_display_name(display_name))
}

pub(crate) fn update_photo_url(photo_url: &str) -> Result<(), FirebaseError> {
    native_call!(update_photo_url(photo_url))
}

pub(crate) fn send_password_reset_email(email: &str) -> Result<(), FirebaseError> {
    native_call!(send_password_reset_email(email))
}

pub(crate) fn sign_out() -> Result<(), FirebaseError> {
    native_call!(sign_out())
}

pub(crate) fn current_user() -> Result<Option<User>, FirebaseError> {
    native_call!(current_user())
}

pub(crate) fn get_id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    native_call!(get_id_token(force_refresh))
}

pub(crate) fn use_auth_emulator(host: &str, port: u16) -> Result<(), FirebaseError> {
    native_call!(use_auth_emulator(host, port))
}

pub(crate) fn auth_state_changes() -> Result<(), FirebaseError> {
    native_call!(auth_state_changes())
}

pub(crate) fn sign_in_with_google() -> Result<User, FirebaseError> {
    native_call!(sign_in_with_google())
}

pub(crate) fn link_with_google() -> Result<User, FirebaseError> {
    native_call!(link_with_google())
}

pub(crate) fn unlink(provider_id: &str) -> Result<User, FirebaseError> {
    native_call!(unlink(provider_id))
}

pub(crate) fn sign_in_with_apple() -> Result<User, FirebaseError> {
    native_call!(sign_in_with_apple())
}

pub(crate) fn link_with_apple() -> Result<User, FirebaseError> {
    native_call!(link_with_apple())
}

pub(crate) fn pick_google_credential() -> Result<ProviderCredential, FirebaseError> {
    native_call!(pick_google_credential())
}

pub(crate) fn pick_apple_credential() -> Result<ProviderCredential, FirebaseError> {
    native_call!(pick_apple_credential())
}

pub(crate) fn reload_current_user() -> Result<Option<User>, FirebaseError> {
    native_call!(reload_current_user())
}
