//! Platform dispatch for native Firebase Auth calls.

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::user::User;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod android_plugin;
#[cfg(target_os = "ios")]
mod ios;
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

pub(crate) fn initialize(options: &FirebaseOptions) -> Result<(), FirebaseError> {
    native_call!(initialize(options))
}

pub(crate) fn sign_in_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    native_call!(sign_in_with_email(email, password))
}

pub(crate) fn create_user_with_email(email: &str, password: &str) -> Result<User, FirebaseError> {
    native_call!(create_user_with_email(email, password))
}

pub(crate) fn update_display_name(display_name: &str) -> Result<(), FirebaseError> {
    native_call!(update_display_name(display_name))
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

pub(crate) fn id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    native_call!(id_token(force_refresh))
}

pub(crate) fn use_emulator(host: &str, port: u16) -> Result<(), FirebaseError> {
    native_call!(use_emulator(host, port))
}

pub(crate) fn subscribe_auth_state() -> Result<(), FirebaseError> {
    native_call!(subscribe_auth_state())
}
