//! Auth-state subscription for Dioxus signals.

use std::sync::{Arc, Mutex, OnceLock};

use crate::error::FirebaseError;
use crate::host_protocol::parse_user_fields;
use crate::init::require_initialized;
use crate::native;
use crate::user::User;

type AuthCallback = Arc<dyn Fn(Option<User>) + Send + Sync + 'static>;

static CALLBACK: OnceLock<Mutex<Option<AuthCallback>>> = OnceLock::new();

fn callback_slot() -> &'static Mutex<Option<AuthCallback>> {
    CALLBACK.get_or_init(|| Mutex::new(None))
}

/// Guard returned by [`subscribe_auth_state`]. Dropping clears the callback
/// (native listener may still fire until process exit — v1 keeps one global).
#[derive(Debug)]
pub struct AuthStateSubscription;

impl Drop for AuthStateSubscription {
    fn drop(&mut self) {
        if let Ok(mut guard) = callback_slot().lock() {
            *guard = None;
        }
    }
}

/// Subscribe to Firebase Auth state (restore / sign-in / sign-out).
///
/// Only one subscriber is supported in v1. A second call replaces the previous
/// callback. Keep the returned [`AuthStateSubscription`] alive for the app
/// lifetime (store it in a `use_hook` / context).
///
/// The callback may run on a native background or main thread. For Dioxus
/// Signals, schedule the write onto the UI runtime (channel + `spawn`, or
/// your app's equivalent) rather than mutating UI state directly from the
/// callback if your runtime requires it:
///
/// ```rust,ignore
/// use dioxus::prelude::*;
/// use dioxus_firebase::{subscribe_auth_state, User};
///
/// enum AuthStatus { Restoring, Guest, Ready(User) }
///
/// let mut status = use_signal(|| AuthStatus::Restoring);
/// let (tx, mut rx) = futures_channel::mpsc::unbounded();
/// let _sub = subscribe_auth_state(move |user| {
///     let _ = tx.unbounded_send(user);
/// })?;
/// spawn(async move {
///     while let Some(user) = rx.next().await {
///         status.set(match user {
///             Some(u) => AuthStatus::Ready(u),
///             None => AuthStatus::Guest,
///         });
///     }
/// });
/// ```
///
/// The native SDK also restores the session — call this after
/// [`crate::initialize`]; do not poll `current_user` for restore.
pub fn subscribe_auth_state<F>(on_change: F) -> Result<AuthStateSubscription, FirebaseError>
where
    F: Fn(Option<User>) + Send + Sync + 'static,
{
    require_initialized()?;
    {
        let mut guard = callback_slot().lock().map_err(|_| FirebaseError::Native {
            message: "auth-state callback lock poisoned".into(),
        })?;
        *guard = Some(Arc::new(on_change));
    }
    native::subscribe_auth_state()?;
    Ok(AuthStateSubscription)
}

/// Invoked from platform bridges when Auth state changes.
#[allow(dead_code)] // called from android/ios native modules
pub(crate) fn dispatch_auth_state(user: Option<User>) {
    let callback = callback_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().cloned());
    if let Some(callback) = callback {
        callback(user);
    }
}

/// Parse a host user payload (`uid\x1femail\x1fdisplayName` or empty = signed out).
#[allow(dead_code)] // called from android/ios native modules
pub(crate) fn dispatch_auth_state_payload(payload: Option<&str>) {
    let user = payload.and_then(|raw| {
        let raw = raw.trim_end_matches('\0');
        if raw.is_empty() {
            return None;
        }
        parse_user_fields(raw).ok()
    });
    dispatch_auth_state(user);
}
