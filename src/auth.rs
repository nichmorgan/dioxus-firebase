//! Email/password Auth primitives and token helpers.

use crate::error::FirebaseError;
use crate::host_protocol::{require_email, require_email_password};
use crate::init::require_initialized;
use crate::native;
use crate::user::User;

/// Blocks until the native Firebase Task completes — call from
/// `spawn_blocking` / a worker thread, not the UI thread or first paint.
pub fn sign_in_with_email(
    email: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<User, FirebaseError> {
    require_initialized()?;
    let email = email.as_ref();
    let password = password.as_ref();
    require_email_password(email, password)?;
    native::sign_in_with_email(email.trim(), password)
}

pub fn create_user_with_email(
    email: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<User, FirebaseError> {
    require_initialized()?;
    let email = email.as_ref();
    let password = password.as_ref();
    require_email_password(email, password)?;
    native::create_user_with_email(email.trim(), password)
}

pub fn update_display_name(display_name: impl AsRef<str>) -> Result<(), FirebaseError> {
    require_initialized()?;
    native::update_display_name(display_name.as_ref())?;
    // Auth state listeners do not fire for profile updates. Republish the
    // reloaded user so subscribers see display_name.
    if let Some(user) = native::current_user()? {
        crate::subscribe::dispatch_auth_state(Some(user));
    }
    Ok(())
}

pub fn send_password_reset_email(email: impl AsRef<str>) -> Result<(), FirebaseError> {
    require_initialized()?;
    let email = email.as_ref();
    require_email(email)?;
    native::send_password_reset_email(email.trim())
}

pub fn sign_out() -> Result<(), FirebaseError> {
    require_initialized()?;
    native::sign_out()
}

/// Current user snapshot, or `None` when signed out.
pub fn current_user() -> Result<Option<User>, FirebaseError> {
    require_initialized()?;
    native::current_user()
}

/// ID token for the current user.
///
/// Returns `Ok(None)` when signed out. Pass `force_refresh = true` to bypass
/// the native cache.
pub fn id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    require_initialized()?;
    native::id_token(force_refresh)
}

/// Point Auth at the local Emulator Suite.
///
/// Call after [`crate::initialize`]. JusFine may keep using staging cloud in
/// debug builds and adopt the emulator later.
pub fn use_emulator(host: impl AsRef<str>, port: u16) -> Result<(), FirebaseError> {
    require_initialized()?;
    let host = host.as_ref().trim();
    if host.is_empty() {
        return Err(FirebaseError::InvalidConfig(
            "emulator host must not be empty".into(),
        ));
    }
    native::use_emulator(host, port)
}
