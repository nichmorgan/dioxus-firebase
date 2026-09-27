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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct ClearCallbackGuard {
        _guard: MutexGuard<'static, ()>,
    }

    impl Drop for ClearCallbackGuard {
        fn drop(&mut self) {
            if let Ok(mut slot) = callback_slot().lock() {
                *slot = None;
            }
        }
    }

    #[fixture]
    fn clear_callback() -> ClearCallbackGuard {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut slot) = callback_slot().lock() {
            *slot = None;
        }
        ClearCallbackGuard { _guard: guard }
    }

    #[rstest]
    fn test_subscribe_uninitialized_fails(_clear_callback: ClearCallbackGuard) {
        let err = subscribe_auth_state(|_| {}).expect_err("must require init");
        assert_eq!(err, FirebaseError::NotInitialized);
    }

    #[rstest]
    fn test_dispatch_auth_state_invokes_callback(_clear_callback: ClearCallbackGuard) {
        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();

        {
            let mut guard = callback_slot().lock().unwrap();
            *guard = Some(Arc::new(move |user| {
                if let Some(user) = user {
                    if user.uid == "test_uid" {
                        invoked_clone.store(true, Ordering::SeqCst);
                    }
                }
            }));
        }

        dispatch_auth_state(Some(User {
            uid: "test_uid".into(),
            email: Some("a@b.c".into()),
            display_name: None,
        }));

        assert!(invoked.load(Ordering::SeqCst));
    }

    #[rstest]
    #[case(None, None)]
    #[case(Some(""), None)]
    #[case(Some("\0"), None)]
    #[case(Some("uid123\x1fuser@example.com\x1fJohn Doe"), Some(User {
        uid: "uid123".into(),
        email: Some("user@example.com".into()),
        display_name: Some("John Doe".into()),
    }))]
    fn test_dispatch_auth_state_payload(
        _clear_callback: ClearCallbackGuard,
        #[case] payload: Option<&str>,
        #[case] expected_user: Option<User>,
    ) {
        let received_user = Arc::new(Mutex::new(None));
        let received_user_clone = received_user.clone();

        {
            let mut guard = callback_slot().lock().unwrap();
            *guard = Some(Arc::new(move |user| {
                *received_user_clone.lock().unwrap() = user;
            }));
        }

        dispatch_auth_state_payload(payload);

        let user = received_user.lock().unwrap().clone();
        assert_eq!(user, expected_user);
    }

    #[rstest]
    fn test_subscription_drop_clears_callback(_clear_callback: ClearCallbackGuard) {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let sub = AuthStateSubscription;
        {
            let mut guard = callback_slot().lock().unwrap();
            *guard = Some(Arc::new(move |_| {
                called_clone.store(true, Ordering::SeqCst);
            }));
        }

        // Dropping subscription clears slot
        drop(sub);

        assert!(callback_slot().lock().unwrap().is_none());

        dispatch_auth_state(None);
        assert!(!called.load(Ordering::SeqCst));
    }
}
