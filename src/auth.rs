//! Auth primitives (email/password + Google) and token helpers.
//! Apple Sign-In helpers remain below but are not re-exported (blocked for now).

use crate::error::FirebaseError;
use crate::host_protocol::{require_email, require_email_password};
use crate::init::require_initialized;
use crate::native;
use crate::user::User;

/// Blocks until the native Firebase Task completes — call from
/// `spawn_blocking` / a worker thread, not the UI thread or first paint.
pub fn sign_in_with_email_and_password(
    email: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<User, FirebaseError> {
    require_initialized()?;
    let email = email.as_ref();
    let password = password.as_ref();
    require_email_password(email, password)?;
    native::sign_in_with_email_and_password(email.trim(), password)
}

pub fn create_user_with_email_and_password(
    email: impl AsRef<str>,
    password: impl AsRef<str>,
) -> Result<User, FirebaseError> {
    require_initialized()?;
    let email = email.as_ref();
    let password = password.as_ref();
    require_email_password(email, password)?;
    native::create_user_with_email_and_password(email.trim(), password)
}

pub fn update_display_name(display_name: impl AsRef<str>) -> Result<(), FirebaseError> {
    require_initialized()?;
    native::update_display_name(display_name.as_ref())?;
    republish_current_user()
}

/// Update the current user's profile photo URL (empty clears when the host supports it).
///
/// Auth-state listeners do not fire for profile updates — subscribers are
/// republished after a successful reload.
pub fn update_photo_url(photo_url: impl AsRef<str>) -> Result<(), FirebaseError> {
    require_initialized()?;
    native::update_photo_url(photo_url.as_ref())?;
    republish_current_user()
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

/// ID token for the current user (`User.getIdToken`).
///
/// Returns `Ok(None)` when signed out. Pass `force_refresh = true` to bypass
/// the native cache.
pub fn get_id_token(force_refresh: bool) -> Result<Option<String>, FirebaseError> {
    require_initialized()?;
    native::get_id_token(force_refresh)
}

/// Point Auth at the local Emulator Suite (`useAuthEmulator`).
///
/// Call after [`crate::initialize_app`]. JusFine may keep using staging cloud in
/// debug builds and adopt the emulator later.
pub fn use_auth_emulator(host: impl AsRef<str>, port: u16) -> Result<(), FirebaseError> {
    require_initialized()?;
    let host = host.as_ref().trim();
    if host.is_empty() {
        return Err(FirebaseError::InvalidConfig(
            "emulator host must not be empty".into(),
        ));
    }
    native::use_auth_emulator(host, port)
}

/// Sign in (or create a session) with Google.
///
/// Convenience over credential APIs — official Auth has no `signInWithGoogle`.
/// Requires [`crate::FirebaseOptions::google_web_client_id`] (or iOS
/// `GIDClientID` in Info.plist). Presents the system Google account sheet.
/// Call from `spawn_blocking` / a worker thread.
pub fn sign_in_with_google() -> Result<User, FirebaseError> {
    require_initialized()?;
    native::sign_in_with_google()
}

/// Link Google to the **current** user (`linkWithCredential`).
///
/// Fails with a wire auth code when signed out or when the credential is
/// already tied to another account. UID is unchanged on success.
pub fn link_with_google() -> Result<User, FirebaseError> {
    require_initialized()?;
    native::link_with_google()
}

/// Unlink a provider from the **current** user (`User.unlink`).
///
/// Pass a Firebase provider id such as `"google.com"`, `"apple.com"`, or
/// `"password"`. There is no dedicated “switch store account” API — to attach a
/// different Google identity, call this then [`link_with_google`] while still
/// signed in. Signing in with that provider after unlink (without linking)
/// creates a **new** Firebase user.
pub fn unlink(provider_id: impl AsRef<str>) -> Result<User, FirebaseError> {
    require_initialized()?;
    let provider_id = provider_id.as_ref().trim();
    if provider_id.is_empty() {
        return Err(FirebaseError::InvalidConfig(
            "provider_id must not be empty".into(),
        ));
    }
    let user = native::unlink(provider_id)?;
    crate::subscribe::dispatch_auth_state(Some(user.clone()));
    Ok(user)
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
// Keep bodies; not re-exported from `lib` / `prelude`.
#[allow(dead_code)]
/// Sign in (or create a session) with Apple.
///
/// Presents Sign in with Apple. Call from `spawn_blocking` / a worker thread.
pub fn sign_in_with_apple() -> Result<User, FirebaseError> {
    require_initialized()?;
    native::sign_in_with_apple()
}

// Apple Sign-In blocked for now — re-enable when Console + OAuth are ready.
#[allow(dead_code)]
/// Link Apple to the **current** user (`linkWithCredential`).
///
/// Fails with a wire auth code when signed out or when the credential is
/// already tied to another account. UID is unchanged on success.
pub fn link_with_apple() -> Result<User, FirebaseError> {
    require_initialized()?;
    native::link_with_apple()
}

fn republish_current_user() -> Result<(), FirebaseError> {
    // Auth state listeners do not fire for profile updates. Republish the
    // reloaded user so subscribers see display_name / photo_url.
    if let Some(user) = native::current_user()? {
        crate::subscribe::dispatch_auth_state(Some(user));
    }
    Ok(())
}
